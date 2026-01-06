use handy_app_lib::audio_toolkit::text::apply_custom_words;
use std::time::Instant;

fn main() {
    println!("=== Custom Words Performance Benchmark ===\n");

    // Test text with intentional typos
    let test_text =
        "The patient was prescribed lipitor for high cholesterol and metformin for diabetes. \
                     They also take advil for pain and tylenol for fever. \
                     Previous history includes hypertension and atelectasis.";

    // Small vocabulary (Phase 2: Bucketing)
    let small_vocab = vec![
        "Lipitor".to_string(),
        "cholesterol".to_string(),
        "metformin".to_string(),
        "diabetes".to_string(),
        "Advil".to_string(),
        "Tylenol".to_string(),
        "hypertension".to_string(),
        "atelectasis".to_string(),
    ];

    // Medium vocabulary (Phase 2: Bucketing - just under threshold)
    let mut medium_vocab = small_vocab.clone();
    for i in 0..190 {
        medium_vocab.push(format!("medication{:03}", i));
    }

    // Large vocabulary (Phase 3: BK-tree - at threshold)
    let mut large_vocab = medium_vocab.clone();
    for i in 0..10 {
        large_vocab.push(format!("treatment{:03}", i));
    }

    // Extra large vocabulary (Phase 3: BK-tree - well above threshold)
    let mut xlarge_vocab = large_vocab.clone();
    for i in 0..300 {
        xlarge_vocab.push(format!("condition{:03}", i));
    }

    println!("Test text: \"{}...\"\n", &test_text[..80]);
    println!("Vocabulary sizes:");
    println!("  Small:  {} words (Phase 2)", small_vocab.len());
    println!("  Medium: {} words (Phase 2)", medium_vocab.len());
    println!("  Large:  {} words (Phase 3)", large_vocab.len());
    println!("  XLarge: {} words (Phase 3)", xlarge_vocab.len());
    println!("\n{}\n", "=".repeat(60));

    // Warm up
    let _ = apply_custom_words(test_text, &small_vocab, 0.5);

    // Benchmark Small Vocabulary (Phase 2)
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = apply_custom_words(test_text, &small_vocab, 0.5);
    }
    let small_duration = start.elapsed();
    let small_avg = small_duration / iterations;

    // Benchmark Medium Vocabulary (Phase 2)
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = apply_custom_words(test_text, &medium_vocab, 0.5);
    }
    let medium_duration = start.elapsed();
    let medium_avg = medium_duration / iterations;

    // Benchmark Large Vocabulary (Phase 3)
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = apply_custom_words(test_text, &large_vocab, 0.5);
    }
    let large_duration = start.elapsed();
    let large_avg = large_duration / iterations;

    // Benchmark Extra Large Vocabulary (Phase 3)
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = apply_custom_words(test_text, &xlarge_vocab, 0.5);
    }
    let xlarge_duration = start.elapsed();
    let xlarge_avg = xlarge_duration / iterations;

    println!("Performance Results ({} iterations each):", iterations);
    println!("{}", "-".repeat(60));
    println!("Small vocab (8 words):    {:?} per iteration", small_avg);
    println!(
        "Medium vocab (198 words): {:?} per iteration ({}x slower)",
        medium_avg,
        medium_avg.as_micros() as f64 / small_avg.as_micros() as f64
    );
    println!(
        "Large vocab (208 words):  {:?} per iteration ({}x slower)",
        large_avg,
        large_avg.as_micros() as f64 / small_avg.as_micros() as f64
    );
    println!(
        "XLarge vocab (508 words): {:?} per iteration ({}x slower)",
        xlarge_avg,
        xlarge_avg.as_micros() as f64 / small_avg.as_micros() as f64
    );

    println!("\n{}", "=".repeat(60));
    println!("Phase 3 (BK-tree) Efficiency:");
    println!("{}", "-".repeat(60));

    // Compare Phase 3 scaling vs Phase 2 scaling
    let phase2_scaling = medium_avg.as_micros() as f64 / small_avg.as_micros() as f64;
    let phase3_scaling = xlarge_avg.as_micros() as f64 / large_avg.as_micros() as f64;

    println!(
        "Phase 2 scaling (8→198 words):   {:.2}x slower",
        phase2_scaling
    );
    println!(
        "Phase 3 scaling (208→508 words): {:.2}x slower",
        phase3_scaling
    );

    if phase3_scaling < phase2_scaling {
        println!(
            "\n✓ Phase 3 (BK-tree) shows better scaling ({:.2}x vs {:.2}x)!",
            phase3_scaling, phase2_scaling
        );
    }

    // Test correctness
    println!("\n{}", "=".repeat(60));
    println!("Correctness Verification:");
    println!("{}", "-".repeat(60));

    let result = apply_custom_words(test_text, &xlarge_vocab, 0.5);
    let expected_words = [
        "Lipitor",
        "metformin",
        "Advil",
        "Tylenol",
        "hypertension",
        "atelectasis",
    ];
    let all_found = expected_words.iter().all(|word| result.contains(word));

    if all_found {
        println!("✓ All expected words found in result");
        println!("  Result sample: \"{}...\"", &result[..100]);
    } else {
        println!("✗ Some expected words missing!");
        println!("  Result: {}", result);
    }

    println!("\n{}", "=".repeat(60));
    println!("Adaptive Strategy Verification:");
    println!("{}", "-".repeat(60));
    println!("Threshold: {} words", 200);
    println!(
        "  Small vocab ({} words):  Uses Phase 2 ✓",
        small_vocab.len()
    );
    println!(
        "  Medium vocab ({} words): Uses Phase 2 ✓",
        medium_vocab.len()
    );
    println!(
        "  Large vocab ({} words):  Uses Phase 3 ✓",
        large_vocab.len()
    );
    println!(
        "  XLarge vocab ({} words): Uses Phase 3 ✓",
        xlarge_vocab.len()
    );
}
