# Medical Vocabulary Expansion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand medical vocabulary from ~560 to 15,000+ terms to rival Dragon Dictate accuracy without compromising speed.

**Architecture:** Binary pre-compiled index with BK-tree fuzzy matching + Metaphone phonetic scoring. Integrates with existing `text.rs` infrastructure. User custom vocab loads on top with priority.

**Tech Stack:** Rust, `bk-tree` crate (already used), `bincode` for serialization, `rphonetic` for Metaphone, `memmap2` for memory-mapped loading.

---

## Task 1: Add Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add required crates**

Add to `[dependencies]` section in `src-tauri/Cargo.toml`:

```toml
bincode = "1.3"
memmap2 = "0.9"
rphonetic = "3.0"
```

**Step 2: Verify dependencies resolve**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully, dependencies downloaded

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: add bincode, memmap2, rphonetic dependencies for medical vocab expansion"
```

---

## Task 2: Create MedicalIndex Module

**Files:**
- Create: `src-tauri/src/medical_index.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

**Step 1: Write the MedicalIndex struct and serialization**

Create `src-tauri/src/medical_index.rs`:

```rust
//! Binary medical vocabulary index with BK-tree fuzzy matching and phonetic scoring.
//!
//! This module provides a pre-compiled vocabulary index that loads in ~5ms via memory-mapping
//! and supports O(log n) fuzzy lookups with combined edit-distance and phonetic scoring.

use bk_tree::{metrics::Levenshtein, BKTree};
use rphonetic::{Encoder, Metaphone};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Pre-compiled medical vocabulary index for fast fuzzy matching.
#[derive(Serialize, Deserialize)]
pub struct MedicalIndex {
    /// Exact match lookup (lowercase term -> canonical form)
    pub exact: HashMap<String, String>,

    /// Correction mappings (misheard/variant -> correct term)
    pub corrections: HashMap<String, String>,

    /// Metaphone phonetic hashes for each term
    pub phonetic_hashes: HashMap<String, String>,

    /// All terms for BK-tree reconstruction
    terms: Vec<String>,

    /// Category tags for terms (optional filtering)
    pub categories: HashMap<String, TermCategory>,
}

/// Categories for medical terms
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermCategory {
    MedicationGeneric,
    MedicationBrand,
    Condition,
    Anatomy,
    Procedure,
    LabTest,
    Eponym,
    Abbreviation,
    PhoneticCorrection,
}

/// Runtime index with reconstructed BK-tree
pub struct MedicalIndexRuntime {
    pub index: MedicalIndex,
    pub bk_tree: BKTree<String, Levenshtein>,
    metaphone: Metaphone,
}

impl MedicalIndex {
    /// Build a new index from terms and corrections.
    pub fn build(
        terms: Vec<String>,
        corrections: HashMap<String, String>,
        categories: HashMap<String, TermCategory>,
    ) -> Self {
        let metaphone = Metaphone::default();
        let mut exact = HashMap::new();
        let mut phonetic_hashes = HashMap::new();

        for term in &terms {
            let lower = term.to_lowercase();
            exact.insert(lower.clone(), term.clone());

            // Generate phonetic hash
            if let Ok(hash) = metaphone.encode(&lower) {
                phonetic_hashes.insert(lower, hash);
            }
        }

        // Also add corrections to exact lookup
        for (variant, correct) in &corrections {
            exact.insert(variant.to_lowercase(), correct.clone());
        }

        Self {
            exact,
            corrections,
            phonetic_hashes,
            terms,
            categories,
        }
    }

    /// Serialize index to binary file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }

    /// Load index from binary file.
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let index: Self = bincode::deserialize_from(reader)?;
        Ok(index)
    }
}

impl MedicalIndexRuntime {
    /// Create runtime index from serialized index, reconstructing BK-tree.
    pub fn new(index: MedicalIndex) -> Self {
        let mut bk_tree = BKTree::new(Levenshtein);

        // Rebuild BK-tree from terms
        for term in &index.terms {
            bk_tree.insert(term.to_lowercase());
        }

        Self {
            index,
            bk_tree,
            metaphone: Metaphone::default(),
        }
    }

    /// Load from file and create runtime index.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let index = MedicalIndex::load_from_file(path)?;
        Ok(Self::new(index))
    }

    /// Find correction for a word using hybrid matching.
    ///
    /// 1. Exact match (O(1))
    /// 2. BK-tree fuzzy search (O(log n))
    /// 3. Phonetic scoring for candidates
    pub fn find_correction(&self, word: &str) -> Option<String> {
        let lower = word.to_lowercase();

        // 1. Exact match - fastest path
        if let Some(canonical) = self.index.exact.get(&lower) {
            return Some(canonical.clone());
        }

        // 2. BK-tree fuzzy search (max edit distance 2)
        let candidates: Vec<_> = self.bk_tree.find(&lower, 2).collect();

        if candidates.is_empty() {
            return None;
        }

        // 3. Score candidates by combined edit distance + phonetic similarity
        let word_phonetic = self.metaphone.encode(&lower).ok();

        let best = candidates
            .iter()
            .filter_map(|(distance, candidate)| {
                let score = self.compute_score(&lower, candidate, *distance, &word_phonetic);
                if score > 0.7 {
                    Some((candidate, score))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        best.and_then(|(candidate, _)| self.index.exact.get(*candidate).cloned())
    }

    /// Compute combined score from edit distance and phonetic similarity.
    fn compute_score(
        &self,
        word: &str,
        candidate: &str,
        edit_distance: u32,
        word_phonetic: &Option<String>,
    ) -> f64 {
        let max_len = word.len().max(candidate.len()) as f64;
        if max_len == 0.0 {
            return 0.0;
        }

        // Edit distance score (1.0 = perfect match, 0.0 = very different)
        let edit_score = 1.0 - (edit_distance as f64 / max_len);

        // Phonetic similarity score
        let phonetic_score = match (word_phonetic, self.index.phonetic_hashes.get(candidate)) {
            (Some(w_phone), Some(c_phone)) if w_phone == c_phone => 1.0,
            (Some(w_phone), Some(c_phone)) => {
                // Partial phonetic match - compare first N characters
                let min_len = w_phone.len().min(c_phone.len());
                if min_len > 0 {
                    let matching = w_phone
                        .chars()
                        .zip(c_phone.chars())
                        .take_while(|(a, b)| a == b)
                        .count();
                    matching as f64 / min_len as f64
                } else {
                    0.0
                }
            }
            _ => 0.5, // No phonetic data, neutral score
        };

        // Combined score: weight phonetic matching higher for medical terms
        0.3 * edit_score + 0.7 * phonetic_score
    }

    /// Process text, replacing words with corrections where found.
    pub fn process_text(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut last_end = 0;

        // Simple word tokenization
        for (start, part) in text.match_indices(|c: char| c.is_alphanumeric() || c == '\'') {
            // Find the full word
            let word_end = text[start..]
                .find(|c: char| !c.is_alphanumeric() && c != '\'')
                .map(|i| start + i)
                .unwrap_or(text.len());

            if start > last_end {
                result.push_str(&text[last_end..start]);
            }

            let word = &text[start..word_end];
            if word.len() >= 3 {
                // Only process words 3+ chars
                if let Some(correction) = self.find_correction(word) {
                    // Preserve original case pattern
                    result.push_str(&preserve_case(&correction, word));
                } else {
                    result.push_str(word);
                }
            } else {
                result.push_str(word);
            }

            last_end = word_end;
        }

        // Append any remaining text
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        result
    }
}

/// Preserve the case pattern of the original word in the replacement.
fn preserve_case(replacement: &str, original: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
    } else {
        replacement.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let terms = vec!["metformin".to_string(), "lisinopril".to_string()];
        let corrections = HashMap::new();
        let categories = HashMap::new();
        let index = MedicalIndex::build(terms, corrections, categories);
        let runtime = MedicalIndexRuntime::new(index);

        assert_eq!(
            runtime.find_correction("metformin"),
            Some("metformin".to_string())
        );
        assert_eq!(
            runtime.find_correction("METFORMIN"),
            Some("metformin".to_string())
        );
    }

    #[test]
    fn test_fuzzy_match() {
        let terms = vec!["metformin".to_string(), "lisinopril".to_string()];
        let corrections = HashMap::new();
        let categories = HashMap::new();
        let index = MedicalIndex::build(terms, corrections, categories);
        let runtime = MedicalIndexRuntime::new(index);

        // "metformin" with typo should still match
        let result = runtime.find_correction("metfromin");
        assert!(result.is_some());
    }

    #[test]
    fn test_correction_mapping() {
        let terms = vec!["lisinopril".to_string()];
        let mut corrections = HashMap::new();
        corrections.insert("lysinopril".to_string(), "lisinopril".to_string());
        let categories = HashMap::new();
        let index = MedicalIndex::build(terms, corrections, categories);
        let runtime = MedicalIndexRuntime::new(index);

        assert_eq!(
            runtime.find_correction("lysinopril"),
            Some("lisinopril".to_string())
        );
    }

    #[test]
    fn test_preserve_case() {
        assert_eq!(preserve_case("metformin", "METFORMIN"), "METFORMIN");
        assert_eq!(preserve_case("metformin", "Metformin"), "Metformin");
        assert_eq!(preserve_case("metformin", "metformin"), "metformin");
    }
}
```

**Step 2: Add module to lib.rs**

In `src-tauri/src/lib.rs`, add the module declaration near the top with other modules:

```rust
pub mod medical_index;
```

**Step 3: Run tests**

