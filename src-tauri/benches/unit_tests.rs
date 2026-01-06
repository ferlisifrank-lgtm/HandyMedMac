use handy_app_lib::audio_toolkit::text::apply_custom_words;

fn main() {
    println!("=== Unit Tests for Custom Word Matching ===\n");

    let mut passed = 0;
    let mut failed = 0;

    // Test 1: Exact match
    {
        let text = "hello world";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "Hello World";

        if result == expected {
            println!("✓ Test 1 PASSED: Exact match");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            passed += 1;
        } else {
            println!("✗ Test 1 FAILED: Exact match");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 2: Fuzzy match
    {
        let text = "helo wrold";
        let custom_words = vec!["hello".to_string(), "world".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "hello world";

        if result == expected {
            println!("✓ Test 2 PASSED: Fuzzy match");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            passed += 1;
        } else {
            println!("✗ Test 2 FAILED: Fuzzy match");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 3: Empty custom words
    {
        let text = "hello world";
        let custom_words = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "hello world";

        if result == expected {
            println!("✓ Test 3 PASSED: Empty custom words");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            passed += 1;
        } else {
            println!("✗ Test 3 FAILED: Empty custom words");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 4: Punctuation preservation
    {
        let text = "!hello? world.";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "!Hello? World.";

        if result == expected {
            println!("✓ Test 4 PASSED: Punctuation preservation");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            passed += 1;
        } else {
            println!("✗ Test 4 FAILED: Punctuation preservation");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 5: Case preservation - all caps
    {
        let text = "HELLO";
        let custom_words = vec!["hello".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "HELLO";

        if result == expected {
            println!("✓ Test 5 PASSED: Case preservation (all caps)");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            passed += 1;
        } else {
            println!("✗ Test 5 FAILED: Case preservation (all caps)");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 6: Case preservation - title case
    {
        let text = "Hello";
        let custom_words = vec!["hello".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "Hello";

        if result == expected {
            println!("✓ Test 6 PASSED: Case preservation (title case)");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            passed += 1;
        } else {
            println!("✗ Test 6 FAILED: Case preservation (title case)");
            println!("  Input:    '{}'", text);
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 7: Phase 2 with small vocab
    {
        let text = "lipitor metformin";
        let custom_words = vec!["Lipitor".to_string(), "metformin".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        let expected = "Lipitor metformin";

        if result == expected {
            println!("✓ Test 7 PASSED: Phase 2 (small vocab)");
            println!("  Vocab size: {}", custom_words.len());
            println!("  Algorithm: Phase 2 (bucketing)");
            passed += 1;
        } else {
            println!("✗ Test 7 FAILED: Phase 2 (small vocab)");
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Test 8: Phase 3 with large vocab
    {
        let text = "lipitor metformin";
        let mut large_vocab: Vec<String> = Vec::new();
        large_vocab.push("Lipitor".to_string());
        large_vocab.push("metformin".to_string());
        for i in 0..300 {
            large_vocab.push(format!("medication{:03}", i));
        }

        let result = apply_custom_words(text, &large_vocab, 0.5);
        let expected = "Lipitor metformin";

        if result == expected {
            println!("✓ Test 8 PASSED: Phase 3 (large vocab)");
            println!("  Vocab size: {}", large_vocab.len());
            println!("  Algorithm: Phase 3 (BK-tree)");
            passed += 1;
        } else {
            println!("✗ Test 8 FAILED: Phase 3 (large vocab)");
            println!("  Output:   '{}'", result);
            println!("  Expected: '{}'", expected);
            failed += 1;
        }
        println!();
    }

    // Summary
    println!("{}", "=".repeat(50));
    println!("Test Summary:");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);
    println!("  Total:  {}", passed + failed);

    if failed == 0 {
        println!("\n✓ All tests passed!");
    } else {
        println!("\n✗ Some tests failed!");
        std::process::exit(1);
    }
}
