# Medical Vocabulary Expansion Design

**Date:** 2026-01-17
**Goal:** Expand medical vocabulary from ~560 to 15,000+ terms to rival Dragon Dictate accuracy without compromising speed.

## Summary

- **Scope:** General practice + common specialty terms
- **Priority:** Medications and anatomical/medical terms first, abbreviations second
- **Architecture:** Hybrid BK-tree with phonetic matching (integrate existing `text.rs` infrastructure)
- **Source:** Public medical databases (RxNorm, SNOMED-CT, ICD-10, LOINC, FMA) following Dragon Medical's approach
- **Format:** Binary/indexed for defaults (fast loading), users add custom terms via existing text file

## Vocabulary Breakdown (15,000+ terms)

| Category | Count | Source |
|----------|-------|--------|
| Medications (generic) | 4,000 | RxNorm top prescribed + common |
| Medications (brand) | 2,000 | RxNorm brand names (US + Canada) |
| Conditions/diagnoses | 3,500 | SNOMED-CT common clinical findings |
| Anatomy | 1,500 | Foundational Model of Anatomy |
| Procedures | 1,500 | CPT/SNOMED procedure terms |
| Lab tests | 800 | LOINC common panels |
| Medical eponyms | 500 | Curated list |
| Abbreviation expansions | 400 | Common spoken abbreviations |
| Phonetic corrections | 800+ | Mapped mishearings |

## Architecture

### Current Flow
```
Transcription → MedicalVocabulary::process_text() → Exact string matching → Output
```

### New Flow
```
Transcription → MedicalVocabulary::process_text()
                     ↓
              BKTree fuzzy lookup (O(log n))
                     ↓
              Phonetic scoring (Soundex/Metaphone)
                     ↓
              Candidate ranking → Best match → Output
```

### Key Components

#### 1. Binary Vocabulary Index (`medical_vocab.bin`)
- Pre-compiled at build time from source files
- Contains:
  - BK-tree structure for fuzzy lookup
  - Phonetic hashes (Metaphone) for each term
  - Category tags (medication, anatomy, condition, etc.)
  - Correction mappings (wrong → right)
- Loaded once at app startup via memory-mapped file (~1-2MB)

#### 2. Integration with Existing `text.rs` BK-tree
- Reuse `BKTree` struct and `levenshtein_distance()`
- Add `metaphone_hash()` for phonetic matching
- New scoring: `combined_score = 0.3 * edit_distance + 0.7 * phonetic_match`

#### 3. Lookup Strategy
```rust
fn find_correction(word: &str, vocab: &MedicalIndex) -> Option<String> {
    // 1. Exact match first (fastest path)
    if let Some(correction) = vocab.exact.get(&word.to_lowercase()) {
        return Some(correction.clone());
    }

    // 2. BK-tree fuzzy search (max edit distance 2)
    let candidates = vocab.bktree.find(word, 2);

    // 3. Score candidates by phonetic similarity
    let best = candidates
        .iter()
        .map(|c| (c, phonetic_score(word, c)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // 4. Return if score > 0.7 threshold
    best.filter(|(_, score)| *score > 0.7)
        .map(|(term, _)| vocab.corrections.get(*term).cloned().unwrap_or(term.to_string()))
}
```

#### 4. User Custom Vocab Unchanged
- Users still edit `custom_medical_vocab.txt`
- Custom terms loaded on top of binary index
- Custom terms get priority over defaults

## File Structure

```
src-tauri/
├── resources/
│   ├── medical_vocab.bin               # Pre-compiled binary index
│   └── vocab_sources/                  # Source data for compilation
│       ├── medications_generic.txt     # 4,000 generic drugs
│       ├── medications_brand.txt       # 2,000 brand names
│       ├── conditions.txt              # 3,500 conditions/diagnoses
│       ├── anatomy.txt                 # 1,500 anatomical terms
│       ├── procedures.txt              # 1,500 procedures
│       ├── lab_tests.txt               # 800 lab tests
│       ├── eponyms.txt                 # 500 medical eponyms
│       ├── abbreviations.txt           # 400 abbreviation expansions
│       └── phonetic_corrections.txt    # 800+ mishearing mappings
│
├── src/
│   ├── medical_vocab.rs                # MODIFY: Add binary index loading
│   ├── medical_index.rs                # NEW: BK-tree index structure
│   └── audio_toolkit/
│       └── text.rs                     # REUSE: BK-tree, phonetic functions
│
└── build.rs                            # MODIFY: Compile vocab at build time
```