Run: `cd src-tauri && cargo test medical_index`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/medical_index.rs src-tauri/src/lib.rs
git commit -m "feat: add MedicalIndex module with BK-tree and phonetic matching"
```

---

## Task 3: Create Vocabulary Source Directory Structure

**Files:**
- Create: `src-tauri/resources/vocab_sources/` directory
- Create: All source text files

**Step 1: Create directory**

Run: `mkdir -p src-tauri/resources/vocab_sources`

**Step 2: Create placeholder files with headers**

Create `src-tauri/resources/vocab_sources/medications_generic.txt`:
```
# Generic Medication Names
# Format: one term per line, or "variant -> correct" for corrections
# Source: RxNorm, FDA Orange Book
#
```

Create `src-tauri/resources/vocab_sources/medications_brand.txt`:
```
# Brand Medication Names
# Format: brand_name or "brand_name -> generic_name"
# Source: RxNorm, Health Canada DPD
#
```

Create `src-tauri/resources/vocab_sources/conditions.txt`:
```
# Medical Conditions and Diagnoses
# Format: one term per line
# Source: SNOMED-CT, ICD-10-CM
#
```

Create `src-tauri/resources/vocab_sources/anatomy.txt`:
```
# Anatomical Terms
# Format: one term per line
# Source: Foundational Model of Anatomy
#
```

Create `src-tauri/resources/vocab_sources/procedures.txt`:
```
# Medical Procedures
# Format: one term per line
# Source: CPT, SNOMED-CT
#
```

Create `src-tauri/resources/vocab_sources/lab_tests.txt`:
```
# Laboratory Tests
# Format: one term per line, include spoken forms
# Source: LOINC
#
```

Create `src-tauri/resources/vocab_sources/eponyms.txt`:
```
# Medical Eponyms (Named Conditions, Signs, Procedures)
# Format: one term per line
#
```

Create `src-tauri/resources/vocab_sources/abbreviations.txt`:
```
# Abbreviation Expansions
# Format: spoken_form -> abbreviation
# Example: A one C -> A1C
#
```

Create `src-tauri/resources/vocab_sources/phonetic_corrections.txt`:
```
# Phonetic Corrections (Common Mishearings)
# Format: misheard -> correct
# Source: Common Whisper errors, medical transcription guides
#
```

**Step 3: Commit**

```bash
git add src-tauri/resources/vocab_sources/
git commit -m "chore: create vocab_sources directory structure"
```

---

## Task 4: Migrate Existing Vocabulary

**Files:**
- Read: `src-tauri/resources/default_custom_vocab.txt`
- Modify: All vocab_sources/*.txt files
- Delete: `src-tauri/resources/default_custom_vocab.txt` (after migration)

**Step 1: Categorize and migrate existing terms**

Read `default_custom_vocab.txt` and categorize each term. Here's the migration mapping:

**medications_generic.txt** - Add these generic drug names:
```
acetaminophen
ibuprofen
naproxen
aspirin
metformin
insulin
sitagliptin
empagliflozin
dapagliflozin
canagliflozin
semaglutide
liraglutide
dulaglutide
glipizide
glyburide
pioglitazone
pantoprazole
omeprazole
esomeprazole
rabeprazole
lansoprazole
famotidine
ranitidine
mesalamine
ondansetron
metoclopramide
prochlorperazine
dimenhydrinate
loperamide
docusate
bisacodyl
lactulose
sennosides
psyllium
amlodipine
atorvastatin
rosuvastatin
pravastatin
simvastatin
lisinopril
ramipril
enalapril
perindopril
losartan
valsartan
irbesartan
candesartan
telmisartan
metoprolol
bisoprolol
atenolol
carvedilol
diltiazem
verapamil
nifedipine
furosemide
hydrochlorothiazide
spironolactone
indapamide
apixaban
rivaroxaban
dabigatran
edoxaban
warfarin
clopidogrel
ticagrelor
salbutamol
albuterol
fluticasone
budesonide
formoterol
salmeterol
tiotropium
ipratropium
montelukast
prednisone
prednisolone
methylprednisolone
dexamethasone
hydrocortisone
amoxicillin
amoxicillin-clavulanate
azithromycin
clarithromycin
erythromycin
ciprofloxacin
levofloxacin
moxifloxacin
doxycycline
minocycline
tetracycline
cephalexin
cefuroxime
ceftriaxone
trimethoprim-sulfamethoxazole
nitrofurantoin
metronidazole
clindamycin
vancomycin
sertraline
escitalopram
citalopram
fluoxetine
paroxetine
venlafaxine
duloxetine
bupropion
mirtazapine
trazodone
amitriptyline
nortriptyline
quetiapine
olanzapine
risperidone
aripiprazole
clonazepam
lorazepam
diazepam
alprazolam
zopiclone
zolpidem
tramadol
morphine
hydromorphone
oxycodone
codeine
fentanyl
pregabalin
gabapentin
carbamazepine
phenytoin
valproic acid
levetiracetam
topiramate
lamotrigine
levothyroxine
methimazole
propylthiouracil
alendronate
risedronate
denosumab
calcium carbonate
vitamin D
cholecalciferol
hydroxychloroquine
methotrexate
sulfasalazine
leflunomide
adalimumab
etanercept
infliximab
tofacitinib
finasteride
tamsulosin
silodosin
dutasteride
oxybutynin
tolterodine
solifenacin
mirabegron
sildenafil
tadalafil
vardenafil
latanoprost
timolol
brimonidine
dorzolamide
travoprost
bimatoprost
cetirizine
loratadine
fexofenadine
diphenhydramine
desloratadine
fluticasone nasal
mometasone
beclomethasone
budesonide nasal
epinephrine
atropine
naloxone
glucagon
nitroglycerin
isosorbide
hydralazine
clonidine
methyldopa
prazosin
doxazosin
terazosin
```

**medications_brand.txt** - Add these brand names with mappings:
```
Tylenol -> acetaminophen
Advil -> ibuprofen
Motrin -> ibuprofen
Aleve -> naproxen
Aspirin -> aspirin
Glucophage -> metformin
Januvia -> sitagliptin
Jardiance -> empagliflozin
Farxiga -> dapagliflozin
Invokana -> canagliflozin
Ozempic -> semaglutide
Wegovy -> semaglutide
Rybelsus -> semaglutide
Victoza -> liraglutide
Trulicity -> dulaglutide
Glucotrol -> glipizide
Diabeta -> glyburide
Actos -> pioglitazone
Pantoloc -> pantoprazole
Protonix -> pantoprazole
Losec -> omeprazole
Prilosec -> omeprazole
Nexium -> esomeprazole
Pariet -> rabeprazole
Prevacid -> lansoprazole
Pepcid -> famotidine
Zantac -> ranitidine
Pentasa -> mesalamine
Asacol -> mesalamine
Zofran -> ondansetron
Maxeran -> metoclopramide
Reglan -> metoclopramide
Gravol -> dimenhydrinate
Dramamine -> dimenhydrinate
Imodium -> loperamide
Colace -> docusate
Dulcolax -> bisacodyl
Senokot -> sennosides
Metamucil -> psyllium
Norvasc -> amlodipine
Lipitor -> atorvastatin
Crestor -> rosuvastatin
Pravachol -> pravastatin
Zocor -> simvastatin
Zestril -> lisinopril
Prinivil -> lisinopril
Altace -> ramipril
Vasotec -> enalapril
Coversyl -> perindopril
Cozaar -> losartan
Diovan -> valsartan
Avapro -> irbesartan
Atacand -> candesartan
Micardis -> telmisartan
Lopressor -> metoprolol
Toprol -> metoprolol
Monocor -> bisoprolol
Tenormin -> atenolol
Coreg -> carvedilol
Cardizem -> diltiazem
Tiazac -> diltiazem
Isoptin -> verapamil
Adalat -> nifedipine
Lasix -> furosemide
Hydrodiuril -> hydrochlorothiazide
Aldactone -> spironolactone
Lozide -> indapamide
Eliquis -> apixaban
Xarelto -> rivaroxaban
Pradaxa -> dabigatran
Lixiana -> edoxaban
Coumadin -> warfarin
Plavix -> clopidogrel
Brilinta -> ticagrelor
Ventolin -> salbutamol
Proventil -> albuterol
Flovent -> fluticasone
Pulmicort -> budesonide
Oxeze -> formoterol
Serevent -> salmeterol
Spiriva -> tiotropium
Atrovent -> ipratropium
Singulair -> montelukast
Deltasone -> prednisone
Medrol -> methylprednisolone
Decadron -> dexamethasone
Cortef -> hydrocortisone
Amoxil -> amoxicillin
Augmentin -> amoxicillin-clavulanate
Clavulin -> amoxicillin-clavulanate
Zithromax -> azithromycin
Biaxin -> clarithromycin
Cipro -> ciprofloxacin
Levaquin -> levofloxacin
Avelox -> moxifloxacin
Vibramycin -> doxycycline
Keflex -> cephalexin
Ceftin -> cefuroxime
Rocephin -> ceftriaxone
Bactrim -> trimethoprim-sulfamethoxazole
Septra -> trimethoprim-sulfamethoxazole
Macrobid -> nitrofurantoin
Flagyl -> metronidazole
Dalacin -> clindamycin
Vancocin -> vancomycin
Zoloft -> sertraline
Lexapro -> escitalopram
Cipralex -> escitalopram
Celexa -> citalopram
Prozac -> fluoxetine
Paxil -> paroxetine
Effexor -> venlafaxine
Cymbalta -> duloxetine
Wellbutrin -> bupropion
Remeron -> mirtazapine
Desyrel -> trazodone
Elavil -> amitriptyline
Seroquel -> quetiapine
Zyprexa -> olanzapine
Risperdal -> risperidone
Abilify -> aripiprazole
Rivotril -> clonazepam
Klonopin -> clonazepam
Ativan -> lorazepam
Valium -> diazepam
Xanax -> alprazolam
Imovane -> zopiclone
Ambien -> zolpidem
Ultram -> tramadol
MS Contin -> morphine
Dilaudid -> hydromorphone
OxyContin -> oxycodone
Percocet -> oxycodone
Duragesic -> fentanyl
Lyrica -> pregabalin
Neurontin -> gabapentin
Tegretol -> carbamazepine
Dilantin -> phenytoin
Depakene -> valproic acid
Epival -> valproic acid
Keppra -> levetiracetam
Topamax -> topiramate
Lamictal -> lamotrigine
Synthroid -> levothyroxine
Eltroxin -> levothyroxine
Tapazole -> methimazole
Fosamax -> alendronate
Actonel -> risedronate
Prolia -> denosumab
Tums -> calcium carbonate
Plaquenil -> hydroxychloroquine
Trexall -> methotrexate
Azulfidine -> sulfasalazine
Arava -> leflunomide
Humira -> adalimumab
Enbrel -> etanercept
Remicade -> infliximab
Xeljanz -> tofacitinib
Proscar -> finasteride
Propecia -> finasteride
Flomax -> tamsulosin
Rapaflo -> silodosin
Avodart -> dutasteride
Ditropan -> oxybutynin
Detrol -> tolterodine
Vesicare -> solifenacin
Myrbetriq -> mirabegron
Viagra -> sildenafil
Cialis -> tadalafil
Levitra -> vardenafil
Xalatan -> latanoprost
Timoptic -> timolol
Alphagan -> brimonidine
Trusopt -> dorzolamide
Travatan -> travoprost
Lumigan -> bimatoprost
Reactine -> cetirizine
Zyrtec -> cetirizine
Claritin -> loratadine
Allegra -> fexofenadine
Benadryl -> diphenhydramine
Aerius -> desloratadine
Flonase -> fluticasone nasal
Nasonex -> mometasone
EpiPen -> epinephrine
Narcan -> naloxone
GlucaGen -> glucagon
Nitrostat -> nitroglycerin
```

**conditions.txt** - Add medical conditions:
```
hypertension
hypotension
diabetes mellitus
type 1 diabetes
type 2 diabetes
gestational diabetes
prediabetes
hypoglycemia
hyperglycemia
diabetic ketoacidosis
hyperosmolar hyperglycemic state
asthma
chronic obstructive pulmonary disease
COPD
emphysema
chronic bronchitis
pneumonia
bronchitis
tuberculosis
pulmonary embolism
pleural effusion
pneumothorax
atelectasis
pulmonary fibrosis
interstitial lung disease
sleep apnea
obstructive sleep apnea
atrial fibrillation
atrial flutter
supraventricular tachycardia
ventricular tachycardia
ventricular fibrillation
bradycardia
tachycardia
heart failure
congestive heart failure
coronary artery disease
myocardial infarction
angina pectoris
unstable angina
acute coronary syndrome
cardiomyopathy
dilated cardiomyopathy
hypertrophic cardiomyopathy
pericarditis
endocarditis
myocarditis
valvular heart disease
mitral regurgitation
mitral stenosis
aortic regurgitation
aortic stenosis
tricuspid regurgitation
pulmonary hypertension
peripheral artery disease
deep vein thrombosis
varicose veins
aortic aneurysm
carotid stenosis
stroke
transient ischemic attack
cerebrovascular accident
ischemic stroke
hemorrhagic stroke
subarachnoid hemorrhage
intracerebral hemorrhage
gastroesophageal reflux disease
GERD
peptic ulcer disease
gastritis
duodenitis
esophagitis
Barrett's esophagus
hiatal hernia
gastroparesis
irritable bowel syndrome
inflammatory bowel disease
Crohn's disease
ulcerative colitis
diverticulitis
diverticulosis
celiac disease
pancreatitis
acute pancreatitis
chronic pancreatitis
cholecystitis
cholelithiasis
choledocholithiasis
cholangitis
hepatitis
cirrhosis
fatty liver disease
nonalcoholic fatty liver disease
hepatic encephalopathy
ascites
esophageal varices
gastrointestinal bleeding
upper GI bleed
lower GI bleed
colorectal cancer
colon polyps
appendicitis
bowel obstruction
ileus
hemorrhoids
anal fissure
chronic kidney disease
acute kidney injury
end stage renal disease
nephrotic syndrome
nephritic syndrome
glomerulonephritis
pyelonephritis
urinary tract infection
cystitis
urolithiasis
kidney stones
renal colic
benign prostatic hyperplasia
prostatitis
prostate cancer
erectile dysfunction
urinary incontinence
overactive bladder
hypothyroidism
hyperthyroidism
Graves' disease
Hashimoto's thyroiditis
thyroid nodule
thyroid cancer
goiter
Addison's disease
Cushing's syndrome
pheochromocytoma
hyperaldosteronism
hypopituitarism
diabetes insipidus
acromegaly
prolactinoma
osteoporosis
osteopenia
osteoarthritis
rheumatoid arthritis
psoriatic arthritis
gout
pseudogout
ankylosing spondylitis
systemic lupus erythematosus
lupus
Sjogren's syndrome
scleroderma
polymyalgia rheumatica
fibromyalgia
bursitis
tendinitis
carpal tunnel syndrome
rotator cuff tear
meniscus tear
anterior cruciate ligament tear
ACL tear
herniated disc
spinal stenosis
sciatica
low back pain
cervical radiculopathy
lumbar radiculopathy
scoliosis
kyphosis
fracture
dislocation
sprain
strain
anemia
iron deficiency anemia
vitamin B12 deficiency
folate deficiency
pernicious anemia
hemolytic anemia
sickle cell disease
thalassemia
polycythemia vera
thrombocytopenia
thrombocytosis
leukopenia
leukocytosis
neutropenia
lymphoma
Hodgkin lymphoma
non-Hodgkin lymphoma
leukemia
acute lymphoblastic leukemia
acute myeloid leukemia
chronic lymphocytic leukemia
chronic myeloid leukemia
multiple myeloma
myelodysplastic syndrome
hemophilia
von Willebrand disease
disseminated intravascular coagulation
deep vein thrombosis
pulmonary embolism
Parkinson's disease
Alzheimer's disease
dementia
vascular dementia
Lewy body dementia
frontotemporal dementia
multiple sclerosis
amyotrophic lateral sclerosis
ALS
myasthenia gravis
Guillain-Barre syndrome
epilepsy
seizure disorder
migraine
tension headache
cluster headache
trigeminal neuralgia
Bell's palsy
peripheral neuropathy
diabetic neuropathy
restless leg syndrome
essential tremor
Huntington's disease
meningitis
encephalitis
brain tumor
glioblastoma
concussion
traumatic brain injury
depression
major depressive disorder
persistent depressive disorder
bipolar disorder
generalized anxiety disorder
panic disorder
social anxiety disorder
obsessive-compulsive disorder
post-traumatic stress disorder
PTSD
schizophrenia
schizoaffective disorder
attention deficit hyperactivity disorder
ADHD
autism spectrum disorder
eating disorder
anorexia nervosa
bulimia nervosa
substance use disorder
alcohol use disorder
opioid use disorder
eczema
atopic dermatitis
psoriasis
contact dermatitis
seborrheic dermatitis
rosacea
acne
cellulitis
impetigo
herpes simplex
herpes zoster
shingles
tinea
onychomycosis
urticaria
angioedema
melanoma
basal cell carcinoma
squamous cell carcinoma
breast cancer
lung cancer
prostate cancer
colorectal cancer
pancreatic cancer
ovarian cancer
cervical cancer
uterine cancer
endometrial cancer
bladder cancer
kidney cancer
thyroid cancer
liver cancer
hepatocellular carcinoma
esophageal cancer
gastric cancer
testicular cancer
lymphoma
sarcoma
sepsis
septic shock
bacteremia
pneumonia
cellulitis
abscess
osteomyelitis
infective endocarditis
COVID-19
influenza
mononucleosis
HIV
AIDS
hepatitis A
hepatitis B
hepatitis C
Lyme disease
malaria
tuberculosis
Clostridium difficile
C. diff
MRSA
VRE
allergic rhinitis
sinusitis
pharyngitis
tonsillitis
laryngitis
otitis media
otitis externa
conjunctivitis
glaucoma
cataracts
macular degeneration
diabetic retinopathy
retinal detachment
uveitis
vertigo
Meniere's disease
benign paroxysmal positional vertigo
BPPV
tinnitus
hearing loss
```

**anatomy.txt** - Add anatomical terms:
```
head
skull
cranium
brain
cerebrum
cerebellum
brainstem
frontal lobe
temporal lobe
parietal lobe
occipital lobe
hippocampus
amygdala
hypothalamus
thalamus
pituitary gland
pineal gland
meninges
dura mater
arachnoid mater
pia mater
cerebrospinal fluid
ventricles
face
forehead
temple
orbit
eye
cornea
iris
pupil
lens
retina
optic nerve
sclera
conjunctiva
lacrimal gland
eyelid
eyebrow
eyelash
nose
nasal cavity
nasal septum
turbinates
paranasal sinuses
maxillary sinus
frontal sinus
ethmoid sinus
sphenoid sinus
ear
external ear
auricle
pinna
ear canal
tympanic membrane
eardrum
middle ear
ossicles
malleus
incus
stapes
inner ear
cochlea
vestibule
semicircular canals
mouth
oral cavity
lips
tongue
palate
hard palate
soft palate
uvula
tonsils
adenoids
teeth
gums
gingiva
jaw
mandible
maxilla
temporomandibular joint
TMJ
salivary glands
parotid gland
submandibular gland
sublingual gland
neck
cervical spine
larynx
pharynx
nasopharynx
oropharynx
hypopharynx
epiglottis
vocal cords
thyroid gland
parathyroid glands
trachea
esophagus
carotid artery
jugular vein
lymph nodes
cervical lymph nodes
thorax
chest
sternum
ribs
clavicle
scapula
thoracic spine
thoracic cavity
mediastinum
lungs
right lung
left lung
bronchi
bronchioles
alveoli
pleura
visceral pleura
parietal pleura
pleural cavity
diaphragm
heart
right atrium
left atrium
right ventricle
left ventricle
interventricular septum
interatrial septum
tricuspid valve
mitral valve
bicuspid valve
aortic valve
pulmonic valve
pulmonary valve
chordae tendineae
papillary muscles
pericardium
epicardium
myocardium
endocardium
coronary arteries
left anterior descending
LAD
circumflex artery
right coronary artery
aorta
ascending aorta
aortic arch
descending aorta
pulmonary artery
pulmonary veins
superior vena cava
inferior vena cava
abdomen
abdominal cavity
peritoneum
visceral peritoneum
parietal peritoneum
peritoneal cavity
retroperitoneum
stomach
fundus
body
antrum
pylorus
cardia
gastric mucosa
small intestine
duodenum
jejunum
ileum
large intestine
colon
ascending colon
transverse colon
descending colon
sigmoid colon
cecum
appendix
rectum
anus
anal sphincter
liver
right lobe
left lobe
caudate lobe
quadrate lobe
hepatic artery
portal vein
hepatic veins
bile ducts
common bile duct
gallbladder
cystic duct
pancreas
pancreatic head
pancreatic body
pancreatic tail
pancreatic duct
islets of Langerhans
spleen
kidneys
renal cortex
renal medulla
renal pelvis
nephron
glomerulus
Bowman's capsule
proximal tubule
loop of Henle
distal tubule
collecting duct
ureter
bladder
urethra
adrenal glands
adrenal cortex
adrenal medulla
pelvis
pelvic cavity
pelvic floor
hip
ilium
ischium
pubis
sacrum
coccyx
sacroiliac joint
lumbar spine
spine
vertebrae
vertebral body
vertebral arch
spinous process
transverse process
intervertebral disc
spinal cord
spinal canal
nerve roots
cauda equina
upper extremity
shoulder
glenohumeral joint
rotator cuff
supraspinatus
infraspinatus
teres minor
subscapularis
deltoid
humerus
arm
biceps
triceps
elbow
radius
ulna
forearm
wrist
carpal bones
scaphoid
lunate
triquetrum
pisiform
trapezium
trapezoid
capitate
hamate
hand
metacarpals
phalanges
fingers
thumb
lower extremity
thigh
femur
quadriceps
hamstrings
knee
patella
tibia
fibula
meniscus
anterior cruciate ligament
ACL
posterior cruciate ligament
PCL
medial collateral ligament
MCL
lateral collateral ligament
LCL
leg
calf
gastrocnemius
soleus
Achilles tendon
ankle
talus
calcaneus
foot
tarsal bones
metatarsals
toes
plantar fascia
arteries
veins
capillaries
lymphatic system
lymph vessels
lymph nodes
axillary lymph nodes
inguinal lymph nodes
thymus
bone marrow
skin
epidermis
dermis
subcutaneous tissue
hair follicle
sebaceous gland
sweat gland
nail
muscle
skeletal muscle
smooth muscle
cardiac muscle
tendon
ligament
cartilage
fascia
bursa
synovium
synovial fluid
```

**procedures.txt** - Add medical procedures:
```
physical examination
vital signs
blood pressure measurement
heart rate measurement
respiratory rate measurement
temperature measurement
oxygen saturation measurement
pulse oximetry
height measurement
weight measurement
body mass index
BMI calculation
auscultation
palpation
percussion
inspection
electrocardiogram
ECG
EKG
echocardiogram
stress test
exercise stress test
nuclear stress test
cardiac catheterization
coronary angiography
percutaneous coronary intervention
PCI
angioplasty
stent placement
coronary artery bypass graft
CABG
pacemaker insertion
implantable cardioverter defibrillator
ICD implantation
cardioversion
defibrillation
chest X-ray
computed tomography
CT scan
magnetic resonance imaging
MRI
ultrasound
abdominal ultrasound
pelvic ultrasound
transvaginal ultrasound
transrectal ultrasound
carotid ultrasound
Doppler ultrasound
echocardiography
transesophageal echocardiogram
TEE
mammography
bone density scan
DEXA scan
positron emission tomography
PET scan
nuclear medicine scan
thyroid scan
ventilation perfusion scan
V/Q scan
angiography
venography
arteriography
fluoroscopy
barium swallow
barium enema
upper GI series
small bowel follow-through
endoscopy
upper endoscopy
esophagogastroduodenoscopy
EGD
colonoscopy
sigmoidoscopy
enteroscopy
capsule endoscopy
endoscopic retrograde cholangiopancreatography
ERCP
bronchoscopy
cystoscopy
ureteroscopy
hysteroscopy
laparoscopy
arthroscopy
thoracoscopy
mediastinoscopy
biopsy
fine needle aspiration
FNA
core needle biopsy
excisional biopsy
incisional biopsy
punch biopsy
shave biopsy
bone marrow biopsy
liver biopsy
kidney biopsy
prostate biopsy
breast biopsy
lymph node biopsy
sentinel lymph node biopsy
lumbar puncture
spinal tap
thoracentesis
paracentesis
arthrocentesis
joint aspiration
pericardiocentesis
blood draw
venipuncture
arterial blood gas
ABG
central line placement
peripheral IV placement
PICC line placement
port placement
intubation
endotracheal intubation
mechanical ventilation
tracheostomy
chest tube placement
nasogastric tube placement
NG tube
feeding tube placement
PEG tube placement
Foley catheter placement
urinary catheterization
suprapubic catheter placement
dialysis
hemodialysis
peritoneal dialysis
plasmapheresis
transfusion
blood transfusion
platelet transfusion
plasma transfusion
surgery
laparotomy
thoracotomy
craniotomy
laminectomy
discectomy
spinal fusion
joint replacement
total hip replacement
total knee replacement
hip arthroplasty
knee arthroplasty
shoulder replacement
rotator cuff repair
ACL reconstruction
meniscectomy
appendectomy
cholecystectomy
hernia repair
inguinal hernia repair
umbilical hernia repair
hiatal hernia repair
colectomy
hemicolectomy
colostomy
ileostomy
gastrectomy
gastric bypass
sleeve gastrectomy
bariatric surgery
Whipple procedure
pancreaticoduodenectomy
liver resection
hepatectomy
nephrectomy
partial nephrectomy
cystectomy
prostatectomy
radical prostatectomy
transurethral resection of prostate
TURP
orchiectomy
vasectomy
hysterectomy
total abdominal hysterectomy
laparoscopic hysterectomy
oophorectomy
salpingectomy
tubal ligation
cesarean section
C-section
dilation and curettage
D&C
mastectomy
lumpectomy
breast reconstruction
thyroidectomy
parathyroidectomy
adrenalectomy
lymphadenectomy
tonsillectomy
adenoidectomy
septoplasty
rhinoplasty
sinus surgery
myringotomy
tympanostomy tubes
cochlear implant
cataract surgery
LASIK
vitrectomy
corneal transplant
glaucoma surgery
skin graft
wound debridement
incision and drainage
I&D
excision
amputation
fasciotomy
carpal tunnel release
trigger finger release
Dupuytren's contracture release
radiation therapy
chemotherapy
immunotherapy
targeted therapy
hormone therapy
bone marrow transplant
stem cell transplant
organ transplant
kidney transplant
liver transplant
heart transplant
lung transplant
physical therapy
occupational therapy
speech therapy
cardiac rehabilitation
pulmonary rehabilitation
vaccination
immunization
allergy testing
skin prick test
patch testing
pulmonary function test
spirometry
sleep study
polysomnography
electroencephalogram
EEG
electromyography
EMG
nerve conduction study
genetic testing
prenatal screening
amniocentesis
chorionic villus sampling
CVS
newborn screening
```

**lab_tests.txt** - Add laboratory tests:
```
complete blood count
CBC
white blood cell count
WBC
red blood cell count
RBC
hemoglobin
hematocrit
mean corpuscular volume
MCV
mean corpuscular hemoglobin
MCH
mean corpuscular hemoglobin concentration
MCHC
red cell distribution width
RDW
platelet count
mean platelet volume
MPV
differential
neutrophils
lymphocytes
monocytes
eosinophils
basophils
bands
reticulocyte count
peripheral blood smear
basic metabolic panel
BMP
comprehensive metabolic panel
CMP
sodium
potassium
chloride
bicarbonate
carbon dioxide
CO2
blood urea nitrogen
BUN
creatinine
estimated glomerular filtration rate
eGFR
glucose
fasting glucose
random glucose
calcium
magnesium
phosphorus
albumin
total protein
globulin
bilirubin
total bilirubin
direct bilirubin
indirect bilirubin
alkaline phosphatase
ALP
aspartate aminotransferase
AST
alanine aminotransferase
ALT
gamma-glutamyl transferase
GGT
lactate dehydrogenase
LDH
amylase
lipase
uric acid
lipid panel
total cholesterol
LDL cholesterol
HDL cholesterol
triglycerides
VLDL cholesterol
non-HDL cholesterol
apolipoprotein B
lipoprotein A
hemoglobin A1C
A1C
HbA1c
glycated hemoglobin
fructosamine
fasting insulin
C-peptide
thyroid stimulating hormone
TSH
free T4
free thyroxine
free T3
free triiodothyronine
total T4
total T3
thyroid peroxidase antibodies
TPO antibodies
thyroglobulin antibodies
parathyroid hormone
PTH
vitamin D
25-hydroxyvitamin D
1,25-dihydroxyvitamin D
vitamin B12
folate
folic acid
iron
total iron binding capacity
TIBC
transferrin saturation
ferritin
prothrombin time
PT
international normalized ratio
INR
partial thromboplastin time
PTT
activated partial thromboplastin time
aPTT
fibrinogen
D-dimer
factor V Leiden
antithrombin III
protein C
protein S
lupus anticoagulant
anticardiolipin antibodies
beta-2 glycoprotein antibodies
erythrocyte sedimentation rate
ESR
C-reactive protein
CRP
high-sensitivity CRP
hs-CRP
procalcitonin
antinuclear antibody
ANA
rheumatoid factor
RF
anti-cyclic citrullinated peptide
anti-CCP
complement C3
complement C4
total complement
CH50
anti-double stranded DNA
anti-dsDNA
anti-Smith antibody
anti-SSA
anti-SSB
anti-RNP
ANCA
p-ANCA
c-ANCA
anti-GBM antibody
cryoglobulins
serum protein electrophoresis
SPEP
urine protein electrophoresis
UPEP
immunofixation
free light chains
kappa light chains
lambda light chains
beta-2 microglobulin
lactate
lactic acid
ammonia
arterial blood gas
ABG
venous blood gas
VBG
pH
pCO2
pO2
bicarbonate
base excess
oxygen saturation
carboxyhemoglobin
methemoglobin
troponin
troponin I
troponin T
high-sensitivity troponin
BNP
brain natriuretic peptide
NT-proBNP
creatine kinase
CK
CK-MB
myoglobin
homocysteine
lipoprotein-associated phospholipase A2
Lp-PLA2
prostate specific antigen
PSA
free PSA
PSA density
carcinoembryonic antigen
CEA
alpha-fetoprotein
AFP
CA-125
CA 19-9
CA 15-3
CA 27.29
beta-hCG
human chorionic gonadotropin
hCG
lactate dehydrogenase
LDH
cortisol
morning cortisol
ACTH
aldosterone
renin
plasma renin activity
aldosterone to renin ratio
testosterone
free testosterone
estradiol
progesterone
luteinizing hormone
LH
follicle stimulating hormone
FSH
prolactin
growth hormone
insulin-like growth factor 1
IGF-1
DHEA-S
sex hormone binding globulin
SHBG
urinalysis
UA
urine specific gravity
urine pH
urine protein
urine glucose
urine ketones
urine bilirubin
urobilinogen
urine nitrite
urine leukocyte esterase
urine microscopy
urine red blood cells
urine white blood cells
urine bacteria
urine casts
urine crystals
urine culture
urine drug screen
UDS
urine pregnancy test
24-hour urine collection
urine protein creatinine ratio
urine albumin creatinine ratio
UACR
microalbumin
creatinine clearance
stool occult blood
fecal occult blood test
FOBT
fecal immunochemical test
FIT
stool culture
stool ova and parasites
O&P
Clostridium difficile toxin
C. diff toxin
fecal calprotectin
fecal elastase
cerebrospinal fluid analysis
CSF analysis
CSF protein
CSF glucose
CSF cell count
CSF culture
gram stain
acid-fast bacilli
AFB smear
AFB culture
blood culture
wound culture
throat culture
sputum culture
respiratory culture
viral culture
viral PCR
influenza test
COVID-19 test
SARS-CoV-2 PCR
rapid antigen test
strep test
rapid strep test
mononucleosis test
monospot
HIV test
HIV antibody
HIV RNA
viral load
hepatitis panel
hepatitis A antibody
hepatitis B surface antigen
HBsAg
hepatitis B surface antibody
HBsAb
hepatitis B core antibody
HBcAb
hepatitis B e antigen
hepatitis B e antibody
hepatitis B DNA
hepatitis C antibody
hepatitis C RNA
HCV RNA
Lyme disease test
Lyme antibody
Western blot
RPR
rapid plasma reagin
VDRL
FTA-ABS
tuberculosis test
TB test
tuberculin skin test
TST
PPD
QuantiFERON
T-SPOT
drug levels
therapeutic drug monitoring
digoxin level
lithium level
valproic acid level
phenytoin level
carbamazepine level
vancomycin level
gentamicin level
tobramycin level
tacrolimus level
cyclosporine level
sirolimus level
methotrexate level
theophylline level
salicylate level
acetaminophen level
alcohol level
ethanol level
toxicology screen
heavy metal screen
lead level
mercury level
arsenic level
HLA typing
HLA-B27
crossmatch
type and screen
blood type
Rh factor
Coombs test
direct antiglobulin test
indirect antiglobulin test
```

**eponyms.txt** - Add medical eponyms:
```
Alzheimer's disease
Parkinson's disease
Huntington's disease
Crohn's disease
Addison's disease
Cushing's syndrome
Cushing's disease
Graves' disease
Hashimoto's thyroiditis
Hashimoto's disease
Hodgkin lymphoma
Hodgkin's disease
Sjögren's syndrome
Sjogren's syndrome
Raynaud's phenomenon
Raynaud's disease
Paget's disease
Bell's palsy
Guillain-Barré syndrome
Guillain-Barre syndrome
Meniere's disease
Meniere's syndrome
Tourette's syndrome
Tourette syndrome
Asperger's syndrome
Marfan syndrome
Ehlers-Danlos syndrome
Down syndrome
Turner syndrome
Klinefelter syndrome
Prader-Willi syndrome
Angelman syndrome
Fragile X syndrome
Rett syndrome
Williams syndrome
DiGeorge syndrome
Noonan syndrome
Beckwith-Wiedemann syndrome
Wilms tumor
Ewing sarcoma
Kaposi sarcoma
Burkitt lymphoma
Waldenstrom macroglobulinemia
Waldenström's macroglobulinemia
Barrett's esophagus
Barrett esophagus
Zenker's diverticulum
Meckel's diverticulum
Hirschsprung disease
Hirschsprung's disease
Whipple's disease
Whipple procedure
Wilson's disease
Menkes disease
Fabry disease
Gaucher disease
Niemann-Pick disease
Tay-Sachs disease
Pompe disease
McArdle disease
von Gierke disease
Gilbert's syndrome
Gilbert syndrome
Dubin-Johnson syndrome
Crigler-Najjar syndrome
Rotor syndrome
Budd-Chiari syndrome
Reye's syndrome
Reye syndrome
Wernicke's encephalopathy
Wernicke encephalopathy
Korsakoff syndrome
Wernicke-Korsakoff syndrome
Pick's disease
Lewy body dementia
Binswanger disease
Creutzfeldt-Jakob disease
CJD
Charcot-Marie-Tooth disease
Lou Gehrig's disease
ALS
Friedrich's ataxia
Friedreich ataxia
Duchenne muscular dystrophy
Becker muscular dystrophy
Myotonic dystrophy
Steinert disease
Erb's palsy
Erb-Duchenne palsy
Klumpke's palsy
Horner's syndrome
Horner syndrome
Brown-Séquard syndrome
Brown-Sequard syndrome
Wallenberg syndrome
Weber syndrome
Millard-Gubler syndrome
Foville syndrome
Benedikt syndrome
Claude syndrome
Parinaud syndrome
Argyll Robertson pupil
Marcus Gunn pupil
Adie's pupil
Holmes-Adie syndrome
Riley-Day syndrome
Shy-Drager syndrome
multiple system atrophy
Ondine's curse
Cheyne-Stokes respiration
Kussmaul breathing
Biot's respiration
Babinski sign
Babinski reflex
Hoffman's sign
Romberg sign
Romberg test
Trendelenburg sign
Trendelenburg gait
Patrick's test
FABER test
Lachman test
McMurray test
Apley test
Phalen's test
Phalen's maneuver
Tinel's sign
Finkelstein test
Allen's test
Adson's test
Spurling's test
Kernig's sign
Brudzinski's sign
Battle's sign
Raccoon eyes
Cullen's sign
Grey Turner's sign
Murphy's sign
Rovsing's sign
McBurney's point
Psoas sign
Obturator sign
Kehr's sign
Chvostek's sign
Trousseau's sign
Homans' sign
Homan's sign
Virchow's node
Sister Mary Joseph nodule
Osler nodes
Janeway lesions
Roth spots
Koplik spots
Nikolsky sign
Darier's sign
Auspitz sign
Koebner phenomenon
Wickham's striae
Gottron's papules
Heliotrope rash
Levine's sign
Quincke's sign
de Musset's sign
Corrigan's pulse
water hammer pulse
Duroziez's sign
Hill's sign
Austin Flint murmur
Graham Steell murmur
Carey Coombs murmur
Kussmaul's sign
pulsus paradoxus
Beck's triad
Virchow's triad
Charcot's triad
Reynolds' pentad
Whipple's triad
Samter's triad
```

**abbreviations.txt** - Add spoken abbreviation expansions:
```
A one C -> A1C
A one C level -> A1C level
H B A one C -> HbA1c
B P -> BP
blood pressure -> BP
bee pee -> BP
heart rate -> HR
H R -> HR
respiratory rate -> RR
oh two sat -> O2 sat
oxygen sat -> O2 sat
oh two saturation -> O2 saturation
temp -> temperature
C B C -> CBC
complete blood count -> CBC
B M P -> BMP
basic metabolic panel -> BMP
C M P -> CMP
comprehensive metabolic panel -> CMP
T S H -> TSH
thyroid stimulating hormone -> TSH
E K G -> EKG
E C G -> ECG
electrocardiogram -> ECG
M R I -> MRI
C T scan -> CT scan
C T -> CT
P E T scan -> PET scan
I N R -> INR
P T -> PT
prothrombin time -> PT
P T T -> PTT
A P T T -> aPTT
B U N -> BUN
blood urea nitrogen -> BUN
G F R -> GFR
E G F R -> eGFR
glomerular filtration rate -> GFR
A L T -> ALT
A S T -> AST
A L P -> ALP
alkaline phos -> alkaline phosphatase
G G T -> GGT
L D H -> LDH
C R P -> CRP
C reactive protein -> CRP
E S R -> ESR
sed rate -> ESR
sedimentation rate -> ESR
A N A -> ANA
R F -> RF
rheumatoid factor -> RF
B N P -> BNP
N T pro B N P -> NT-proBNP
P S A -> PSA
prostate specific antigen -> PSA
C E A -> CEA
A F P -> AFP
C A one twenty five -> CA-125
C A nineteen nine -> CA 19-9
H C G -> hCG
beta H C G -> beta-hCG
L H -> LH
F S H -> FSH
D H E A S -> DHEA-S
A C T H -> ACTH
I G F one -> IGF-1
U A -> UA
urinalysis -> UA
U D S -> UDS
urine drug screen -> UDS
C S F -> CSF
cerebrospinal fluid -> CSF
A B G -> ABG
arterial blood gas -> ABG
V B G -> VBG
venous blood gas -> VBG
P O two -> pO2
P C O two -> pCO2
C O P D -> COPD
chronic obstructive pulmonary disease -> COPD
C H F -> CHF
congestive heart failure -> CHF
A fib -> atrial fibrillation
A flutter -> atrial flutter
S V T -> SVT
supraventricular tachycardia -> SVT
V T -> VT
ventricular tachycardia -> VT
V fib -> ventricular fibrillation
C A D -> CAD
coronary artery disease -> CAD
M I -> MI
myocardial infarction -> MI
heart attack -> myocardial infarction
A C S -> ACS
acute coronary syndrome -> ACS
D V T -> DVT
deep vein thrombosis -> DVT
P E -> PE
pulmonary embolism -> PE
C V A -> CVA
cerebrovascular accident -> CVA
T I A -> TIA
transient ischemic attack -> TIA
G E R D -> GERD
gastroesophageal reflux disease -> GERD
I B S -> IBS
irritable bowel syndrome -> IBS
I B D -> IBD
inflammatory bowel disease -> IBD
G I -> GI
gastrointestinal -> GI
U T I -> UTI
urinary tract infection -> UTI
B P H -> BPH
benign prostatic hyperplasia -> BPH
C K D -> CKD
chronic kidney disease -> CKD
A K I -> AKI
acute kidney injury -> AKI
E S R D -> ESRD
end stage renal disease -> ESRD
D M -> DM
diabetes mellitus -> DM
D K A -> DKA
diabetic ketoacidosis -> DKA
H H S -> HHS
hyperosmolar hyperglycemic state -> HHS
O S A -> OSA
obstructive sleep apnea -> OSA
R A -> RA
rheumatoid arthritis -> RA
S L E -> SLE
systemic lupus erythematosus -> SLE
O A -> OA
osteoarthritis -> OA
M S -> MS
multiple sclerosis -> MS
A L S -> ALS
amyotrophic lateral sclerosis -> ALS
P T S D -> PTSD
post-traumatic stress disorder -> PTSD
O C D -> OCD
obsessive-compulsive disorder -> OCD
A D H D -> ADHD
attention deficit hyperactivity disorder -> ADHD
B I D -> BID
twice daily -> BID
T I D -> TID
three times daily -> TID
Q I D -> QID
four times daily -> QID
Q D -> daily
once daily -> daily
Q H S -> at bedtime
at bedtime -> QHS
P R N -> as needed
as needed -> PRN
P O -> by mouth
by mouth -> PO
I V -> IV
intravenous -> IV
I M -> IM
intramuscular -> IM
S C -> SC
subcutaneous -> SC
S Q -> SQ
subcutaneous -> SQ
N P O -> NPO
nothing by mouth -> NPO
S O B -> SOB
shortness of breath -> SOB
D O E -> DOE
dyspnea on exertion -> DOE
C P -> chest pain
chest pain -> CP
A O x three -> AOx3
alert and oriented times three -> AOx3
W N L -> WNL
within normal limits -> WNL
N A D -> NAD
no acute distress -> NAD
H P I -> HPI
history of present illness -> HPI
P M H -> PMH
past medical history -> PMH
R O S -> ROS
review of systems -> ROS
P E -> physical exam
physical exam -> PE
A and P -> assessment and plan
assessment and plan -> A&P
D C -> discharge
discharge -> DC
F U -> follow-up
follow up -> follow-up
R T C -> return to clinic
return to clinic -> RTC
E D -> emergency department
emergency department -> ED
E R -> emergency room
emergency room -> ER
I C U -> ICU
intensive care unit -> ICU
O R -> operating room
operating room -> OR
P A C U -> PACU
post-anesthesia care unit -> PACU
L and D -> labor and delivery
labor and delivery -> L&D
N I C U -> NICU
neonatal intensive care unit -> NICU
```

**phonetic_corrections.txt** - Add common mishearings:
```
# Medication mishearings
met formin -> metformin
lysinopril -> lisinopril
lie sinopril -> lisinopril
ace inhibitor -> ACE inhibitor
ace inhibitors -> ACE inhibitors
a tor va statin -> atorvastatin
rosu va statin -> rosuvastatin
sim va statin -> simvastatin
prava statin -> pravastatin
am lode a peen -> amlodipine
am low dip een -> amlodipine
hydro chloro thigh azide -> hydrochlorothiazide
hydro chlorothiazide -> hydrochlorothiazide
h c t z -> HCTZ
fur o semide -> furosemide
furo semide -> furosemide
lasix -> Lasix
metro prolol -> metoprolol
meto prolol -> metoprolol
bi so prolol -> bisoprolol
car veda lol -> carvedilol
ate en o lol -> atenolol
pro pan o lol -> propranolol
dill tea zem -> diltiazem
ver ap a mil -> verapamil
war far in -> warfarin
coumadin -> Coumadin
a pix a ban -> apixaban
eliquis -> Eliquis
riva rox a ban -> rivaroxaban
xarelto -> Xarelto
omepra zole -> omeprazole
panto pra zole -> pantoprazole
esome pra zole -> esomeprazole
lansopra zole -> lansoprazole
ranitidine -> ranitidine
famotidine -> famotidine
gaba pentin -> gabapentin
pre gab a lin -> pregabalin
lyrica -> Lyrica
neurontin -> Neurontin
leva thigh roxine -> levothyroxine
synthroid -> Synthroid
alen dro nate -> alendronate
fosamax -> Fosamax
meth o trex ate -> methotrexate
sul fa sal a zeen -> sulfasalazine
hydro xychloro queen -> hydroxychloroquine
plaquenil -> Plaquenil
tam sue low sin -> tamsulosin
flomax -> Flomax
sil den a fill -> sildenafil
tad a la fill -> tadalafil
viagra -> Viagra
cialis -> Cialis
flutica zone -> fluticasone
bude so nide -> budesonide
mon ta loo cast -> montelukast
singulair -> Singulair
al buterol -> albuterol
sal bu ta mol -> salbutamol
ventolin -> Ventolin
tio tro pee um -> tiotropium
spiriva -> Spiriva
serra tra line -> sertraline
zoloft -> Zoloft
es cita lo pram -> escitalopram
lexapro -> Lexapro
cipralex -> Cipralex
flu ox a teen -> fluoxetine
prozac -> Prozac
ven la fax een -> venlafaxine
dull ox a teen -> duloxetine
cymbalta -> Cymbalta
boo pro pee on -> bupropion
wellbutrin -> Wellbutrin
tra zone done -> trazodone
quetiapine -> quetiapine
seroquel -> Seroquel
risperi done -> risperidone
risperdal -> Risperdal
aripiprazole -> aripiprazole
abilify -> Abilify
clonazepam -> clonazepam
klonopin -> Klonopin
lor az a pam -> lorazepam
ativan -> Ativan
diazepam -> diazepam
valium -> Valium
zolpidem -> zolpidem
ambien -> Ambien
oxy codone -> oxycodone
hydro more phone -> hydromorphone
dilaudid -> Dilaudid
morphine -> morphine
fentanyl -> fentanyl
tramadol -> tramadol
a mox a sill in -> amoxicillin
amoxil -> Amoxil
aug men tin -> Augmentin
a zithro my sin -> azithromycin
zithromax -> Zithromax
z pack -> Z-pack
zee pack -> Z-pack
sip row flox a sin -> ciprofloxacin
cipro -> Cipro
levo flox a sin -> levofloxacin
levaquin -> Levaquin
doxy cycling -> doxycycline
cephalexin -> cephalexin
keflex -> Keflex
metro nida zole -> metronidazole
flagyl -> Flagyl
clin da my sin -> clindamycin
nit ro furen toyn -> nitrofurantoin
macrobid -> Macrobid
trim ethoprim sulfa meth ox a zole -> trimethoprim-sulfamethoxazole
bactrim -> Bactrim
septra -> Septra

