use atuin_bar_lib::{atuin_search, SearchFilters};

// Helper function to parse atuin output
// Format: {command}|{exit}|{directory}|{time}
// Note: command can contain '|' characters, so we split from the right
fn parse_atuin_line(line: &str) -> Option<(String, String, String, String)> {
    let parts: Vec<&str> = line.rsplitn(4, '|').collect();
    if parts.len() == 4 {
        // rsplitn gives us parts in reverse order: [time, directory, exit, command]
        Some((
            parts[3].to_string(), // command
            parts[2].to_string(), // exit
            parts[1].to_string(), // directory
            parts[0].to_string(), // time
        ))
    } else {
        None
    }
}

#[test]
fn test_atuin_search_e2e() {
    // This is a real end-to-end test that calls the actual atuin command
    let result = atuin_search("ls", None);

    // Check if atuin is installed
    match result {
        Ok(output) => {
            // Atuin is installed and returned results
            println!("Atuin search returned {} lines", output.lines().count());

            // Verify the output format matches our expected pattern
            // Format should be: {command}|{exit}|{directory}|{time}
            // Note: Some commands are multi-line (ending with \) and won't parse
            if !output.is_empty() {
                let mut parsed_count = 0;
                let mut unparsed_count = 0;

                for line in output.lines() {
                    match parse_atuin_line(line) {
                        Some((command, exit, _directory, time)) => {
                            // Verify each field is present
                            assert!(!command.is_empty(), "Command should not be empty");
                            assert!(!exit.is_empty(), "Exit code should not be empty");
                            assert!(!time.is_empty(), "Time should not be empty");
                            parsed_count += 1;
                        }
                        None => {
                            // This is OK - likely a multi-line command continuation
                            unparsed_count += 1;
                        }
                    }
                }

                println!("Parsed: {}, Unparsed (likely continuations): {}", parsed_count, unparsed_count);
                // At least some lines should parse successfully
                assert!(parsed_count > 0, "Should be able to parse at least some lines");
            }
        }
        Err(e) => {
            // Atuin might not be installed or query failed
            println!("Atuin search failed (this is OK if atuin is not installed): {}", e);

            // If the error is about atuin not being found, that's acceptable
            // We just want to make sure our error handling works
            assert!(
                e.contains("Failed to execute atuin command") ||
                e.contains("atuin command failed"),
                "Error should be a known atuin error type, got: {}",
                e
            );
        }
    }
}

#[test]
fn test_atuin_search_empty_query() {
    // Test with empty query - should still work if atuin is installed
    let result = atuin_search("", None);

    match result {
        Ok(_) => {
            // Empty query succeeded (atuin is installed)
            // This is acceptable behavior
        }
        Err(e) => {
            // Atuin not installed or failed
            assert!(
                e.contains("Failed to execute atuin command") ||
                e.contains("atuin command failed"),
                "Error should be a known atuin error type, got: {}",
                e
            );
        }
    }
}

#[test]
fn test_atuin_search_special_characters() {
    // Test with special characters that might cause issues
    let queries = vec![
        "git commit",
        "cd ..",
        "echo 'hello world'",
    ];

    for query in queries {
        let result = atuin_search(query, None);

        // We don't care if it finds results or not, just that it doesn't crash
        match result {
            Ok(output) => {
                // Verify format if we got output (skip unparseable multi-line commands)
                if !output.is_empty() {
                    let parseable = output.lines().filter(|line| parse_atuin_line(line).is_some()).count();
                    // At least some lines should be parseable
                    assert!(parseable > 0 || output.lines().count() == 0,
                        "Should be able to parse at least some output lines");
                }
            }
            Err(e) => {
                // Make sure error is expected type
                assert!(
                    e.contains("Failed to execute atuin command") ||
                    e.contains("atuin command failed"),
                    "Error should be a known atuin error type, got: {}",
                    e
                );
            }
        }
    }
}

#[test]
fn test_atuin_search_output_format() {
    // Test that the output format is correct when atuin is available
    let result = atuin_search("cargo", None);

    if let Ok(output) = result {
        if !output.is_empty() {
            // Find the first parseable line (skip multi-line command continuations)
            let first_parseable = output
                .lines()
                .find_map(|line| parse_atuin_line(line).map(|parsed| (line, parsed)));

            if let Some((_line, (command, exit, directory, time))) = first_parseable {
                // Command should not be empty
                assert!(!command.is_empty(), "Command should not be empty");

                // Exit code should be a number (or -1 for unknown)
                let _exit_code: i32 = exit.parse().expect("Exit code should be a number");

                // Directory should be a valid path (contains /) or "unknown" or empty
                assert!(
                    directory.contains('/') || directory == "unknown" || directory.is_empty(),
                    "Directory should be a path or 'unknown', got: {}",
                    directory
                );

                // Time should not be empty
                assert!(!time.is_empty(), "Timestamp should not be empty");
            }
            // If no lines are parseable, that's OK - might be all multi-line commands
        }
    }
}

#[test]
fn test_atuin_search_with_filters() {
    // Test with various filter combinations
    let filters = SearchFilters {
        directory: Some("/tmp".to_string()),
        exit_filter: Some("success".to_string()),
        time_range: Some("7d".to_string()),
    };

    let result = atuin_search("", Some(filters));

    match result {
        Ok(_) => {
            // Filters were applied successfully (may return empty results)
        }
        Err(e) => {
            assert!(
                e.contains("Failed to execute atuin command") ||
                e.contains("atuin command failed"),
                "Error should be a known atuin error type, got: {}",
                e
            );
        }
    }
}

#[test]
fn test_atuin_search_exit_filter_failure() {
    let filters = SearchFilters {
        directory: None,
        exit_filter: Some("failure".to_string()),
        time_range: None,
    };

    let result = atuin_search("git", Some(filters));

    // Should work and only return failed commands
    if let Ok(output) = result {
        if !output.is_empty() {
            for line in output.lines() {
                if let Some((_, exit, _, _)) = parse_atuin_line(line) {
                    assert_ne!(exit, "0", "Failure filter should exclude exit code 0");
                }
            }
        }
    }
}
