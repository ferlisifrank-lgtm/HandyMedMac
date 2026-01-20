use bk_tree::BKTree;
use natural::phonetics::soundex;
use regex::Regex;
use std::collections::HashMap;
use strsim::levenshtein;

// Threshold for switching between Phase 2 (bucketing) and Phase 3 (BK-tree)
const BKTREE_THRESHOLD: usize = 200;

/// Shared implementation for applying word corrections
/// Takes a closure that provides candidates for a given cleaned word
fn apply_corrections_impl<F>(
    text: &str,
    original_words: &[String],
    words_lower: &[String],
    threshold: f64,
    mut find_candidates: F,
) -> String
where
    F: FnMut(&str) -> Vec<(usize, String)>,
{
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut corrected_words = Vec::new();

    for word in words {
        let cleaned_word = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();

        if cleaned_word.is_empty() || cleaned_word.len() > 50 {
            corrected_words.push(word.to_string());
            continue;
        }

        let candidates = find_candidates(&cleaned_word);
        let mut best_match: Option<&String> = None;
        let mut best_score = f64::MAX;

        for (dist, candidate) in candidates {
            if let Some(original_idx) = words_lower.iter().position(|w| w == &candidate) {
                // Early exit for exact match
                if dist == 0 {
                    best_match = Some(&original_words[original_idx]);
                    break;
                }

                let max_len = cleaned_word.len().max(candidate.len()) as f64;
                let levenshtein_score = if max_len > 0.0 {
                    dist as f64 / max_len
                } else {
                    1.0
                };

                if levenshtein_score > threshold {
                    continue;
                }

                let phonetic_match = soundex(&cleaned_word, &candidate);
                let combined_score = if phonetic_match {
                    levenshtein_score * 0.3
                } else {
                    levenshtein_score
                };

                if combined_score < threshold && combined_score < best_score {
                    best_match = Some(&original_words[original_idx]);
                    best_score = combined_score;
                }
            }
        }

        if let Some(replacement) = best_match {
            let corrected = preserve_case_pattern(word, replacement);
            let (prefix, suffix) = extract_punctuation(word);
            corrected_words.push(format!("{}{}{}", prefix, corrected, suffix));
        } else {
            corrected_words.push(word.to_string());
        }
    }

    corrected_words.join(" ")
}

/// Cached custom words processor for performance optimization
/// Caches lowercased words and length buckets to avoid repeated preprocessing
pub struct CustomWordsCache {
    words_lower: Vec<String>,
    length_buckets: HashMap<usize, Vec<(usize, String)>>,
    bk_tree: Option<BKTree<String, bk_tree::metrics::Levenshtein>>,
}

impl CustomWordsCache {
    /// Create a new cache from custom words
    pub fn new(custom_words: &[String]) -> Self {
        if custom_words.is_empty() {
            return Self {
                words_lower: Vec::new(),
                length_buckets: HashMap::new(),
                bk_tree: None,
            };
        }

        let words_lower: Vec<String> = custom_words.iter().map(|w| w.to_lowercase()).collect();

        // Build data structure based on vocabulary size
        if custom_words.len() >= BKTREE_THRESHOLD {
            // Build BK-tree for large vocabularies
            let mut tree = BKTree::new(bk_tree::metrics::Levenshtein);
            for word in &words_lower {
                tree.add(word.clone());
            }
            Self {
                words_lower,
                length_buckets: HashMap::new(),
                bk_tree: Some(tree),
            }
        } else {
            // Build length buckets for small vocabularies
            let mut length_buckets: HashMap<usize, Vec<(usize, String)>> = HashMap::new();
            for (i, word_lower) in words_lower.iter().enumerate() {
                let len = word_lower.len();
                length_buckets
                    .entry(len)
                    .or_default()
                    .push((i, word_lower.clone()));
            }
            Self {
                words_lower,
                length_buckets,
                bk_tree: None,
            }
        }
    }

    /// Apply corrections using the cached data
    pub fn apply_corrections(
        &self,
        text: &str,
        original_words: &[String],
        threshold: f64,
    ) -> String {
        if self.words_lower.is_empty() {
            return text.to_string();
        }

        if self.bk_tree.is_some() {
            self.apply_with_bktree(text, original_words, threshold)
        } else {
            self.apply_with_bucketing(text, original_words, threshold)
        }
    }