# Condition mishearings
high per tension -> hypertension
hyper tension -> hypertension
high blood pressure -> hypertension
high cholesterol -> hypercholesterolemia
die a beet ease -> diabetes
die a beat us -> diabetes
sugar diabetes -> diabetes mellitus
type one diabetes -> type 1 diabetes
type to diabetes -> type 2 diabetes
type two diabetes -> type 2 diabetes
a fib -> atrial fibrillation
a flutter -> atrial flutter
a trial fibrillation -> atrial fibrillation
a trial flutter -> atrial flutter
my o cardinal in farction -> myocardial infarction
em eye -> MI
heart attack -> myocardial infarction
see oh pee dee -> COPD
sea oh pee dee -> COPD
chronic obstructive -> chronic obstructive pulmonary disease
new moan ya -> pneumonia
noo monia -> pneumonia
bron kite us -> bronchitis
gastro esophageal reflux -> gastroesophageal reflux disease
gerd -> GERD
you tea eye -> UTI
urinary infection -> urinary tract infection
see kay dee -> CKD
chronic kidney -> chronic kidney disease
end stage renal -> end stage renal disease
stroke -> cerebrovascular accident
sea vee ay -> CVA
tee eye ay -> TIA
mini stroke -> transient ischemic attack
parkinsons -> Parkinson's disease
alzheimers -> Alzheimer's disease
alz hi mers -> Alzheimer's disease
ms -> multiple sclerosis
m s -> MS
lou gehrigs -> ALS
a l s -> ALS
lupus -> systemic lupus erythematosus
s l e -> SLE
rheumatoid -> rheumatoid arthritis
r a -> RA
osteo arthritis -> osteoarthritis
o a -> OA
osteo pour oh sis -> osteoporosis
osteo pee nia -> osteopenia
high per thyroid -> hyperthyroidism
hypo thyroid -> hypothyroidism
graves -> Graves' disease
hashimotos -> Hashimoto's thyroiditis
hashimoto -> Hashimoto's thyroiditis

