// Medical vocabulary processor for Canadian family medicine
// File: src-tauri/src/medical_vocab.rs

use log::{debug, info};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Pre-compiled regex patterns for medical numbers (compiled once, used many times)
static BP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(blood pressure|BP|B P)\s+(one hundred \w+|one \w+|\w+)\s+over\s+(\w+\s?\w*)\b",
    )
    .unwrap()
});

static HR_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(heart rate|HR|H R)\s+(\w+\s?\w*)\b").unwrap());

static RR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(respiratory rate|RR|R R|respiration rate)\s+(\w+\s?\w*)\b").unwrap()
});

static O2_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(oxygen saturation|O2 sat|O2sat|oxygen sat)\s+(\w+\s?\w*)\s*percent\b")
        .unwrap()
});

static TEMP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(temperature|temp)\s+(thirty|forty)\s*(one|two|three|four|five|six|seven|eight|nine)?\s*point\s*(\w+)\b").unwrap()
});

static MED_UNITS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|twenty five|thirty|forty|fifty|seventy five|one hundred|two hundred|five hundred|one thousand)\s+(kilograms?|milligrams?|micrograms?|grams?|milliliters?|millilitres?|liters?|litres?|units?|percent|kgs?|mgs?|mcgs?|gms?|mls?)\b").unwrap()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalVocabulary {
    terms: HashMap<String, String>,
    canadian_spellings: HashMap<String, String>,
    common_corrections: HashMap<String, Vec<String>>,
    medication_corrections: HashMap<String, String>,
    custom_vocab_path: Option<PathBuf>,
    #[serde(skip)]
    regex_cache: HashMap<String, Regex>,
}

impl MedicalVocabulary {
    pub fn new() -> Self {
        let mut vocab = MedicalVocabulary {
            terms: HashMap::new(),
            canadian_spellings: HashMap::new(),
            common_corrections: HashMap::new(),
            medication_corrections: HashMap::new(),
            custom_vocab_path: None,
            regex_cache: HashMap::new(),
        };
        vocab.initialize();
        vocab
    }

    #[allow(dead_code)]
    pub fn with_custom_vocab(custom_vocab_path: PathBuf) -> Self {
        let mut vocab = Self::new();
        vocab.custom_vocab_path = Some(custom_vocab_path.clone());
        vocab.load_custom_vocabulary_txt(&custom_vocab_path);
        vocab
    }

    fn get_default_custom_vocab_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").ok()?;
            Some(PathBuf::from(format!(
                "{}/Library/Application Support/com.pais.handy/custom_medical_vocab.txt",
                home
            )))
        }
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").ok()?;
            Some(PathBuf::from(format!(
                "{}\\com.pais.handy\\custom_medical_vocab.txt",
                appdata
            )))
        }
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").ok()?;
            Some(PathBuf::from(format!(
                "{}/.config/com.pais.handy/custom_medical_vocab.txt",
                home
            )))
        }
    }

    pub fn ensure_custom_vocab_file_exists() -> Result<PathBuf, String> {
        let path = Self::get_default_custom_vocab_path()
            .ok_or("Could not determine custom vocabulary path")?;

        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }

            let default_content = r#"# Custom Medical Vocabulary
# Lines starting with # are comments and will be ignored
# Edit this file to add your own medical terms and corrections

# MEDICAL TERMS (one per line)
# Add medical terms that should be recognized
bronchoscopy
colonoscopy
myocarditis

# CORRECTIONS (format: wrong phrase -> correct phrase)
# Use -> to separate the incorrect phrase from the correct one
sugar diabetes -> diabetes mellitus
heart attack -> myocardial infarction
high cholesterol -> hypercholesterolemia

