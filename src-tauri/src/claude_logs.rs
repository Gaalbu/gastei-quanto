use crate::pricing::{calculate_cost, TokenUsage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct LogLine {
    timestamp: Option<String>,
    cwd: Option<String>,
    #[serde(alias = "sessionId")]
    session_id: Option<String>,
    #[serde(alias = "gitBranch")]
    git_branch: Option<String>,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub total: f64,
    pub currency: &'static str,
    pub basis: &'static str,
    pub price_table_version: &'static str,
    pub by_model: BTreeMap<String, Breakdown>,
    pub by_project: BTreeMap<String, Breakdown>,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct Breakdown {
    pub amount: f64,
    pub tokens: TokenTotals,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct TokenTotals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

pub fn parse_line(line: &str, month: &str) -> Option<(String, String, Breakdown)> {
    let parsed: LogLine = serde_json::from_str(line).ok()?;
    let timestamp = parsed.timestamp.as_deref()?;
    if !timestamp.starts_with(month) {
        return None;
    }

    let message = parsed.message?;
    let model = message.model?;
    let usage = message.usage?;
    let tokens = TokenTotals {
        input: usage.input_tokens.unwrap_or_default(),
        output: usage.output_tokens.unwrap_or_default(),
        cache_read: usage.cache_read_input_tokens.unwrap_or_default(),
        cache_write: usage.cache_creation_input_tokens.unwrap_or_default(),
    };
    let cost = calculate_cost(
        &model,
        TokenUsage {
            input: tokens.input,
            output: tokens.output,
            cache_read: tokens.cache_read,
            cache_write: tokens.cache_write,
            reasoning: 0,
        },
    )?;
    let project = parsed.cwd.or(parsed.session_id).unwrap_or_else(|| "sem projeto".to_string());

    Some((model, project, Breakdown { amount: cost.amount, tokens }))
}

pub fn aggregate<'a>(lines: impl IntoIterator<Item = &'a str>, month: &str) -> Report {
    let mut report = Report {
        currency: "USD",
        basis: "estimated",
        price_table_version: "anthropic-2026-08-28",
        ..Report::default()
    };

    for line in lines {
        let Some((model, project, breakdown)) = parse_line(line, month) else {
            continue;
        };
        report.total += breakdown.amount;
        add_breakdown(report.by_model.entry(model).or_default(), &breakdown);
        add_breakdown(report.by_project.entry(project).or_default(), &breakdown);
    }

    report
}

fn add_breakdown(target: &mut Breakdown, source: &Breakdown) {
    target.amount += source.amount;
    target.tokens.input += source.tokens.input;
    target.tokens.output += source.tokens.output;
    target.tokens.cache_read += source.tokens.cache_read;
    target.tokens.cache_write += source.tokens.cache_write;
}

#[cfg(test)]
mod tests {
    use super::{aggregate, parse_line};

    const LINE: &str = r#"{"timestamp":"2026-08-28T12:00:00Z","cwd":"/work/app","session_id":"s1","message":{"model":"claude-sonnet-5","usage":{"input_tokens":1000000,"output_tokens":1000000,"cache_creation_input_tokens":1000000,"cache_read_input_tokens":1000000}}}"#;

    #[test]
    fn parses_only_matching_month_and_keeps_project() {
        let (_, project, breakdown) = parse_line(LINE, "2026-08").expect("valid line");
        assert_eq!(project, "/work/app");
        assert_eq!(breakdown.tokens.cache_read, 1_000_000);
        assert!(parse_line(LINE, "2026-07").is_none());
    }

    #[test]
    fn aggregates_same_model_and_project() {
        let report = aggregate([LINE, LINE], "2026-08");
        assert_eq!(report.by_model["claude-sonnet-5"].tokens.input, 2_000_000);
        assert_eq!(report.by_project["/work/app"].tokens.output, 2_000_000);
        assert_eq!(report.total, 26.7);
    }

    #[test]
    fn ignores_malformed_and_unknown_model_lines() {
        let report = aggregate(["not json", r#"{"timestamp":"2026-08-28","message":{"model":"unknown","usage":{}}}"#], "2026-08");
        assert!(report.by_model.is_empty());
        assert_eq!(report.total, 0.0);
    }
}