# Anatomy mishearings
thor ax -> thorax
ab doe men -> abdomen
peri tone e um -> peritoneum
retro peri tone e um -> retroperitoneum
stomach -> stomach
duo dee num -> duodenum
jejunum -> jejunum
ill e um -> ileum
see come -> cecum
appendix -> appendix
colon -> colon
sigmoid -> sigmoid colon
rectum -> rectum
liver -> liver
gall bladder -> gallbladder
pan cree ass -> pancreas
spleen -> spleen
kidneys -> kidneys
ureters -> ureters
bladder -> bladder
urethra -> urethra
prostate -> prostate
uterus -> uterus
ovaries -> ovaries
cervix -> cervix
vertebrae -> vertebrae
vertebra -> vertebra
inter vertebral -> intervertebral
laminectomy -> laminectomy
discectomy -> discectomy
spinal stenosis -> spinal stenosis
herniated disc -> herniated disc
bulging disc -> bulging disc
sciatica -> sciatica
radiculopathy -> radiculopathy

# Procedure mishearings
echo cardiogram -> echocardiogram
e k g -> EKG
e c g -> ECG
electrocardiogram -> electrocardiogram
c t scan -> CT scan
cat scan -> CT scan
m r i -> MRI
magnetic resonance -> magnetic resonance imaging
ultra sound -> ultrasound
x ray -> X-ray
xray -> X-ray
mammogram -> mammography
colonoscopy -> colonoscopy
endoscopy -> endoscopy
e g d -> EGD
bronchoscopy -> bronchoscopy
biopsy -> biopsy
lumbar puncture -> lumbar puncture
spinal tap -> lumbar puncture
thoracentesis -> thoracentesis
paracentesis -> paracentesis
cath lab -> catheterization laboratory
cardiac cath -> cardiac catheterization
angiogram -> angiography
angioplasty -> angioplasty
stent -> stent
bypass -> coronary artery bypass graft
c a b g -> CABG
cabbage -> CABG
pacemaker -> pacemaker
i c d -> ICD
defibrillator -> implantable cardioverter defibrillator
dialysis -> dialysis
hemo dialysis -> hemodialysis
peritoneal dialysis -> peritoneal dialysis
transplant -> transplant
chemo -> chemotherapy
radiation -> radiation therapy

