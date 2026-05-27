use ts_rs::TS;

/// A single parsed result from atuin search output
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, TS)]
pub struct AtuinResult {
    pub command: String,
    pub exit: String,
    pub duration: String,
    pub directory: String,
    pub time: String,
}

/// Parse a single line of atuin output in the format:
/// `{command}|{exit}|{duration}|{directory}|{time}`
///
/// The command field may contain `|` characters, so we split from the right.
pub fn parse_atuin_line(line: &str) -> Option<AtuinResult> {
    let parts: Vec<&str> = line.rsplitn(5, '|').collect();
    if parts.len() == 5 {
        Some(AtuinResult {
            command: parts[4].to_string(),
            exit: parts[3].to_string(),
            duration: parts[2].to_string(),
            directory: parts[1].to_string(),
            time: parts[0].to_string(),
        })
    } else {
        None
    }
}

/// Parse the full raw output from atuin search into a vec of results.
pub fn parse_atuin_output(raw: &str) -> Vec<AtuinResult> {
    let mut results = Vec::new();
    let mut skipping_multiline_record = false;

    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        match parse_atuin_line(line) {
            Some(result) if skipping_multiline_record => {
                // The current line is the metadata-bearing tail of a command that
                // began on an earlier physical line. With atuin's line-based
                // custom format, that true multiline command cannot be
                // reconstructed safely, so drop the whole record instead of
                // accepting a truncated continuation as a command.
                skipping_multiline_record = false;
                drop(result);
            }
            Some(result) => results.push(result),
            None => skipping_multiline_record = true,
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_containing_delimiters() {
        let parsed = parse_atuin_line("echo a|b|0|12ms|/tmp|2026-01-01").unwrap();
        assert_eq!(parsed.command, "echo a|b");
        assert_eq!(parsed.exit, "0");
        assert_eq!(parsed.duration, "12ms");
        assert_eq!(parsed.directory, "/tmp");
        assert_eq!(parsed.time, "2026-01-01");
    }

    #[test]
    fn skips_malformed_output_lines_and_their_parseable_tail() {
        let raw = "not enough fields\ntruncated tail|0|1ms|/tmp|now\nvalid|0|1ms|/tmp|now\n";
        assert_eq!(
            parse_atuin_output(raw),
            vec![AtuinResult {
                command: "valid".to_string(),
                exit: "0".to_string(),
                duration: "1ms".to_string(),
                directory: "/tmp".to_string(),
                time: "now".to_string(),
            }]
        );
    }

    #[test]
    fn multiline_command_output_does_not_accept_truncated_continuation() {
        let raw = "printf 'one\ncontinuation with delimiters|0|3ms|/Users/me|yesterday\nls -la|0|3ms|/Users/me|today\n";
        let parsed = parse_atuin_output(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].command, "ls -la");
    }
}
