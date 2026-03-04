use atuin_bar_lib::{atuin_search, SearchFilters};
use serial_test::serial;

#[test]
fn test_atuin_search_e2e() {
    let result = atuin_search("ls", None);

    match result {
        Ok(results) => {
            println!("Atuin search returned {} results", results.len());

            if !results.is_empty() {
                for r in &results {
                    assert!(!r.command.is_empty(), "Command should not be empty");
                    assert!(!r.exit.is_empty(), "Exit code should not be empty");
                    assert!(!r.time.is_empty(), "Time should not be empty");
                }
            }
        }
        Err(e) => {
            println!("Atuin search failed (OK if atuin is not installed): {}", e);
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
    let result = atuin_search("", None);

    match result {
        Ok(_) => {}
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
fn test_atuin_search_special_characters() {
    let queries = vec![
        "git commit",
        "cd ..",
        "echo 'hello world'",
    ];

    for query in queries {
        let result = atuin_search(query, None);

        match result {
            Ok(_) => {}
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
}

#[test]
fn test_atuin_search_output_format() {
    let result = atuin_search("cargo", None);

    if let Ok(results) = result {
        if let Some(r) = results.first() {
            assert!(!r.command.is_empty(), "Command should not be empty");

            let _exit_code: i32 = r.exit.parse().expect("Exit code should be a number");

            assert!(
                r.directory.contains('/') || r.directory == "unknown" || r.directory.is_empty(),
                "Directory should be a path or 'unknown', got: {}",
                r.directory
            );

            assert!(!r.time.is_empty(), "Timestamp should not be empty");
        }
    }
}

#[test]
fn test_atuin_search_with_filters() {
    let filters = SearchFilters {
        directory: Some("/tmp".to_string()),
        exit_filter: Some("success".to_string()),
        time_range: Some("7d".to_string()),
    };

    let result = atuin_search("", Some(filters));

    match result {
        Ok(_) => {}
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

    if let Ok(results) = result {
        for r in &results {
            assert_ne!(r.exit, "0", "Failure filter should exclude exit code 0");
        }
    }
}

/// Verify that atuin_search works even when ATUIN_SESSION is not in
/// the process environment.  A GUI app never has it, so the function
/// must set it on the spawned Command itself.
#[test]
#[serial]
fn test_atuin_search_works_without_atuin_session_env() {
    let saved = std::env::var("ATUIN_SESSION").ok();
    std::env::remove_var("ATUIN_SESSION");

    let result = atuin_search("ls", None);

    // Restore before any assertions so a failure doesn't leak state.
    match saved {
        Some(v) => std::env::set_var("ATUIN_SESSION", v),
        None => {}
    }

    match result {
        Ok(_) => {} // success — atuin_search provided the env var itself
        Err(e) => {
            assert!(
                !e.contains("ATUIN_SESSION"),
                "atuin_search must set ATUIN_SESSION on the command; got: {}",
                e
            );
        }
    }
}
