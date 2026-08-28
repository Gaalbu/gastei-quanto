use crate::claude_logs::{Breakdown, Report, TokenTotals};
use crate::pricing::{calculate_cost, TokenUsage};
use chrono::{DateTime, FixedOffset, Local};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Deserialize)]
struct Line {
    timestamp: Option<String>,
    r#type: Option<String>,
    payload: Option<Payload>,
}
#[derive(Deserialize)]
struct Payload {
    r#type: Option<String>,
    turn_id: Option<String>,
    model: Option<String>,
    cwd: Option<String>,
    info: Option<Info>,
}
#[derive(Deserialize)]
struct Info {
    last_token_usage: Option<Usage>,
    total_token_usage: Option<Usage>,
}
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    cache_write_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

fn local_month(value: &str, month: &str) -> bool {
    value
        .parse::<DateTime<FixedOffset>>()
        .map(|v| v.with_timezone(&Local).format("%Y-%m").to_string() == month)
        .unwrap_or(false)
}

pub(crate) fn collect_jsonl(root: &Path, report: &mut Report, month: &str) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_jsonl(&path, report, month)?;
        } else if path.extension().and_then(|v| v.to_str()) == Some("jsonl") {
            process_file(&path, report, month)?;
        }
    }
    Ok(())
}

fn process_file(path: &Path, report: &mut Report, month: &str) -> io::Result<()> {
    let mut model: Option<String> = None;
    let mut project = "sem projeto".to_string();
    let mut seen: HashSet<Usage> = HashSet::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let Ok(line) = line else { continue };
        let Ok(event) = serde_json::from_str::<Line>(&line) else {
            continue;
        };
        let Some(payload) = event.payload else {
            continue;
        };
        if event.r#type.as_deref() == Some("turn_context")
            || payload.r#type.as_deref() == Some("turn_context")
        {
            if payload.turn_id.is_none() {
                continue;
            }
            model = payload.model;
            project = payload.cwd.unwrap_or_else(|| "sem projeto".to_string());
            seen.clear();
            continue;
        }
        if event.r#type.as_deref() != Some("event_msg")
            || payload.r#type.as_deref() != Some("token_count")
            || !local_month(event.timestamp.as_deref().unwrap_or(""), month)
        {
            continue;
        }
        let Some(info) = payload.info else { continue };
        let Some(last) = info.last_token_usage else {
            continue;
        };
        let Some(model) = model.as_deref() else {
            continue;
        };
        let fingerprint = info.total_token_usage.unwrap_or_else(|| last.clone());
        if !seen.insert(fingerprint) {
            continue;
        }
        let tokens = TokenTotals {
            input: last
                .input_tokens
                .saturating_sub(last.cached_input_tokens)
                .saturating_sub(last.cache_write_input_tokens),
            output: last.output_tokens,
            cache_read: last.cached_input_tokens,
            cache_write: last.cache_write_input_tokens,
            ..TokenTotals::default()
        };
        let usage = TokenUsage {
            input: tokens.input,
            output: tokens.output,
            cache_read: tokens.cache_read,
            cache_write: tokens.cache_write,
            ..TokenUsage::default()
        };
        let Some(cost) = calculate_cost(model, usage) else {
            add_unpriced(report, model, &tokens);
            continue;
        };
        let breakdown = Breakdown {
            amount: cost.amount,
            tokens: tokens.clone(),
        };
        report.total += cost.amount;
        report
            .by_model
            .entry(model.to_string())
            .or_default()
            .add(&breakdown);
        report
            .by_project
            .entry(project.clone())
            .or_default()
            .add(&breakdown);
        report
            .by_provider
            .entry("Codex".to_string())
            .or_default()
            .add(&breakdown);
    }
    Ok(())
}

fn add_unpriced(report: &mut Report, model: &str, tokens: &TokenTotals) {
    let target = report.unpriced_models.entry(model.to_string()).or_default();
    target.input += tokens.input;
    target.output += tokens.output;
    target.cache_read += tokens.cache_read;
    target.cache_write += tokens.cache_write;
}