    fn apply_with_bktree(&self, text: &str, original_words: &[String], threshold: f64) -> String {
        let tree = self.bk_tree.as_ref().unwrap();
        apply_corrections_impl(
            text,
            original_words,
            &self.words_lower,
            threshold,
            |cleaned_word| {
                let max_word_len = cleaned_word.len();
                let max_distance = ((max_word_len as f64) * threshold * 1.5).ceil() as u32;
                let candidates = tree.find(cleaned_word, max_distance);
                candidates
                    .into_iter()
                    .map(|(dist, word)| (dist as usize, word.clone()))
                    .collect()
            },
        )
    }

    fn apply_with_bucketing(
        &self,
        text: &str,
        original_words: &[String],
        threshold: f64,
    ) -> String {
        apply_corrections_impl(
            text,
            original_words,
            &self.words_lower,
            threshold,
            |cleaned_word| {
                let target_len = cleaned_word.len();
                let min_len = target_len.saturating_sub(5);
                let max_len = target_len + 5;

                let mut candidates = Vec::new();
                for bucket_len in min_len..=max_len {
                    if let Some(bucket) = self.length_buckets.get(&bucket_len) {
                        for (_idx, custom_word_lower) in bucket {
                            let dist = levenshtein(cleaned_word, custom_word_lower);
                            candidates.push((dist, custom_word_lower.clone()));
                        }
                    }
                }
                candidates
            },
        )
    }
}