# Lab test mishearings
see bee see -> CBC
complete blood count -> CBC
hem o globe in -> hemoglobin
he matt o crit -> hematocrit
white count -> white blood cell count
platelet count -> platelet count
bee em pee -> BMP
see em pee -> CMP
metabolic panel -> metabolic panel
sodium -> sodium
potassium -> potassium
chloride -> chloride
bicarb -> bicarbonate
bun -> BUN
creatinine -> creatinine
g f r -> GFR
glucose -> glucose
blood sugar -> glucose
calcium -> calcium
magnesium -> magnesium
liver function -> liver function tests
a l t -> ALT
a s t -> AST
bilirubin -> bilirubin
alkaline phosphatase -> alkaline phosphatase
lip id panel -> lipid panel
cholesterol -> cholesterol
l d l -> LDL
h d l -> HDL
triglycerides -> triglycerides
a one c -> A1C
hemoglobin a one c -> hemoglobin A1C
thyroid panel -> thyroid panel
t s h -> TSH
free t four -> free T4
b n p -> BNP
troponin -> troponin
d dimer -> D-dimer
i n r -> INR
p t -> PT
p t t -> PTT
sed rate -> ESR
e s r -> ESR
c reactive protein -> C-reactive protein
c r p -> CRP
urinalysis -> urinalysis
you a -> UA
urine culture -> urine culture
blood culture -> blood culture
```

**Step 3: Delete old default_custom_vocab.txt**

Run: `rm src-tauri/resources/default_custom_vocab.txt`

**Step 4: Verify file sizes**

Run: `wc -l src-tauri/resources/vocab_sources/*.txt`
Expected: Total around 2,500-3,000 lines (initial migration, will expand later)

**Step 5: Commit**

```bash
git add src-tauri/resources/vocab_sources/
git rm src-tauri/resources/default_custom_vocab.txt
git commit -m "feat: migrate existing vocabulary to categorized source files

- medications_generic.txt: 200+ generic drug names
- medications_brand.txt: 200+ brand name mappings
- conditions.txt: 300+ medical conditions
- anatomy.txt: 400+ anatomical terms
- procedures.txt: 300+ medical procedures
- lab_tests.txt: 300+ laboratory tests
- eponyms.txt: 150+ medical eponyms
- abbreviations.txt: 200+ abbreviation expansions
- phonetic_corrections.txt: 300+ common mishearings

Delete old default_custom_vocab.txt"
```

---

## Task 5: Create Vocabulary Build Script

**Files:**
- Create: `src-tauri/src/vocab_builder.rs`
- Modify: `src-tauri/build.rs`

**Step 1: Create vocabulary builder module**

Create `src-tauri/src/vocab_builder.rs`:

```rust
//! Build-time vocabulary compilation.
//!
//! This module is used by build.rs to compile vocabulary source files
//! into a binary index.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;

use crate::medical_index::{MedicalIndex, TermCategory};

/// Source file to category mapping
const SOURCE_CATEGORIES: &[(&str, TermCategory)] = &[
    ("medications_generic.txt", TermCategory::MedicationGeneric),
    ("medications_brand.txt", TermCategory::MedicationBrand),
    ("conditions.txt", TermCategory::Condition),
    ("anatomy.txt", TermCategory::Anatomy),
    ("procedures.txt", TermCategory::Procedure),
    ("lab_tests.txt", TermCategory::LabTest),
    ("eponyms.txt", TermCategory::Eponym),
    ("abbreviations.txt", TermCategory::Abbreviation),
    ("phonetic_corrections.txt", TermCategory::PhoneticCorrection),
];

/// Compile vocabulary source files into binary index.
pub fn compile_vocabulary(sources_dir: &Path, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut all_terms: HashSet<String> = HashSet::new();
    let mut corrections: HashMap<String, String> = HashMap::new();
    let mut categories: HashMap<String, TermCategory> = HashMap::new();

    for (filename, category) in SOURCE_CATEGORIES {
        let file_path = sources_dir.join(filename);
        if !file_path.exists() {
            eprintln!("Warning: Source file not found: {}", file_path.display());
            continue;
        }

        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for correction mapping (contains " -> ")
            if let Some(arrow_pos) = trimmed.find(" -> ") {
                let variant = trimmed[..arrow_pos].trim().to_string();
                let correct = trimmed[arrow_pos + 4..].trim().to_string();

                // Add both variant and correct form
                corrections.insert(variant.to_lowercase(), correct.clone());
                all_terms.insert(correct.clone());
                categories.insert(correct.to_lowercase(), *category);
            } else {
                // Simple term
                all_terms.insert(trimmed.to_string());
                categories.insert(trimmed.to_lowercase(), *category);
            }
        }
    }

    let terms: Vec<String> = all_terms.into_iter().collect();

    println!(
        "cargo:warning=Compiled {} terms and {} corrections into medical vocabulary index",
        terms.len(),
        corrections.len()
    );

    let index = MedicalIndex::build(terms, corrections, categories);
    index.save_to_file(output_path)?;

    Ok(())
}

/// Get statistics about vocabulary sources.
pub fn get_vocabulary_stats(sources_dir: &Path) -> HashMap<String, usize> {
    let mut stats = HashMap::new();

    for (filename, _) in SOURCE_CATEGORIES {
        let file_path = sources_dir.join(filename);
        if let Ok(file) = File::open(&file_path) {
            let reader = BufReader::new(file);
            let count = reader
                .lines()
                .filter_map(|l| l.ok())
                .filter(|l| {
                    let t = l.trim();
                    !t.is_empty() && !t.starts_with('#')
                })
                .count();
            stats.insert(filename.to_string(), count);
        }
    }

    stats
}
```

**Step 2: Update build.rs to compile vocabulary**

Add to `src-tauri/build.rs` (add this function and call it from main):

```rust
use std::path::Path;

fn compile_medical_vocabulary() {
    let sources_dir = Path::new("resources/vocab_sources");
    let output_path = Path::new("resources/medical_vocab.bin");

    // Only recompile if sources changed
    for entry in std::fs::read_dir(sources_dir).unwrap() {
        if let Ok(entry) = entry {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    // Import the vocab builder (this requires making it available at build time)
    // For now, we'll use a simpler inline implementation

    use std::collections::{HashMap, HashSet};
    use std::fs::File;
    use std::io::{BufRead, BufReader, BufWriter, Write};

    let source_files = [
        "medications_generic.txt",
        "medications_brand.txt",
        "conditions.txt",
        "anatomy.txt",
        "procedures.txt",
        "lab_tests.txt",
        "eponyms.txt",
        "abbreviations.txt",
        "phonetic_corrections.txt",
    ];

    let mut all_terms: HashSet<String> = HashSet::new();
    let mut corrections: HashMap<String, String> = HashMap::new();

    for filename in &source_files {
        let file_path = sources_dir.join(filename);
        if !file_path.exists() {
            println!("cargo:warning=Vocabulary source not found: {}", file_path.display());
            continue;
        }

        if let Ok(file) = File::open(&file_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                if let Some(arrow_pos) = trimmed.find(" -> ") {
                    let variant = trimmed[..arrow_pos].trim().to_string();
                    let correct = trimmed[arrow_pos + 4..].trim().to_string();
                    corrections.insert(variant.to_lowercase(), correct.clone());
                    all_terms.insert(correct);
                } else {
                    all_terms.insert(trimmed.to_string());
                }
            }
        }
    }

    println!(
        "cargo:warning=Medical vocabulary: {} terms, {} corrections",
        all_terms.len(),
        corrections.len()
    );

    // Write as JSON for now (simpler than bincode in build script)
    // The runtime will parse this and build the BK-tree
    let output = serde_json::json!({
        "terms": all_terms.into_iter().collect::<Vec<_>>(),
        "corrections": corrections,
    });

    let file = File::create(output_path).expect("Failed to create medical_vocab.bin");
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &output).expect("Failed to write vocabulary");

    println!("cargo:warning=Medical vocabulary compiled to {}", output_path.display());
}
```

Then in `main()`, add the call:
```rust
fn main() {
    compile_medical_vocabulary();
    generate_tray_translations();
    tauri_build::build();
}
```

**Step 3: Add serde_json to build-dependencies**

In `src-tauri/Cargo.toml`, add:

```toml
[build-dependencies]
serde_json = "1.0"
```

**Step 4: Test build**

Run: `cd src-tauri && cargo build`
Expected: Build succeeds, prints vocabulary statistics

**Step 5: Commit**

```bash
git add src-tauri/src/vocab_builder.rs src-tauri/build.rs src-tauri/Cargo.toml
git commit -m "feat: add vocabulary compilation to build process

- Create vocab_builder.rs module
- Update build.rs to compile vocabulary sources
- Generate medical_vocab.bin at build time
- Add serde_json build dependency"
```

---

## Task 6: Update MedicalVocabulary to Use New Index

**Files:**
- Modify: `src-tauri/src/medical_vocab.rs`

**Step 1: Add imports and update struct**

At the top of `medical_vocab.rs`, update imports:

```rust
use crate::medical_index::MedicalIndexRuntime;
use std::path::Path;
```

Update the `MedicalVocabulary` struct to include the new index:

```rust
pub struct MedicalVocabulary {
    /// New: Pre-compiled medical index with BK-tree
    index: Option<MedicalIndexRuntime>,

    // Keep existing fields for backward compatibility during transition
    terms: HashMap<String, String>,
    canadian_spellings: HashMap<String, String>,
    common_corrections: HashMap<String, Vec<String>>,
    medication_corrections: HashMap<String, String>,
    custom_vocab_path: Option<PathBuf>,
    #[serde(skip)]
    regex_cache: HashMap<String, Regex>,
}
```

**Step 2: Update constructor to load index**

Update the `new()` method:

```rust
impl MedicalVocabulary {
    pub fn new() -> Self {
        let mut vocab = Self {
            index: None,
            terms: HashMap::new(),
            canadian_spellings: HashMap::new(),
            common_corrections: HashMap::new(),
            medication_corrections: HashMap::new(),
            custom_vocab_path: None,
            regex_cache: HashMap::new(),
        };

        // Try to load pre-compiled index
        vocab.load_index();

        // Load user's custom vocabulary on top
        vocab.load_user_custom_vocab();

        vocab
    }

    fn load_index(&mut self) {
        // Try to load from resources directory
        let index_path = Path::new("resources/medical_vocab.bin");

        // Also try relative to executable
        let exe_path = std::env::current_exe().ok();
        let resource_paths = [
            index_path.to_path_buf(),
            exe_path
                .as_ref()
                .and_then(|p| p.parent())
                .map(|p| p.join("resources/medical_vocab.bin"))
                .unwrap_or_default(),
            exe_path
                .as_ref()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.join("Resources/resources/medical_vocab.bin"))
                .unwrap_or_default(),
        ];

        for path in &resource_paths {
            if path.exists() {
                match MedicalIndexRuntime::load(path) {
                    Ok(index) => {
                        self.index = Some(index);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Failed to load medical index from {:?}: {}", path, e);
                    }
                }
            }
        }

        // Fall back to loading JSON format from build
        self.load_index_json();
    }

    fn load_index_json(&mut self) {
        // Fallback: load JSON format produced by build.rs
        // This allows the index to work even without bincode serialization
        let json_paths = [
            Path::new("resources/medical_vocab.bin").to_path_buf(),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("resources/medical_vocab.bin")))
                .unwrap_or_default(),
        ];

        for path in &json_paths {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&contents) {
                        // Extract terms and corrections from JSON
                        if let Some(terms) = data.get("terms").and_then(|t| t.as_array()) {
                            for term in terms {
                                if let Some(t) = term.as_str() {
                                    self.terms.insert(t.to_lowercase(), t.to_string());
                                }
                            }
                        }
                        if let Some(corrections) = data.get("corrections").and_then(|c| c.as_object()) {
                            for (variant, correct) in corrections {
                                if let Some(c) = correct.as_str() {
                                    // Add to appropriate correction map based on word count
                                    if variant.contains(' ') {
                                        self.common_corrections
                                            .entry(c.to_string())
                                            .or_insert_with(Vec::new)
                                            .push(variant.clone());
                                    } else {
                                        self.medication_corrections.insert(variant.clone(), c.to_string());
                                    }
                                }
                            }
                        }
                        return;
                    }
                }
            }
        }
    }

    fn load_user_custom_vocab(&mut self) {
        // Keep existing custom vocab loading logic
        if let Some(path) = Self::get_default_custom_vocab_path() {
            self.custom_vocab_path = Some(path.clone());
            if path.exists() {
                self.load_custom_vocabulary_txt(&path);
            }
        }
    }
}
```

**Step 3: Update process_text to use index**

Update the `process_text` method to use the new index when available:

```rust
pub fn process_text(&mut self, text: &str) -> String {
    let mut result = text.to_string();

    // Use new index if available
    if let Some(ref index) = self.index {
        result = index.process_text(&result);
    }

    // Apply legacy corrections (for backward compatibility and user custom vocab)
    result = self.apply_medication_corrections(&result);
    result = self.apply_common_corrections(&result);
    result = self.apply_canadian_spellings(&result);

    // Format medical numbers (keep existing logic)
    result = self.format_medical_numbers(&result);

    result
}
```

**Step 4: Run tests**

Run: `cd src-tauri && cargo test medical_vocab`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src-tauri/src/medical_vocab.rs
git commit -m "feat: integrate MedicalIndex into MedicalVocabulary

- Load pre-compiled index on initialization
- Fall back to JSON format if binary not available
- Use index for fuzzy matching when available
- Keep legacy corrections for user custom vocab
- Maintain backward compatibility"
```

---

## Task 7: Expand Vocabulary from Public Databases

**Files:**
- Modify: All `src-tauri/resources/vocab_sources/*.txt` files

**Step 1: Research and download RxNorm data**

RxNorm provides normalized drug names. Download from: https://www.nlm.nih.gov/research/umls/rxnorm/

Extract top prescribed medications and add to `medications_generic.txt`:
- Focus on: Top 500 most prescribed drugs in US/Canada
- Include: All dosage forms and strengths as separate terms where relevant
- Format: One drug name per line

**Step 2: Expand with SNOMED-CT clinical findings**

SNOMED-CT provides standardized medical terminology. Add common clinical findings to `conditions.txt`:
- Focus on: Primary care diagnoses, chronic disease management
- Include: Synonyms and common variations
- Target: 3,500 total terms

**Step 3: Add ICD-10-CM descriptions**

ICD-10 diagnosis codes with descriptions. Add to `conditions.txt`:
- Focus on: Frequently used diagnosis codes
- Include: Full descriptions for recognition

**Step 4: Expand anatomy from FMA**

Foundational Model of Anatomy. Add to `anatomy.txt`:
- Include: All body systems
- Target: 1,500 total anatomical terms

**Step 5: Add LOINC lab tests**

LOINC provides standardized lab test names. Add to `lab_tests.txt`:
- Include: Common panels and individual tests
- Include: Spoken forms (how doctors say them)
- Target: 800 total terms

**Step 6: Expand phonetic corrections**

Based on common Whisper/speech recognition errors:
- Research Dragon Medical user forums
- Add common mishearings for all medication names
- Target: 800+ correction mappings

**Step 7: Verify counts**

Run: `wc -l src-tauri/resources/vocab_sources/*.txt`
Expected totals:
- medications_generic.txt: ~4,000
- medications_brand.txt: ~2,000
- conditions.txt: ~3,500
- anatomy.txt: ~1,500
- procedures.txt: ~1,500
- lab_tests.txt: ~800
- eponyms.txt: ~500
- abbreviations.txt: ~400
- phonetic_corrections.txt: ~800
- **Total: ~15,000+ terms**

**Step 8: Test build**

Run: `cd src-tauri && cargo build`
Expected: Build succeeds, vocabulary statistics show 15,000+ terms

**Step 9: Commit**

```bash
git add src-tauri/resources/vocab_sources/
git commit -m "feat: expand vocabulary to 15,000+ terms

Sources:
- RxNorm: 4,000 generic medications
- RxNorm + Health Canada: 2,000 brand names
- SNOMED-CT + ICD-10: 3,500 conditions
- FMA: 1,500 anatomical terms
- CPT + SNOMED: 1,500 procedures
- LOINC: 800 lab tests
- Curated: 500 eponyms, 400 abbreviations
- Phonetic: 800+ correction mappings"
```

---

## Task 8: Performance Testing and Tuning

**Files:**
- Modify: `src-tauri/benches/medical_vocab_benchmark.rs`

**Step 1: Update benchmark with realistic test cases**

Update the benchmark to test with 15,000+ vocabulary:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use handy::medical_vocab::MedicalVocabulary;

fn benchmark_process_text(c: &mut Criterion) {
    let mut vocab = MedicalVocabulary::new();

    // Realistic medical dictation samples
    let test_cases = vec![
        "Patient presents with hyper tension and type two diabetes on met formin five hundred milligrams twice daily",
        "Labs show a one c of seven point two, tsh normal, lipid panel with elevated l d l",
        "Assessment: uncontrolled diabetes, high cholesterol, recommend increasing rosu va statin dose",
        "Physical exam: blood pressure one forty over ninety, heart rate seventy two, oh two sat ninety eight percent on room air",
        "History of my o cardinal in farction, currently on a pix a ban and metro prolol",
        "Differential diagnosis includes c o p d exacerbation versus pneumonia",
        "Plan: order c t chest, start leva flox a sin, follow up in one week",
        "Patient with a fib on coumadin, i n r therapeutic at two point five",
        "Complains of gerd symptoms, increase panto pra zole to forty milligrams daily",
        "Review of systems positive for shortness of breath, negative for chest pain",
    ];

    c.bench_function("process_text_medical_samples", |b| {
        b.iter(|| {
            for text in &test_cases {
                black_box(vocab.process_text(text));
            }
        })
    });

    // Benchmark single word lookups
    let single_words = vec![
        "metformin", "lisinopril", "atorvastatin", "hypertension",
        "diabetes", "pneumonia", "bronchitis", "colonoscopy",
    ];

    c.bench_function("single_word_lookup", |b| {
        b.iter(|| {
            for word in &single_words {
                black_box(vocab.process_text(word));
            }
        })
    });

    // Benchmark fuzzy matching
    let fuzzy_words = vec![
        "metfromin", "lysionpril", "atorvastaitn", "hypertnesion",
        "diabeets", "pnuemonia", "bronchitsi", "colonscopy",
    ];

    c.bench_function("fuzzy_word_lookup", |b| {
        b.iter(|| {
            for word in &fuzzy_words {
                black_box(vocab.process_text(word));
            }
        })
    });
}

fn benchmark_initialization(c: &mut Criterion) {
    c.bench_function("vocabulary_initialization", |b| {
        b.iter(|| {
            black_box(MedicalVocabulary::new())
        })
    });
}

criterion_group!(benches, benchmark_process_text, benchmark_initialization);
criterion_main!(benches);
```

**Step 2: Run benchmarks**

Run: `cd src-tauri && cargo bench`
Expected:
- Initialization: <10ms
- 10 medical samples: <15ms
- Single word lookup: <1ms
- Fuzzy word lookup: <2ms

**Step 3: Tune thresholds if needed**

If performance is not meeting targets, adjust:
- BK-tree max edit distance (currently 2)
- Phonetic score threshold (currently 0.7)
- Minimum word length for fuzzy matching (currently 3)

**Step 4: Commit**

```bash
git add src-tauri/benches/medical_vocab_benchmark.rs
git commit -m "test: update benchmarks for expanded vocabulary

- Add realistic medical dictation samples
- Benchmark fuzzy matching performance
- Verify <15ms processing target"
```

---

## Task 9: Integration Testing

**Files:**
- Create: `src-tauri/tests/medical_vocab_integration.rs`

**Step 1: Write integration tests**

Create `src-tauri/tests/medical_vocab_integration.rs`:

```rust
use handy::medical_vocab::MedicalVocabulary;

#[test]
fn test_medication_corrections() {
    let mut vocab = MedicalVocabulary::new();

    // Test common medication mishearings
    assert!(vocab.process_text("met formin").contains("metformin"));
    assert!(vocab.process_text("lysinopril").contains("lisinopril"));
    assert!(vocab.process_text("ator va statin").contains("atorvastatin"));
}

#[test]
fn test_condition_recognition() {
    let mut vocab = MedicalVocabulary::new();

    // Test condition corrections
    assert!(vocab.process_text("high per tension").contains("hypertension"));
    assert!(vocab.process_text("a fib").contains("atrial fibrillation"));
    assert!(vocab.process_text("see oh pee dee").contains("COPD"));
}

#[test]
fn test_abbreviation_expansion() {
    let mut vocab = MedicalVocabulary::new();

    // Test abbreviation handling
    assert!(vocab.process_text("A one C level").contains("A1C"));
    assert!(vocab.process_text("T S H normal").contains("TSH"));
    assert!(vocab.process_text("bee em pee").contains("BMP"));
}

#[test]
fn test_case_preservation() {
    let mut vocab = MedicalVocabulary::new();

    // Test that case is preserved appropriately
    let result = vocab.process_text("METFORMIN");
    assert!(result.contains("METFORMIN") || result.contains("Metformin"));
}

#[test]
fn test_no_false_positives() {
    let mut vocab = MedicalVocabulary::new();

    // Test that common words are not incorrectly corrected
    let text = "The patient came in today for a follow-up appointment.";
    let result = vocab.process_text(text);

    // These common words should not be changed
    assert!(result.contains("patient"));
    assert!(result.contains("today"));
    assert!(result.contains("appointment"));
}

#[test]
fn test_user_custom_vocab_priority() {
    // User custom vocabulary should take priority over defaults
    // This test would require setting up a test custom vocab file
}

#[test]
fn test_real_dictation_sample() {
    let mut vocab = MedicalVocabulary::new();

    let dictation = "Patient is a fifty five year old male with history of \
        hyper tension diabetes type two and high cholesterol. Currently on \
        met formin five hundred milligrams twice daily, lysinopril ten milligrams \
        daily, and ator va statin twenty milligrams at bedtime. Labs today show \
        a one c of seven point five and l d l of one twenty. Blood pressure \
        one thirty two over eighty four, heart rate seventy six. Assessment: \
        diabetes not at goal, recommend increasing met formin to one thousand \
        milligrams twice daily.";

    let result = vocab.process_text(dictation);

    // Verify key corrections were made
    assert!(result.contains("hypertension") || result.contains("Hypertension"));
    assert!(result.contains("metformin") || result.contains("Metformin"));
    assert!(result.contains("lisinopril") || result.contains("Lisinopril"));
    assert!(result.contains("atorvastatin") || result.contains("Atorvastatin"));
    assert!(result.contains("A1C") || result.contains("a1c"));
    assert!(result.contains("LDL") || result.contains("ldl"));
}
```

**Step 2: Run integration tests**

Run: `cd src-tauri && cargo test --test medical_vocab_integration`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src-tauri/tests/medical_vocab_integration.rs
git commit -m "test: add integration tests for medical vocabulary

- Test medication corrections
- Test condition recognition
- Test abbreviation expansion
- Test case preservation
- Test real dictation sample"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md` (if exists)

**Step 1: Update CLAUDE.md**

Add section about medical vocabulary:

```markdown
## Medical Vocabulary System

The app includes a comprehensive medical vocabulary with 15,000+ terms for accurate medical dictation.

### Vocabulary Sources
- **Medications:** 6,000+ generic and brand names from RxNorm
- **Conditions:** 3,500+ diagnoses from SNOMED-CT/ICD-10
- **Anatomy:** 1,500+ anatomical terms from FMA
- **Procedures:** 1,500+ procedures from CPT/SNOMED
- **Lab Tests:** 800+ tests from LOINC
- **Eponyms:** 500+ named conditions/signs
- **Abbreviations:** 400+ spoken-to-written mappings
- **Phonetic Corrections:** 800+ common mishearings

### Architecture
- Pre-compiled binary index loaded at startup (~5ms)
- BK-tree for O(log n) fuzzy matching
- Metaphone phonetic scoring for speech recognition errors
- User custom vocabulary loads on top with priority

### Adding Custom Terms
Users can add their own terms via the custom vocabulary file:
- macOS: `~/Library/Application Support/com.pais.handy/custom_medical_vocab.txt`
- Windows: `%APPDATA%\com.pais.handy\custom_medical_vocab.txt`

Format:
```
# Comments start with #
medical_term
wrong_spelling -> correct_spelling
```
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update documentation for medical vocabulary system"
```

---

## Task 11: Final Validation

**Step 1: Clean build**

Run:
```bash
cd src-tauri
cargo clean
cargo build --release
```
Expected: Build succeeds

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Run benchmarks**

Run: `cargo bench`
Expected: Performance within targets

**Step 4: Test in development mode**

Run: `bun run tauri dev`
Expected: App launches, medical mode works correctly

**Step 5: Test real dictation**

Manually test with real medical dictation samples:
1. Enable medical mode in settings
2. Dictate: "Patient has hypertension and diabetes on metformin"
3. Verify corrections are applied correctly
4. Verify no performance degradation

**Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete medical vocabulary expansion to 15,000+ terms

This major update expands the medical vocabulary from ~560 to 15,000+ terms
to rival Dragon Dictate accuracy:

- BK-tree fuzzy matching with phonetic scoring
- Pre-compiled binary index for fast loading
- Sources: RxNorm, SNOMED-CT, ICD-10, LOINC, FMA
- Comprehensive coverage of medications, conditions, anatomy, procedures
- Common speech recognition error corrections

Performance: <15ms for 100-word transcription, <10ms initialization"
```

---

## Summary

This plan implements the medical vocabulary expansion in 11 tasks:

1. **Add Dependencies** - bincode, memmap2, rphonetic
2. **Create MedicalIndex Module** - BK-tree + phonetic matching
3. **Create Vocabulary Sources** - Directory structure
4. **Migrate Existing Vocabulary** - Categorize 560 terms
5. **Create Build Script** - Compile vocabulary at build time
6. **Update MedicalVocabulary** - Integrate new index
7. **Expand Vocabulary** - Add 15,000+ terms from databases
8. **Performance Testing** - Benchmark and tune
9. **Integration Testing** - Validate corrections
10. **Update Documentation** - Document the system
11. **Final Validation** - Full testing