trait AddBreakdown {
    fn add(&mut self, source: &Breakdown);
}
impl AddBreakdown for Breakdown {
    fn add(&mut self, source: &Breakdown) {
        self.amount += source.amount;
        self.tokens.input += source.tokens.input;
        self.tokens.output += source.tokens.output;
        self.tokens.cache_read += source.tokens.cache_read;
        self.tokens.cache_write += source.tokens.cache_write;
    }
}

#[cfg(test)]
mod tests {
    use super::process_file;
    use crate::claude_logs::new_report;
    use std::io::Write;

    const FIXTURE: &str = include_str!("../tests/fixtures/codex_usage.jsonl");

    fn fixture(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        file
    }

    #[test]
    fn deduplicates_repeated_cumulative_snapshot_but_counts_distinct_calls() {
        let file = fixture(&[
            r#"{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t1","model":"gpt-5.6-luna","cwd":"/work"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:00:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":10},"total_token_usage":{"input_tokens":100,"output_tokens":10}}}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:01:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":10},"total_token_usage":{"input_tokens":100,"output_tokens":10}}}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:02:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":200,"output_tokens":20},"total_token_usage":{"input_tokens":300,"output_tokens":30}}}}"#,
        ]);
        let mut report = new_report();
        process_file(file.path(), &mut report, "2026-08").unwrap();
        assert_eq!(report.by_model["gpt-5.6-luna"].tokens.input, 300);
        assert_eq!(report.by_model["gpt-5.6-luna"].tokens.output, 30);
    }

    #[test]
    fn keeps_unknown_codex_models_visible_without_cost() {
        let file = fixture(&[
            r#"{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t1","model":"gpt-reserve","cwd":"/work"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:00:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":5,"output_tokens":2}}}}"#,
        ]);
        let mut report = new_report();
        process_file(file.path(), &mut report, "2026-08").unwrap();
        assert_eq!(report.unpriced_models["gpt-reserve"].input, 5);
        assert_eq!(report.total, 0.0);
    }

    #[test]
    fn ignores_events_without_turn_context_or_valid_info() {
        let file = fixture(&[
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:00:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":5,"output_tokens":2}}}}"#,
            r#"{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t1","model":"gpt-5.6-luna","cwd":"/work"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:01:00Z","payload":{"type":"token_count","info":null}}"#,
        ]);
        let mut report = new_report();
        process_file(file.path(), &mut report, "2026-08").unwrap();
        assert!(report.by_model.is_empty());
    }

    #[test]
    fn resets_deduplication_when_turn_changes() {
        let file = fixture(&[
            r#"{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t1","model":"gpt-5.6-luna","cwd":"/one"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:00:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":10},"total_token_usage":{"input_tokens":100,"output_tokens":10}}}}"#,
            r#"{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t2","model":"gpt-5.6-luna","cwd":"/two"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:01:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":10},"total_token_usage":{"input_tokens":100,"output_tokens":10}}}}"#,
        ]);
        let mut report = new_report();
        process_file(file.path(), &mut report, "2026-08").unwrap();
        assert_eq!(report.by_model["gpt-5.6-luna"].tokens.input, 200);
        assert_eq!(report.by_project["/one"].tokens.input, 100);
        assert_eq!(report.by_project["/two"].tokens.input, 100);
    }

    #[test]
    fn subtracts_cached_and_cache_write_input_saturating() {
        let file = fixture(&[
            r#"{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t1","model":"gpt-5.6-luna","cwd":"/work"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-08-28T12:00:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":5,"cached_input_tokens":8,"cache_write_input_tokens":4,"output_tokens":2}}}}"#,
        ]);
        let mut report = new_report();
        process_file(file.path(), &mut report, "2026-08").unwrap();
        let tokens = &report.by_model["gpt-5.6-luna"].tokens;
        assert_eq!(tokens.input, 0);
        assert_eq!(tokens.cache_read, 8);
        assert_eq!(tokens.cache_write, 4);
    }

    #[test]
    fn integrates_fixture_across_turns_and_models() {
        let file = fixture(FIXTURE.lines().collect::<Vec<_>>().as_slice());
        let mut report = new_report();
        process_file(file.path(), &mut report, "2026-08").unwrap();
        assert_eq!(report.by_model["gpt-5.6-luna"].tokens.input, 130);
        assert_eq!(report.by_model["gpt-5.5"].tokens.input, 40);
        assert_eq!(report.by_provider["Codex"].tokens.output, 19);
    }
}