**Note:** Delete existing `default_custom_vocab.txt` - those ~560 terms will be migrated into the new source files.

## Build Process

```rust
// build.rs addition
fn compile_medical_vocab() {
    let sources = [
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

    let mut terms = HashSet::new();
    let mut corrections = HashMap::new();

    for source in sources {
        for line in read_lines(source) {
            if line.contains(" -> ") {
                // Correction mapping
                let parts: Vec<&str> = line.split(" -> ").collect();
                corrections.insert(parts[0].to_string(), parts[1].to_string());
            } else {
                // Plain term
                terms.insert(line);
            }
        }
    }

    let index = MedicalIndex::build(terms, corrections);
    index.serialize_to("resources/medical_vocab.bin");
}
```

## Runtime Loading

```rust
// medical_vocab.rs
impl MedicalVocabulary {
    pub fn new() -> Self {
        // 1. Load binary index (memory-mapped, ~5ms)
        let index = MedicalIndex::load("medical_vocab.bin");

        // 2. Load user's custom file (if exists)
        let user_custom = load_user_custom_vocab();

        Self { index, user_custom }
    }
}
```

## Performance Expectations

- Binary index load: ~5ms (memory-mapped)
- Exact match lookup: O(1)
- Fuzzy lookup: O(log n) with BK-tree, ~0.1ms per word
- 100-word transcription: <15ms total processing

## Vocabulary Sources

### Medications (6,000 terms)
- **RxNorm** via NIH API - top prescribed drugs
- **FDA Orange Book** - approved drug products
- **Health Canada Drug Product Database** - Canadian brand names
- Format: `generic_name` + `brand_name -> generic_name` corrections

### Conditions/Diagnoses (3,500 terms)
- **SNOMED-CT** common clinical findings subset
- **ICD-10-CM** frequently used diagnosis codes with descriptions
- Focus on: primary care encounters, chronic disease, common acute conditions

### Anatomy (1,500 terms)
- **Foundational Model of Anatomy (FMA)** - standardized anatomical terms
- All body systems: musculoskeletal, cardiovascular, neurological, GI, respiratory, etc.

### Procedures (1,500 terms)
- **CPT codes** common procedure descriptions
- **SNOMED-CT** procedure subset
- Office procedures, imaging, surgical terms

### Lab Tests (800 terms)
- **LOINC** common lab panels and individual tests
- Include spoken forms: "hemoglobin A1C", "basic metabolic panel", etc.

### Phonetic Corrections (800+ mappings)
- Manually curated from common Whisper/speech recognition errors
- Format: `misheard -> correct` (e.g., `ace inhibitor -> ACE inhibitor`)
- Sourced from: medical transcription forums, Dragon user guides, common patterns

### Eponyms (500 terms)
- Named diseases: Alzheimer's, Parkinson's, Crohn's, Hashimoto's
- Named signs/tests: Babinski, Romberg, Murphy's sign
- Named procedures: Whipple, Nissen, McBurney's point

## Implementation Phases

### Phase 1: Infrastructure
1. Create `medical_index.rs` with BK-tree index structure
2. Add Metaphone phonetic hashing (reuse/extend `text.rs`)
3. Implement binary serialization/deserialization
4. Update `build.rs` to compile vocabulary at build time

### Phase 2: Vocabulary Collection
1. Create `vocab_sources/` directory structure
2. Gather terms from RxNorm, SNOMED-CT, ICD-10, LOINC, FMA
3. Migrate existing 560 terms into appropriate source files
4. Create phonetic corrections mappings
5. Deduplicate across all sources
6. Delete `default_custom_vocab.txt`

### Phase 3: Integration
1. Modify `medical_vocab.rs` to load binary index
2. Replace exact-match with hybrid lookup (exact → BK-tree → phonetic)
3. Keep user custom file loading (priority over defaults)
4. Update tests

### Phase 4: Validation
1. Test with real dictation samples
2. Benchmark performance (target: <15ms for 100 words)
3. Tune fuzzy match thresholds
4. Verify no regressions in existing corrections

## Migration Notes

- Existing `default_custom_vocab.txt` (~560 terms) will be categorized and migrated into appropriate source files
- `default_custom_vocab.txt` will be deleted after migration
- User's `custom_medical_vocab.txt` remains unchanged and continues to work
- User custom terms always take priority over binary index
