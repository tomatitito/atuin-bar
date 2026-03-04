# Plan: Property-Based Testing for Atuin-Bar Completions

## Goal

Verify that atuin-bar faithfully represents atuin's search results through two
complementary property-based tests:

1. **Parser roundtrip** — can we correctly parse any valid atuin output?
2. **CLI equivalence** — are we asking atuin the right question?

## Prerequisites: Move the Parser to Rust

The parser currently lives in TypeScript (`src/main.ts:parseAtuinLine`). A
duplicate (and broken — only 4 fields instead of 5) exists in the integration
test. Before we can property-test the parser in Rust, we need a single canonical
parser there.

### Step 1: Add `AtuinResult` struct to `lib.rs`

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AtuinResult {
    pub command: String,
    pub exit: String,
    pub duration: String,
    pub directory: String,
    pub time: String,
}
```

Needs `Serialize` so Tauri auto-converts it to JSON over IPC.
Needs `PartialEq` for test assertions.

### Step 2: Add `parse_atuin_output` function to `lib.rs`

```rust
pub fn parse_atuin_line(line: &str) -> Option<AtuinResult> {
    // 5 fields: command|exit|duration|directory|time
    // command can contain '|', so split from the right
    let parts: Vec<&str> = line.rsplitn(5, '|').collect();
    if parts.len() == 5 {
        Some(AtuinResult {
            command:   parts[4].to_string(),
            exit:      parts[3].to_string(),
            duration:  parts[2].to_string(),
            directory: parts[1].to_string(),
            time:      parts[0].to_string(),
        })
    } else {
        None
    }
}

pub fn parse_atuin_output(raw: &str) -> Vec<AtuinResult> {
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(parse_atuin_line)
        .collect()
}
```

### Step 3: Change `atuin_search` return type

Change signature from `Result<String, String>` to
`Result<Vec<AtuinResult>, String>`. Parse the raw output before returning.

The Tauri command wrapper `atuin_search_command` inherits this — Tauri will
serialize `Vec<AtuinResult>` as JSON automatically.

### Step 4: Simplify the frontend

In `src/main.ts`:
- Remove `parseAtuinLine()` function
- Remove the `AtuinResult` interface (or keep it as a TypeScript type matching
  the Rust struct)
- The `invoke("atuin_search_command", ...)` call now returns
  `AtuinResult[]` directly — no parsing needed

### Step 5: Fix the integration test

In `atuin_integration_test.rs`:
- Remove the local `parse_atuin_line` helper
- Import `parse_atuin_line` and `AtuinResult` from `atuin_bar_lib`
- Update assertions to work with `Vec<AtuinResult>` instead of raw strings

---

## Property-Based Test 1: Parser Roundtrip

**Property:** For any arbitrary (command, exit, duration, directory, time)
tuple, formatting it as `"{command}|{exit}|{duration}|{directory}|{time}"` and
parsing it back with `parse_atuin_line` should yield the original values.

**What it catches:** Edge cases in parsing — pipes in commands, unicode,
empty fields, unusual characters.

**Requirements:** None (pure test, no atuin needed).

### Implementation

Add `proptest` to `[dev-dependencies]` in `Cargo.toml`.

```rust
use proptest::prelude::*;

/// Generate strings that don't contain newlines (atuin output is line-delimited)
fn arbitrary_field() -> impl Strategy<Value = String> {
    "[^\n]{0,100}"
}

/// Commands can contain '|' but not newlines
fn arbitrary_command() -> impl Strategy<Value = String> {
    "[^\n]{1,200}"
}

/// Fields other than command must NOT contain '|' (they are the delimiters)
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
```

**Key constraint:** The 4 metadata fields (exit, duration, directory, time)
must not contain `|` — otherwise the `rsplitn(5, '|')` split is ambiguous.
Only the command field (leftmost) can contain pipes. This is a real invariant
of atuin's output format since these fields are structured values (integers,
paths, timestamps).

### Location

New file: `src-tauri/tests/property_tests.rs`

---

## Property-Based Test 2: CLI Equivalence

**Property:** For any query and filter combination, the commands returned by
`atuin_search(query, filters)` should match the commands returned by calling
`atuin search` directly with the same arguments but a simpler format.

**What it catches:** Incorrect argument construction, wrong filter mapping,
results being lost or reordered.

**Requirements:** atuin installed with history data. Mark as `#[ignore]` for
CI, run explicitly during development.

### Implementation

```rust
proptest! {
    #[test]
    #[ignore] // requires atuin with history
    fn cli_equivalence(
        query in "[a-zA-Z]{0,8}",
        exit_filter in prop::option::of(prop_oneof!["success", "failure"]),
        time_range in prop::option::of(prop_oneof!["1h", "24h", "7d", "30d"]),
    ) {
        let filters = SearchFilters {
            directory: None,
            exit_filter: exit_filter.clone(),
            time_range: time_range.clone(),
        };

        // Get results through atuin-bar's pipeline (format + parse)
        let bar_results = atuin_search(&query, Some(filters));

        // Build equivalent direct atuin command with just "{command}" format
        let mut cmd = Command::new(find_atuin_binary());
        cmd.args(["search", "--search-mode", "prefix", "--limit", "50"]);
        cmd.args(["--format", "{command}"]);
        // ... apply same filters ...
        cmd.arg(&query);
        let direct = cmd.output();

        // Compare: same commands in same order
        match (bar_results, direct) {
            (Ok(bar), Ok(direct_out)) if direct_out.status.success() => {
                let direct_commands: Vec<&str> = String::from_utf8_lossy(&direct_out.stdout)
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .collect();
                let bar_commands: Vec<&str> = bar.iter()
                    .map(|r| r.command.as_str())
                    .collect();
                prop_assert_eq!(bar_commands, direct_commands);
            }
            _ => {} // both failing is acceptable (atuin not installed)
        }
    }
}
```

### Location

Same file: `src-tauri/tests/property_tests.rs`

---

## File Changes Summary

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `proptest = "1"` to dev-dependencies |
| `src-tauri/src/lib.rs` | Add `AtuinResult` struct, `parse_atuin_line`, `parse_atuin_output`. Change `atuin_search` return type. |
| `src/main.ts` | Remove `parseAtuinLine()`, simplify to consume typed JSON |
| `src-tauri/tests/property_tests.rs` | New file: roundtrip + CLI equivalence property tests |
| `src-tauri/tests/atuin_integration_test.rs` | Remove local parser, use `parse_atuin_line` from lib, update for new return type |

## Execution Order

1. Steps 1-3 (Rust parser + return type change)
2. Step 5 (fix integration test — so `cargo test` passes)
3. Step 4 (frontend simplification)
4. Property test 1 (parser roundtrip)
5. Property test 2 (CLI equivalence)

Steps 4 and 5 can be done in parallel since they touch different files.