/// Normalizes spoken year phrases into numeric format
///
/// This function converts spoken year formats like "twenty twenty-five" or
/// "two thousand twenty-five" into numeric format like "2025".
///
/// Supported formats:
/// - "twenty twenty-five" → "2025"
/// - "two thousand twenty-five" → "2025"
/// - "nineteen ninety-nine" → "1999"
/// - "two thousand" → "2000"
///
/// # Arguments
/// * `text` - The input text to normalize
///
/// # Returns
/// The text with normalized year formats
pub fn normalize_years(text: &str) -> String {
    type ConverterFn = Box<dyn Fn(&regex::Captures) -> Option<String>>;

    let patterns: Vec<(Regex, ConverterFn)> = vec![
        // Pattern: "twenty twenty-five" or "twenty twenty five" → "2025"
        (
            Regex::new(r"(?i)\b(twenty)[\s-]+(twenty)[\s-]+(\w+)\b").unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let last_digit = caps.get(3)?.as_str();
                let ones = word_to_number(last_digit)?;
                if ones <= 9 {
                    Some(format!("20{}", 20 + ones))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "two thousand twenty-five" or "two thousand and twenty five" → "2025"
        // Process compound numbers first (two words)
        (
            Regex::new(r"(?i)\b(two\s+thousand)(?:\s+and)?\s+(\w+[\s-]+\w+)\b").unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let year_part = caps.get(2)?.as_str().trim();
                let last_two = parse_tens_and_ones(year_part)?;
                if last_two <= 99 {
                    Some(format!("20{:02}", last_two))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "two thousand twenty" or "two thousand and twenty" → "2020"
        // Process simple numbers second (one word)
        (
            Regex::new(r"(?i)\b(two\s+thousand)(?:\s+and)?\s+(\w+)\b").unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let year_part = caps.get(2)?.as_str().trim();
                let last_two = word_to_number(year_part)?;
                if last_two <= 99 {
                    Some(format!("20{:02}", last_two))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "nineteen ninety-nine" or "nineteen ninety nine" → "1999"
        (
            Regex::new(r"(?i)\b(nineteen)[\s-]+(\w+[\s-]+\w+|\w+)\b").unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let second_part = caps.get(2)?.as_str();
                let last_two = if second_part.contains(' ') || second_part.contains('-') {
                    parse_tens_and_ones(second_part)?
                } else {
                    word_to_number(second_part)?
                };
                if last_two <= 99 {
                    Some(format!("19{:02}", last_two))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "eighteen eighty-five" → "1885"
        (
            Regex::new(r"(?i)\b(eighteen)[\s-]+(\w+[\s-]+\w+|\w+)\b").unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let second_part = caps.get(2)?.as_str();
                let last_two = if second_part.contains(' ') || second_part.contains('-') {
                    parse_tens_and_ones(second_part)?
                } else {
                    word_to_number(second_part)?
                };
                if last_two <= 99 {
                    Some(format!("18{:02}", last_two))
                } else {
                    None
                }
            }),
        ),
    ];

    let mut result = text.to_string();

    for (pattern, converter) in patterns {
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        for caps in pattern.captures_iter(&result) {
            if let Some(replacement) = converter(&caps) {
                if let Some(full_match) = caps.get(0) {
                    replacements.push((full_match.start(), full_match.end(), replacement));
                }
            }
        }

        // Apply replacements in reverse order to maintain correct indices
        for (start, end, replacement) in replacements.into_iter().rev() {
            result.replace_range(start..end, &replacement);
        }
    }

    result
}

/// Converts a word to its numeric value (0-99)
fn word_to_number(word: &str) -> Option<u32> {
    let word_lower = word.to_lowercase();
    match word_lower.as_str() {
        "zero" | "oh" => Some(0),
        "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        "thirty" => Some(30),
        "forty" => Some(40),
        "fifty" => Some(50),
        "sixty" => Some(60),
        "seventy" => Some(70),
        "eighty" => Some(80),
        "ninety" => Some(90),
        _ => None,
    }
}

/// Parses phrases like "twenty-five" or "twenty five" into numeric value
fn parse_tens_and_ones(phrase: &str) -> Option<u32> {
    let parts: Vec<&str> = phrase.split([' ', '-']).collect();

    if parts.len() == 1 {
        return word_to_number(parts[0]);
    }

    if parts.len() == 2 {
        let tens = word_to_number(parts[0])?;
        let ones = word_to_number(parts[1])?;
        if (20..=90).contains(&tens) && ones <= 9 {
            return Some(tens + ones);
        }
    }

    None
}

/// Converts a spoken number phrase to numeric value
/// Handles numbers from 0 to 9999
fn parse_spoken_number(text: &str) -> Option<u32> {
    let text = text.trim().to_lowercase();

    // First try to parse using parse_tens_and_ones which handles hyphens
    if let Some(result) = parse_tens_and_ones(&text) {
        return Some(result);
    }

    let parts: Vec<&str> = text.split_whitespace().collect();

    // Handle simple single-word numbers
    if parts.len() == 1 {
        return word_to_number(parts[0]);
    }

    // Handle compound numbers like "twenty five"
    if parts.len() == 2 {
        // Check for "X hundred" pattern
        if parts[1] == "hundred" {
            let hundreds = word_to_number(parts[0])?;
            if hundreds <= 9 {
                return Some(hundreds * 100);
            }
        }
        // Already tried parse_tens_and_ones above
    }

    // Handle patterns like "one hundred twenty" or "two hundred fifty"
    if parts.len() >= 3 && parts[1] == "hundred" {
        let hundreds = word_to_number(parts[0])?;
        if hundreds > 9 {
            return None;
        }
        let base = hundreds * 100;

        // Join remaining parts and parse as tens/ones
        let remainder = parts[2..].join(" ");
        if let Some(last_two) = parse_tens_and_ones(&remainder) {
            return Some(base + last_two);
        } else {
            // Maybe it's just "X hundred" with no remainder
            return Some(base);
        }
    }

    None
}

/// Normalizes spoken measurements into numeric format with abbreviated units
///
/// This function converts spoken measurements like "twenty five milligrams" or
/// "one hundred fifty pounds" into numeric format like "25 mg" or "150 lbs".
///
/// Supported units:
/// - Weight: milligrams (mg), grams (g), kilograms (kg), pounds (lbs), ounces (oz)
/// - Length: millimeters (mm), centimeters (cm), meters (m), kilometers (km), inches (in), feet (ft), yards (yd), miles (mi)
/// - Volume: milliliters (ml), liters (l), gallons (gal), quarts (qt), pints (pt), cups (c), tablespoons (tbsp), teaspoons (tsp)
///
/// # Arguments
/// * `text` - The input text to normalize
///
/// # Returns
/// The text with normalized measurement formats
pub fn normalize_measurements(text: &str) -> String {
    // Define unit mappings: (full_name, abbreviation)
    let units = vec![
        // Weight
        ("milligrams?", "mg"),
        ("grams?", "g"),
        ("kilograms?", "kg"),
        ("pounds?", "lbs"),
        ("ounces?", "oz"),
        // Length
        ("millimeters?", "mm"),
        ("centimeters?", "cm"),
        ("meters?", "m"),
        ("kilometers?", "km"),
        ("inches?", "in"),
        ("feet", "ft"),
        ("foot", "ft"),
        ("yards?", "yd"),
        ("miles?", "mi"),
        // Volume
        ("milliliters?", "ml"),
        ("liters?", "l"),
        ("gallons?", "gal"),
        ("quarts?", "qt"),
        ("pints?", "pt"),
        ("cups?", "c"),
        ("tablespoons?", "tbsp"),
        ("teaspoons?", "tsp"),
    ];

    let mut result = text.to_string();

    for (unit_pattern, unit_abbr) in units {
        // Pattern matches 1-4 words before the unit
        // Try matching from longest to shortest to capture compound numbers first
        let patterns_to_try = vec![
            // Four words: "one hundred fifty five"
            format!(r"(?i)\b(\w+\s+\w+\s+\w+\s+\w+)\s+({})\b", unit_pattern),
            // Three words: "one hundred fifty"
            format!(r"(?i)\b(\w+\s+\w+\s+\w+)\s+({})\b", unit_pattern),
            // Two words: "twenty five"
            format!(r"(?i)\b(\w+\s+\w+)\s+({})\b", unit_pattern),
            // One word: "five"
            format!(r"(?i)\b(\w+)\s+({})\b", unit_pattern),
        ];

        for pattern_str in patterns_to_try {
            if let Ok(pattern) = Regex::new(&pattern_str) {
                let mut replacements: Vec<(usize, usize, String)> = Vec::new();

                for caps in pattern.captures_iter(&result) {
                    if let Some(number_text) = caps.get(1) {
                        if let Some(number) = parse_spoken_number(number_text.as_str()) {
                            if let Some(full_match) = caps.get(0) {
                                let replacement = format!("{} {}", number, unit_abbr);
                                replacements.push((
                                    full_match.start(),
                                    full_match.end(),
                                    replacement,
                                ));
                            }
                        }
                    }
                }

                // Apply replacements in reverse order to maintain correct indices
                for (start, end, replacement) in replacements.into_iter().rev() {
                    result.replace_range(start..end, &replacement);
                }
            }
        }
    }

    result
}

/// Normalizes spoken time phrases into numeric format
///
/// This function converts spoken time formats into numeric format like "10:15".
///
/// Supported formats:
/// - "ten fifteen" → "10:15"
/// - "ten oh five" → "10:05"
/// - "ten o'clock" → "10:00"
/// - "three forty-five" → "3:45"
///
/// # Arguments
/// * `text` - The input text to normalize
///
/// # Returns
/// The text with normalized time formats
pub fn normalize_times(text: &str) -> String {
    type ConverterFn = Box<dyn Fn(&regex::Captures) -> Option<String>>;

    // Build pattern for hours (one through twelve)
    let hour_words = r"(?:one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)";

    let patterns: Vec<(Regex, ConverterFn)> = vec![
        // Pattern: "ten o'clock" → "10:00"
        (
            Regex::new(&format!(r"(?i)\b({})\s+o'?clock\b", hour_words)).unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let hour_word = caps.get(1)?.as_str();
                let hour = word_to_number(hour_word)?;
                if (1..=12).contains(&hour) {
                    Some(format!("{}:00", hour))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "ten oh five" → "10:05" (single digit minutes with "oh")
        (
            Regex::new(&format!(
                r"(?i)\b({})\s+oh\s+(one|two|three|four|five|six|seven|eight|nine)\b",
                hour_words
            ))
            .unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let hour_word = caps.get(1)?.as_str();
                let minute_word = caps.get(2)?.as_str();
                let hour = word_to_number(hour_word)?;
                let minute = word_to_number(minute_word)?;
                if (1..=12).contains(&hour) && (1..=9).contains(&minute) {
                    Some(format!("{}:{:02}", hour, minute))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "ten twenty-five" or "ten twenty five" → "10:25" (compound minutes)
        (
            Regex::new(&format!(
                r"(?i)\b({})[\s-]+(twenty|thirty|forty|fifty)[\s-]+(one|two|three|four|five|six|seven|eight|nine)\b",
                hour_words
            ))
            .unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let hour_word = caps.get(1)?.as_str();
                let tens_word = caps.get(2)?.as_str();
                let ones_word = caps.get(3)?.as_str();
                let hour = word_to_number(hour_word)?;
                let tens = word_to_number(tens_word)?;
                let ones = word_to_number(ones_word)?;
                let minutes = tens + ones;
                if (1..=12).contains(&hour) && (21..=59).contains(&minutes) {
                    Some(format!("{}:{:02}", hour, minutes))
                } else {
                    None
                }
            }),
        ),
        // Pattern: "ten fifteen" → "10:15" (simple minutes 10-59)
        (
            Regex::new(&format!(
                r"(?i)\b({})\s+(ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fifty)\b",
                hour_words
            ))
            .unwrap(),
            Box::new(|caps: &regex::Captures| -> Option<String> {
                let hour_word = caps.get(1)?.as_str();
                let minute_word = caps.get(2)?.as_str();
                let hour = word_to_number(hour_word)?;
                let minutes = word_to_number(minute_word)?;
                if (1..=12).contains(&hour) && (10..=59).contains(&minutes) {
                    Some(format!("{}:{:02}", hour, minutes))
                } else {
                    None
                }
            }),
        ),
    ];

    let mut result = text.to_string();

    for (pattern, converter) in patterns {
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        for caps in pattern.captures_iter(&result) {
            if let Some(replacement) = converter(&caps) {
                if let Some(full_match) = caps.get(0) {
                    replacements.push((full_match.start(), full_match.end(), replacement));
                }
            }
        }

        // Apply replacements in reverse order to maintain correct indices
        for (start, end, replacement) in replacements.into_iter().rev() {
            result.replace_range(start..end, &replacement);
        }
    }

    result
}

/// Applies custom word corrections to transcribed text using fuzzy matching
///
/// This function corrects words in the input text by finding the best matches
/// from a list of custom words using a combination of:
/// - Levenshtein distance for string similarity
/// - Soundex phonetic matching for pronunciation similarity
///
/// Uses adaptive algorithm selection:
/// - < 200 words: Length-based bucketing (Phase 2)
/// - >= 200 words: BK-tree indexing (Phase 3)
///
/// # Arguments
/// * `text` - The input text to correct
/// * `custom_words` - List of custom words to match against
/// * `threshold` - Maximum similarity score to accept (0.0 = exact match, 1.0 = any match)
///
/// # Returns
/// The corrected text with custom words applied
pub fn apply_custom_words(text: &str, custom_words: &[String], threshold: f64) -> String {
    if custom_words.is_empty() {
        return text.to_string();
    }

    // Adaptive strategy: choose algorithm based on vocabulary size
    if custom_words.len() >= BKTREE_THRESHOLD {
        apply_custom_words_bktree(text, custom_words, threshold)
    } else {
        apply_custom_words_bucketing(text, custom_words, threshold)
    }
}

/// Phase 3: BK-tree implementation for large vocabularies (200+ words)
fn apply_custom_words_bktree(text: &str, custom_words: &[String], threshold: f64) -> String {
    // Build BK-tree index using built-in Levenshtein metric
    let mut tree = BKTree::new(bk_tree::metrics::Levenshtein);

    let custom_words_lower: Vec<String> = custom_words.iter().map(|w| w.to_lowercase()).collect();

    for word in &custom_words_lower {
        tree.add(word.clone());
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut corrected_words = Vec::new();

    for word in words {
        let cleaned_word = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();

        if cleaned_word.is_empty() {
            corrected_words.push(word.to_string());
            continue;
        }

        if cleaned_word.len() > 50 {
            corrected_words.push(word.to_string());
            continue;
        }

        // Calculate max edit distance based on word length and threshold
        let max_word_len = cleaned_word.len();
        let max_distance = ((max_word_len as f64) * threshold * 1.5).ceil() as u32;

        // Find candidates within edit distance
        // BKTree::find returns Vec<(distance, &value)>
        let candidates = tree.find(&cleaned_word, max_distance);

        let mut best_match: Option<&String> = None;
        let mut best_score = f64::MAX;

        for (bk_distance, candidate) in candidates {
            // Find original word index
            if let Some(original_idx) = custom_words_lower.iter().position(|w| w == candidate) {
                // Use the BK-tree distance as Levenshtein distance
                let levenshtein_dist = bk_distance as usize;

                // Early exit for exact match
                if levenshtein_dist == 0 {
                    best_match = Some(&custom_words[original_idx]);
                    break;
                }

                let max_len = cleaned_word.len().max(candidate.len()) as f64;
                let levenshtein_score = if max_len > 0.0 {
                    levenshtein_dist as f64 / max_len
                } else {
                    1.0
                };

                if levenshtein_score > threshold {
                    continue;
                }

                let phonetic_match = soundex(&cleaned_word, candidate);
                let combined_score = if phonetic_match {
                    levenshtein_score * 0.3
                } else {
                    levenshtein_score
                };

                if combined_score < threshold && combined_score < best_score {
                    best_match = Some(&custom_words[original_idx]);
                    best_score = combined_score;
                }
            }
        }

        if let Some(replacement) = best_match {
            let corrected = preserve_case_pattern(word, replacement);
            let (prefix, suffix) = extract_punctuation(word);
            corrected_words.push(format!("{}{}{}", prefix, corrected, suffix));
        } else {
            corrected_words.push(word.to_string());
        }
    }

    corrected_words.join(" ")
}

/// Phase 2: Length-based bucketing for small-medium vocabularies (< 200 words)
fn apply_custom_words_bucketing(text: &str, custom_words: &[String], threshold: f64) -> String {
    // Build length-based buckets for fast lookup
    let mut length_buckets: HashMap<usize, Vec<(usize, String)>> = HashMap::new();

    for (i, word) in custom_words.iter().enumerate() {
        let word_lower = word.to_lowercase();
        let len = word_lower.len();
        length_buckets.entry(len).or_default().push((i, word_lower));
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut corrected_words = Vec::new();

    for word in words {
        let cleaned_word = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();

        if cleaned_word.is_empty() {
            corrected_words.push(word.to_string());
            continue;
        }

        // Skip extremely long words to avoid performance issues
        if cleaned_word.len() > 50 {
            corrected_words.push(word.to_string());
            continue;
        }

        let mut best_match: Option<&String> = None;
        let mut best_score = f64::MAX;

        // Phase 2: Only search words within ±5 length range
        let target_len = cleaned_word.len();
        let min_len = target_len.saturating_sub(5);
        let max_len = target_len + 5;

        for bucket_len in min_len..=max_len {
            if let Some(bucket) = length_buckets.get(&bucket_len) {
                for (original_idx, custom_word_lower) in bucket {
                    // Calculate Levenshtein distance (normalized by length)
                    let levenshtein_dist = levenshtein(&cleaned_word, custom_word_lower);
                    let max_len = cleaned_word.len().max(custom_word_lower.len()) as f64;
                    let levenshtein_score = if max_len > 0.0 {
                        levenshtein_dist as f64 / max_len
                    } else {
                        1.0
                    };

                    // Optimization: Early exit for exact matches
                    if levenshtein_dist == 0 {
                        best_match = Some(&custom_words[*original_idx]);
                        best_score = 0.0;
                        break; // Found exact match, stop searching this bucket
                    }

                    // Optimization: Skip expensive phonetic check if Levenshtein already too high
                    if levenshtein_score > threshold {
                        continue;
                    }

                    // Calculate phonetic similarity using Soundex
                    let phonetic_match = soundex(&cleaned_word, custom_word_lower);

                    // Combine scores: favor phonetic matches, but also consider string similarity
                    let combined_score = if phonetic_match {
                        levenshtein_score * 0.3 // Give significant boost to phonetic matches
                    } else {
                        levenshtein_score
                    };

                    // Accept if the score is good enough (configurable threshold)
                    if combined_score < threshold && combined_score < best_score {
                        best_match = Some(&custom_words[*original_idx]);
                        best_score = combined_score;
                    }
                }

                // If we found an exact match, no need to check other length buckets
                if best_score == 0.0 {
                    break;
                }
            }
        }

        if let Some(replacement) = best_match {
            // Preserve the original case pattern as much as possible
            let corrected = preserve_case_pattern(word, replacement);

            // Preserve punctuation from original word
            let (prefix, suffix) = extract_punctuation(word);
            corrected_words.push(format!("{}{}{}", prefix, corrected, suffix));
        } else {
            corrected_words.push(word.to_string());
        }
    }

    corrected_words.join(" ")
}

/// Preserves the case pattern of the original word when applying a replacement
fn preserve_case_pattern(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars: Vec<char> = replacement.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect()
    } else {
        replacement.to_string()
    }
}

/// Extracts punctuation prefix and suffix from a word
fn extract_punctuation(word: &str) -> (&str, &str) {
    let prefix_end = word.chars().take_while(|c| !c.is_alphabetic()).count();
    let suffix_start = word
        .char_indices()
        .rev()
        .take_while(|(_, c)| !c.is_alphabetic())
        .count();

    let prefix = if prefix_end > 0 {
        &word[..prefix_end]
    } else {
        ""
    };

    let suffix = if suffix_start > 0 {
        &word[word.len() - suffix_start..]
    } else {
        ""
    };

    (prefix, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_years_twenty_twenty_format() {
        assert_eq!(
            normalize_years("The meeting is in twenty twenty-five"),
            "The meeting is in 2025"
        );
        assert_eq!(
            normalize_years("In twenty twenty five we will meet"),
            "In 2025 we will meet"
        );
        assert_eq!(normalize_years("Back in twenty twenty-one"), "Back in 2021");
    }

    #[test]
    fn test_normalize_years_two_thousand_format() {
        assert_eq!(
            normalize_years("The year two thousand twenty-five"),
            "The year 2025"
        );
        assert_eq!(
            normalize_years("In two thousand and twenty-five"),
            "In 2025"
        );
        assert_eq!(
            normalize_years("The year two thousand twenty five"),
            "The year 2025"
        );
        assert_eq!(normalize_years("two thousand twenty"), "2020");
    }

    #[test]
    fn test_normalize_years_nineteen_format() {
        assert_eq!(
            normalize_years("Back in nineteen ninety-nine"),
            "Back in 1999"
        );
        assert_eq!(normalize_years("In nineteen ninety nine"), "In 1999");
        assert_eq!(
            normalize_years("The year nineteen eighty-five"),
            "The year 1985"
        );
    }

    #[test]
    fn test_normalize_years_eighteen_format() {
        assert_eq!(normalize_years("In eighteen eighty-five"), "In 1885");
        assert_eq!(normalize_years("Back in eighteen seventy"), "Back in 1870");
    }

    #[test]
    fn test_normalize_years_mixed_text() {
        assert_eq!(
            normalize_years("From nineteen ninety-nine to twenty twenty-five"),
            "From 1999 to 2025"
        );
        assert_eq!(
            normalize_years("Between two thousand twenty and twenty twenty-five"),
            "Between 2020 and 2025"
        );
    }

    #[test]
    fn test_normalize_years_no_match() {
        let text = "Hello world with no years";
        assert_eq!(normalize_years(text), text);
    }

    #[test]
    fn test_normalize_years_case_insensitive() {
        assert_eq!(normalize_years("In TWENTY TWENTY-FIVE"), "In 2025");
        assert_eq!(normalize_years("In Two Thousand Twenty-Five"), "In 2025");
    }

    #[test]
    fn test_word_to_number() {
        assert_eq!(word_to_number("twenty"), Some(20));
        assert_eq!(word_to_number("five"), Some(5));
        assert_eq!(word_to_number("ninety"), Some(90));
        assert_eq!(word_to_number("invalid"), None);
    }

    #[test]
    fn test_parse_tens_and_ones() {
        assert_eq!(parse_tens_and_ones("twenty-five"), Some(25));
        assert_eq!(parse_tens_and_ones("twenty five"), Some(25));
        assert_eq!(parse_tens_and_ones("ninety-nine"), Some(99));
        assert_eq!(parse_tens_and_ones("forty-two"), Some(42));
        assert_eq!(parse_tens_and_ones("twenty"), Some(20));
        assert_eq!(parse_tens_and_ones("invalid-value"), None);
    }

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec!["hello".to_string(), "world".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_preserve_case_pattern() {
        assert_eq!(preserve_case_pattern("HELLO", "world"), "WORLD");
        assert_eq!(preserve_case_pattern("Hello", "world"), "World");
        assert_eq!(preserve_case_pattern("hello", "WORLD"), "WORLD");
    }

    #[test]
    fn test_extract_punctuation() {
        assert_eq!(extract_punctuation("hello"), ("", ""));
        assert_eq!(extract_punctuation("!hello?"), ("!", "?"));
        assert_eq!(extract_punctuation("...hello..."), ("...", "..."));
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_normalize_measurements_weight() {
        assert_eq!(
            normalize_measurements("Take twenty five milligrams daily"),
            "Take 25 mg daily"
        );
        assert_eq!(
            normalize_measurements("She weighs fifty kilograms"),
            "She weighs 50 kg"
        );
        assert_eq!(
            normalize_measurements("Add ten grams of sugar"),
            "Add 10 g of sugar"
        );
        assert_eq!(normalize_measurements("Lost fifteen pounds"), "Lost 15 lbs");
    }

    #[test]
    fn test_normalize_measurements_length() {
        assert_eq!(
            normalize_measurements("He is six feet tall"),
            "He is 6 ft tall"
        );
        assert_eq!(
            normalize_measurements("The room is ten meters wide"),
            "The room is 10 m wide"
        );
        assert_eq!(
            normalize_measurements("Cut fifty centimeters of rope"),
            "Cut 50 cm of rope"
        );
        assert_eq!(normalize_measurements("Drive twenty miles"), "Drive 20 mi");
    }

    #[test]
    fn test_normalize_measurements_volume() {
        assert_eq!(
            normalize_measurements("Pour five hundred milliliters"),
            "Pour 500 ml"
        );
        assert_eq!(
            normalize_measurements("Add two liters of water"),
            "Add 2 l of water"
        );
        assert_eq!(
            normalize_measurements("One tablespoon of oil"),
            "1 tbsp of oil"
        );
    }

    #[test]
    fn test_normalize_measurements_compound_numbers() {
        assert_eq!(
            normalize_measurements("Take ninety nine milligrams"),
            "Take 99 mg"
        );
        assert_eq!(
            normalize_measurements("One hundred fifty pounds"),
            "150 lbs"
        );
    }

    #[test]
    fn test_normalize_measurements_singular_plural() {
        assert_eq!(
            normalize_measurements("One kilogram of flour"),
            "1 kg of flour"
        );
        assert_eq!(
            normalize_measurements("Two kilograms of flour"),
            "2 kg of flour"
        );
    }

    #[test]
    fn test_normalize_measurements_mixed_text() {
        assert_eq!(
            normalize_measurements("Take twenty five milligrams and walk five miles"),
            "Take 25 mg and walk 5 mi"
        );
    }

    #[test]
    fn test_normalize_measurements_no_match() {
        let text = "Hello world with no measurements";
        assert_eq!(normalize_measurements(text), text);
    }

    #[test]
    fn test_parse_spoken_number() {
        assert_eq!(parse_spoken_number("five"), Some(5));
        assert_eq!(parse_spoken_number("twenty five"), Some(25));
        assert_eq!(parse_spoken_number("ninety nine"), Some(99));
        assert_eq!(parse_spoken_number("one hundred"), Some(100));
        assert_eq!(parse_spoken_number("five hundred"), Some(500));
        assert_eq!(parse_spoken_number("one hundred fifty"), Some(150));
    }

    #[test]
    fn test_normalize_times_oclock() {
        assert_eq!(normalize_times("at ten o'clock"), "at 10:00");
        assert_eq!(normalize_times("three oclock"), "3:00");
        assert_eq!(normalize_times("at twelve o'clock"), "at 12:00");
    }

    #[test]
    fn test_normalize_times_oh_minutes() {
        assert_eq!(normalize_times("ten oh five"), "10:05");
        assert_eq!(normalize_times("at three oh nine"), "at 3:09");
        assert_eq!(normalize_times("eleven oh one"), "11:01");
    }

    #[test]
    fn test_normalize_times_simple_minutes() {
        assert_eq!(normalize_times("ten fifteen"), "10:15");
        assert_eq!(normalize_times("three twenty"), "3:20");
        assert_eq!(normalize_times("twelve fifty"), "12:50");
        assert_eq!(normalize_times("at five ten"), "at 5:10");
    }

    #[test]
    fn test_normalize_times_compound_minutes() {
        assert_eq!(normalize_times("ten twenty-five"), "10:25");
        assert_eq!(normalize_times("three forty five"), "3:45");
        assert_eq!(normalize_times("at seven thirty-nine"), "at 7:39");
    }

    #[test]
    fn test_normalize_times_mixed_text() {
        assert_eq!(
            normalize_times("The meeting is at ten fifteen"),
            "The meeting is at 10:15"
        );
        assert_eq!(
            normalize_times("Call me at three o'clock or four thirty"),
            "Call me at 3:00 or 4:30"
        );
    }

    #[test]
    fn test_normalize_times_no_match() {
        let text = "Hello world with no times";
        assert_eq!(normalize_times(text), text);
    }

    #[test]
    fn test_normalize_times_case_insensitive() {
        assert_eq!(normalize_times("TEN FIFTEEN"), "10:15");
        assert_eq!(normalize_times("Three O'Clock"), "3:00");
    }
}