# CANADIAN SPELLINGS (format: US spelling -> Canadian spelling)
# Convert American spellings to Canadian
hemophilia -> haemophilia
pediatrician -> paediatrician
esophagus -> oesophagus
"#;

            fs::write(&path, default_content)
                .map_err(|e| format!("Failed to write custom vocabulary file: {}", e))?;

            info!("Created default custom vocabulary file at: {:?}", path);
        }

        Ok(path)
    }

    fn load_custom_vocabulary_txt(&mut self, path: &PathBuf) {
        match fs::read_to_string(path) {
            Ok(contents) => {
                info!("Loading custom vocabulary from: {:?}", path);

                for line in contents.lines() {
                    let line = line.trim();

                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    if line.contains("->") {
                        let parts: Vec<&str> = line.split("->").collect();
                        if parts.len() == 2 {
                            let wrong = parts[0].trim().to_string();
                            let correct = parts[1].trim().to_string();

                            let wrong_words = wrong.split_whitespace().count();

                            if wrong_words == 1 {
                                self.canadian_spellings.insert(wrong, correct);
                            } else {
                                self.common_corrections
                                    .entry(correct.clone())
                                    .or_insert_with(Vec::new)
                                    .push(wrong);
                            }
                        }
                    } else {
                        self.terms.insert(line.to_lowercase(), line.to_string());
                    }
                }

                debug!("Custom vocabulary loaded successfully");
            }
            Err(e) => {
                debug!("Could not read custom vocabulary file: {}", e);
            }
        }
    }

    fn initialize(&mut self) {
        let canadian_spellings = vec![
            ("anemia", "anaemia"),
            ("center", "centre"),
            ("centers", "centres"),
            ("color", "colour"),
            ("colored", "coloured"),
            ("defense", "defence"),
            ("diarrhea", "diarrhoea"),
            ("edema", "oedema"),
            ("estrogen", "oestrogen"),
            ("favor", "favour"),
            ("favored", "favoured"),
            ("fetus", "foetus"),
            ("fiber", "fibre"),
            ("hemoglobin", "haemoglobin"),
            ("hemorrhage", "haemorrhage"),
            ("leukemia", "leukaemia"),
            ("liter", "litre"),
            ("liters", "litres"),
            ("meter", "metre"),
            ("meters", "metres"),
            ("neighbor", "neighbour"),
            ("pediatric", "paediatric"),
            ("tumor", "tumour"),
            ("tumors", "tumours"),
        ];

        for (us, ca) in canadian_spellings {
            self.canadian_spellings
                .insert(us.to_string(), ca.to_string());
        }

        self.common_corrections.insert(
            "hypertension".to_string(),
            vec![
                "high per tension".to_string(),
                "hyper tension".to_string(),
                "hi pertension".to_string(),
            ],
        );

        self.common_corrections.insert(
            "atrial fibrillation".to_string(),
            vec![
                "a fib".to_string(),
                "afib".to_string(),
                "A-fib".to_string(),
                "atrial fib".to_string(),
            ],
        );

        self.common_corrections.insert(
            "COPD".to_string(),
            vec![
                "see oh pee dee".to_string(),
                "C O P D".to_string(),
                "copd".to_string(),
            ],
        );

        self.common_corrections.insert(
            "GERD".to_string(),
            vec![
                "gerd".to_string(),
                "gurd".to_string(),
                "G E R D".to_string(),
            ],
        );

        self.common_corrections.insert(
            "type 2 diabetes".to_string(),
            vec![
                "type two diabetes".to_string(),
                "type II diabetes".to_string(),
                "type-2 diabetes".to_string(),
                "type 2 diabeetus".to_string(),
            ],
        );

        let medications = vec![
            ("metformin", vec!["met formin", "metform in"]),
            (
                "lisinopril",
                vec!["lysinopril", "liz in o pril", "lisenopril"],
            ),
            ("atorvastatin", vec!["a tor va statin", "ator vastatin"]),
            ("amlodipine", vec!["am low di peen", "amlodipeen"]),
            ("levothyroxine", vec!["levo thyroxine", "levo thyro xine"]),
            ("omeprazole", vec!["oh mep ra zole", "omeprazol"]),
            ("pantoprazole", vec!["panto prazole", "panto prazol"]),
            ("ramipril", vec!["ram i pril", "ramapril"]),
            ("rosuvastatin", vec!["ro su va statin", "rosuvastaten"]),
            ("salbutamol", vec!["sal bute a mol", "albuterol"]),
        ];

        for (correct, variants) in medications {
            for variant in variants {
                self.medication_corrections
                    .insert(variant.to_string(), correct.to_string());
            }
        }

        self.add_terms(&[
            "hypertension",
            "diabetes",
            "asthma",
            "COPD",
            "arthritis",
            "atrial fibrillation",
            "pneumonia",
            "bronchitis",
            "GERD",
            "hypothyroidism",
            "hyperthyroidism",
            "osteoporosis",
            "osteoarthritis",
            "hyperlipidemia",
            "hypercholesterolemia",
            "dyslipidemia",
            "angina",
            "myocardial infarction",
            "stroke",
            "TIA",
            "depression",
            "anxiety",
            "insomnia",
            "metformin",
            "lisinopril",
            "atorvastatin",
            "amlodipine",
            "levothyroxine",
            "omeprazole",
            "acetaminophen",
            "ibuprofen",
            "salbutamol",
            "ramipril",
            "rosuvastatin",
            "pantoprazole",
            "citalopram",
            "escitalopram",
            "sertraline",
            "venlafaxine",
            "gabapentin",
            "pregabalin",
            "tramadol",
            "codeine",
            "Tylenol",
            "Advil",
            "Lipitor",
            "Synthroid",
            "Ventolin",
            "OHIP",
            "MSP",
            "RAMQ",
            "health card",
            "family physician",
            "ECG",
            "electrocardiogram",
            "X-ray",
            "ultrasound",
            "CT scan",
            "MRI",
            "blood pressure",
            "heart rate",
            "respiratory rate",
            "blood work",
            "urinalysis",
            "CBC",
            "complete blood count",
            "lipid panel",
            "A1C",
            "hemoglobin A1C",
            "TSH",
            "INR",
            "chest pain",
            "shortness of breath",
            "dyspnea",
            "wheezing",
            "cough",
            "fever",
            "nausea",
            "vomiting",
            "diarrhea",
            "headache",
            "dizziness",
            "fatigue",
            "malaise",
            "milligrams",
            "micrograms",
            "milliliters",
            "units",
            "once daily",
            "twice daily",
            "three times daily",
            "as needed",
            "prn",
            "with food",
            "on empty stomach",
        ]);

        if let Some(path) = self.custom_vocab_path.clone() {
            self.load_custom_vocabulary_txt(&path);
        } else if let Some(default_path) = Self::get_default_custom_vocab_path() {
            if default_path.exists() {
                self.load_custom_vocabulary_txt(&default_path);
            }
        }
    }

    fn add_terms(&mut self, terms: &[&str]) {
        for term in terms {
            self.terms.insert(term.to_lowercase(), term.to_string());
        }
    }

    pub fn process_text(&mut self, text: &str) -> String {
        let mut processed = text.to_string();

        for (wrong, correct) in &self.medication_corrections.clone() {
            processed = self.replace_case_insensitive(&processed, wrong, correct);
        }

        for (correct, variants) in &self.common_corrections.clone() {
            for variant in variants {
                processed = self.replace_case_insensitive(&processed, variant, correct);
            }
        }

        for (us_spelling, ca_spelling) in &self.canadian_spellings.clone() {
            processed = self.replace_word_boundary(&processed, us_spelling, ca_spelling);
        }

        processed = self.format_medical_numbers(&processed);

        processed
    }

    fn replace_case_insensitive(&mut self, text: &str, pattern: &str, replacement: &str) -> String {
        let cache_key = format!("ci:{}", pattern);

        if !self.regex_cache.contains_key(&cache_key) {
            let regex_pattern = format!(r"(?i)\b{}\b", regex::escape(pattern));
            if let Ok(re) = Regex::new(&regex_pattern) {
                self.regex_cache.insert(cache_key.clone(), re);
            } else {
                return text.to_string();
            }
        }

        if let Some(re) = self.regex_cache.get(&cache_key) {
            re.replace_all(text, replacement).to_string()
        } else {
            text.to_string()
        }
    }

    fn replace_word_boundary(&mut self, text: &str, pattern: &str, replacement: &str) -> String {
        let cache_key = format!("wb:{}", pattern);

        if !self.regex_cache.contains_key(&cache_key) {
            let regex_pattern = format!(r"\b{}\b", regex::escape(pattern));
            if let Ok(re) = Regex::new(&regex_pattern) {
                self.regex_cache.insert(cache_key.clone(), re);
            } else {
                return text.to_string();
            }
        }

        if let Some(re) = self.regex_cache.get(&cache_key) {
            re.replace_all(text, replacement).to_string()
        } else {
            text.to_string()
        }
    }

    fn format_medical_numbers(&self, text: &str) -> String {
        let mut processed = text.to_string();

        // Number mappings
        let number_map: HashMap<&str, &str> = [
            ("zero", "0"),
            ("one", "1"),
            ("two", "2"),
            ("three", "3"),
            ("four", "4"),
            ("five", "5"),
            ("six", "6"),
            ("seven", "7"),
            ("eight", "8"),
            ("nine", "9"),
            ("ten", "10"),
            ("eleven", "11"),
            ("twelve", "12"),
            ("thirteen", "13"),
            ("fourteen", "14"),
            ("fifteen", "15"),
            ("sixteen", "16"),
            ("seventeen", "17"),
            ("eighteen", "18"),
            ("nineteen", "19"),
            ("twenty", "20"),
            ("twenty five", "25"),
            ("thirty", "30"),
            ("thirty five", "35"),
            ("forty", "40"),
            ("fifty", "50"),
            ("sixty", "60"),
            ("seventy", "70"),
            ("seventy five", "75"),
            ("eighty", "80"),
            ("ninety", "90"),
            ("ninety five", "95"),
            ("ninety eight", "98"),
            ("ninety nine", "99"),
            ("one hundred", "100"),
            ("one hundred twenty", "120"),
            ("one hundred thirty", "130"),
            ("one hundred forty", "140"),
            ("one hundred fifty", "150"),
            ("two hundred", "200"),
            ("five hundred", "500"),
            ("one thousand", "1000"),
        ]
        .iter()
        .copied()
        .collect();

        // VITAL SIGNS FORMATTING - using pre-compiled static regexes

        // Blood Pressure
        processed = BP_PATTERN
            .replace_all(&processed, |caps: &regex::Captures| {
                let _prefix = caps.get(1).unwrap().as_str();
                let systolic_word = caps.get(2).unwrap().as_str().to_lowercase();
                let diastolic_word = caps.get(3).unwrap().as_str().to_lowercase();

                let systolic_binding = systolic_word.as_str();
                let systolic = number_map
                    .get(systolic_word.as_str())
                    .unwrap_or(&systolic_binding);
                let diastolic_binding = diastolic_word.as_str();
                let diastolic = number_map
                    .get(diastolic_word.as_str())
                    .unwrap_or(&diastolic_binding);

                format!("BP {}/{}", systolic, diastolic)
            })
            .to_string();

        // Heart Rate
        processed = HR_PATTERN
            .replace_all(&processed, |caps: &regex::Captures| {
                let rate_word = caps.get(2).unwrap().as_str().to_lowercase();
                let rate_binding = rate_word.as_str();
                let rate = number_map.get(rate_word.as_str()).unwrap_or(&rate_binding);
                format!("HR {}", rate)
            })
            .to_string();

        // Respiratory Rate
        processed = RR_PATTERN
            .replace_all(&processed, |caps: &regex::Captures| {
                let rate_word = caps.get(2).unwrap().as_str().to_lowercase();
                let rate_binding = rate_word.as_str();
                let rate = number_map.get(rate_word.as_str()).unwrap_or(&rate_binding);
                format!("RR {}", rate)
            })
            .to_string();

        // Oxygen Saturation
        processed = O2_PATTERN
            .replace_all(&processed, |caps: &regex::Captures| {
                let sat_word = caps.get(2).unwrap().as_str().to_lowercase();
                let sat_binding = sat_word.as_str();
                let sat = number_map.get(sat_word.as_str()).unwrap_or(&sat_binding);
                format!("O2 sat {}%", sat)
            })
            .to_string();

        // Temperature
        processed = TEMP_PATTERN
            .replace_all(&processed, |caps: &regex::Captures| {
                let tens = caps.get(2).unwrap().as_str().to_lowercase();
                let ones = caps.get(3).map(|m| m.as_str().to_lowercase());
                let decimal = caps.get(4).unwrap().as_str().to_lowercase();

                let tens_num = if tens == "thirty" {
                    "3"
                } else if tens == "forty" {
                    "4"
                } else {
                    ""
                };
                let ones_num = ones
                    .as_ref()
                    .and_then(|o| number_map.get(o.as_str()))
                    .unwrap_or(&"");
                let decimal_binding = decimal.as_str();
                let decimal_num = number_map.get(decimal.as_str()).unwrap_or(&decimal_binding);

                format!("temp {}{}.{}°C", tens_num, ones_num, decimal_num)
            })
            .to_string();

        // LAB VALUE FORMATTING

        // Common lab abbreviations
        let lab_corrections = vec![
            ("A one C", "A1C"),
            ("A 1 C", "A1C"),
            ("hemoglobin A one C", "hemoglobin A1C"),
            ("hemoglobin A 1 C", "hemoglobin A1C"),
            ("T S H", "TSH"),
            ("I N R", "INR"),
            ("C B C", "CBC"),
            ("C R P", "CRP"),
            ("E S R", "ESR"),
            ("A L T", "ALT"),
            ("A S T", "AST"),
            ("G F R", "GFR"),
            ("B U N", "BUN"),
            ("H D L", "HDL"),
            ("L D L", "LDL"),
            ("P S A", "PSA"),
            ("complete blood count", "CBC"),
            ("C reactive protein", "CRP"),
        ];

        for (spoken, abbrev) in lab_corrections {
            let pattern = format!(r"(?i)\b{}\b", regex::escape(spoken));
            if let Ok(re) = Regex::new(&pattern) {
                processed = re.replace_all(&processed, abbrev).to_string();
            }
        }

        // MEDICATION UNITS - using pre-compiled static regex
        processed = MED_UNITS_PATTERN
            .replace_all(&processed, |caps: &regex::Captures| {
                let num_word = caps.get(1).unwrap().as_str().to_lowercase();
                let unit = caps.get(2).unwrap().as_str().to_lowercase();

                let digit = number_map.get(num_word.as_str()).unwrap_or(&"");

                let abbrev = match unit.as_str() {
                    "kilogram" | "kilograms" | "kgs" | "kg" => "kg",
                    "milligram" | "milligrams" | "mgs" | "mg" => "mg",
                    "microgram" | "micrograms" | "mcgs" | "mcg" => "mcg",
                    "gram" | "grams" | "gms" | "g" => "g",
                    "milliliter" | "milliliters" | "millilitre" | "millilitres" | "mls" | "ml" => {
                        "mL"
                    }
                    "liter" | "liters" | "litre" | "litres" => "L",
                    "unit" | "units" => "units",
                    "percent" => "%",
                    _ => &unit,
                };

                format!("{} {}", digit, abbrev)
            })
            .to_string();

        processed
    }

    #[allow(dead_code)]
    pub fn reload_custom_vocabulary(&mut self) {
        if let Some(path) = self.custom_vocab_path.clone() {
            info!("Reloading custom vocabulary from: {:?}", path);
            self.load_custom_vocabulary_txt(&path);
        } else if let Some(default_path) = Self::get_default_custom_vocab_path() {
            if default_path.exists() {
                info!("Reloading custom vocabulary from: {:?}", default_path);
                self.load_custom_vocabulary_txt(&default_path);
            }
        }
    }
}

impl Default for MedicalVocabulary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canadian_spelling() {
        let vocab = MedicalVocabulary::new();
        let result = vocab.process_text("The patient has anemia and edema.");
        assert!(result.contains("anaemia"));
        assert!(result.contains("oedema"));
    }

    #[test]
    fn test_medical_corrections() {
        let vocab = MedicalVocabulary::new();
        let result = vocab.process_text("Patient has high per tension and a fib.");
        assert!(result.contains("hypertension"));
        assert!(result.contains("atrial fibrillation"));
    }

    #[test]
    fn test_medication_corrections() {
        let vocab = MedicalVocabulary::new();
        let result = vocab.process_text("Prescribed met formin and lysinopril.");
        assert!(result.contains("metformin"));
        assert!(result.contains("lisinopril"));
    }

    #[test]
    fn test_number_formatting() {
        let vocab = MedicalVocabulary::new();
        let result = vocab.process_text("Give twenty five milligrams and fifty kilograms.");
        assert!(result.contains("25 mg"));
        assert!(result.contains("50 kg"));
    }
}
