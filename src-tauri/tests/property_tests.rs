use atuin_bar_lib::{atuin_search, parse_atuin_line, ExitFilter, SearchFilters, TimeRange};
use proptest::prelude::*;
use std::process::Command;

/// Commands can contain '|' but not newlines
fn arbitrary_command() -> impl Strategy<Value = String> {
    "[^\n]{1,200}"
}

/// Metadata fields must NOT contain '|' (they are delimiters) or newlines
fn arbitrary_metadata_field() -> impl Strategy<Value = String> {
    "[^\n|]{1,50}"
}

proptest! {
    #[test]
    fn parser_roundtrip(
        command in arbitrary_command(),
        exit in arbitrary_metadata_field(),
        duration in arbitrary_metadata_field(),
        directory in arbitrary_metadata_field(),
        time in arbitrary_metadata_field(),
    ) {
        let line = format!("{}|{}|{}|{}|{}", command, exit, duration, directory, time);
        let parsed = parse_atuin_line(&line).expect("should parse successfully");

        prop_assert_eq!(&parsed.command, &command);
        prop_assert_eq!(&parsed.exit, &exit);
        prop_assert_eq!(&parsed.duration, &duration);
        prop_assert_eq!(&parsed.directory, &directory);
        prop_assert_eq!(&parsed.time, &time);
    }
}

/// Resolve the full path to the `atuin` binary (same logic as lib.rs).
fn find_atuin_binary() -> String {
    let candidates = ["/opt/homebrew/bin/atuin", "/usr/local/bin/atuin"];
    let home = std::env::var("HOME").unwrap_or_default();
    let cargo_candidate = format!("{}/.cargo/bin/atuin", home);

    for path in candidates
        .iter()
        .chain(std::iter::once(&cargo_candidate.as_str()))
    {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "atuin".to_string()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn cli_equivalence(
        query in "[a-zA-Z]{0,8}",
        exit_filter in proptest::option::of(prop_oneof![Just("success".to_string()), Just("failure".to_string())]),
        time_range in proptest::option::of(prop_oneof![Just("1h".to_string()), Just("24h".to_string()), Just("7d".to_string()), Just("30d".to_string())]),
    ) {
        let filters = SearchFilters {
            directory: None,
            exit_filter: exit_filter.as_deref().map(|ef| match ef {
                "success" => ExitFilter::Success,
                "failure" => ExitFilter::Failure,
                _ => unreachable!(),
            }),
            time_range: time_range.as_deref().map(|tr| match tr {
                "1h" => TimeRange::OneHour,
                "24h" => TimeRange::TwentyFourHours,
                "7d" => TimeRange::SevenDays,
                "30d" => TimeRange::ThirtyDays,
                _ => unreachable!(),
            }),
        };

        // Get results through atuin-bar's pipeline (format + parse)
        let bar_results = atuin_search(&query, Some(filters));

        // Build equivalent direct atuin command with just "{command}" format
        let atuin_bin = find_atuin_binary();
        let mut cmd = Command::new(&atuin_bin);
        cmd.env("ATUIN_SESSION", "atuin-bar");
        cmd.args(["search", "--search-mode", "prefix", "--filter-mode", "global", "--limit", "50"]);
        cmd.args(["--format", "{command}"]);

        if let Some(ref ef) = exit_filter {
            match ef.as_str() {
                "success" => { cmd.arg("--exit").arg("0"); }
                "failure" => { cmd.arg("--exclude-exit").arg("0"); }
                _ => {}
            }
        }

        if let Some(ref tr) = time_range {
            let after = match tr.as_str() {
                "1h" => Some("1 hour ago"),
                "24h" => Some("1 day ago"),
                "7d" => Some("7 days ago"),
                "30d" => Some("30 days ago"),
                _ => None,
            };
            if let Some(after_str) = after {
                cmd.arg("--after").arg(after_str);
            }
        }

        cmd.arg(&query);
        let direct = cmd.output();

        // Compare: our parsed commands should be a subsequence of the direct
        // output. Multi-line commands (with \ continuations) produce extra lines
        // in the direct {command}-only output that our 5-field parser correctly
        // skips, so an exact match is not expected.
        let bar_results = bar_results.map_err(|e| {
            TestCaseError::fail(format!("atuin_search returned error: {}", e))
        })?;

        let direct_out = direct.map_err(|e| {
            TestCaseError::fail(format!("failed to run atuin binary: {}", e))
        })?;

        // atuin exits 1 when there are no results — treat empty stderr as "no results"
        let direct_stderr = String::from_utf8_lossy(&direct_out.stderr);
        if !direct_out.status.success() && !direct_stderr.trim().is_empty() {
            return Err(TestCaseError::fail(format!(
                "atuin exited with status {}: {}",
                direct_out.status, direct_stderr
            )));
        }

        let direct_stdout = String::from_utf8_lossy(&direct_out.stdout);
        let direct_commands: Vec<&str> = direct_stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        let bar_commands: Vec<&str> = bar_results.iter()
            .map(|r| r.command.as_str())
            .collect();

        // Every command from bar should appear in direct output, in order
        let mut direct_iter = direct_commands.iter();
        for bar_cmd in &bar_commands {
            let found = direct_iter.any(|d| d == bar_cmd);
            prop_assert!(
                found,
                "Command {:?} from bar pipeline not found (in order) in direct output",
                bar_cmd
            );
        }
    }
}
