use crate::pricing::{calculate_cost, TokenUsage};
use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Deserialize)]
struct LogLine {
    timestamp: Option<String>,
    cwd: Option<String>,
    #[serde(alias = "requestId")]
    request_id: Option<String>,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(alias = "id")]
    message_id: Option<String>,
    #[serde(alias = "requestId")]
    request_id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_creation: Option<CacheCreation>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CacheCreation {
    ephemeral_5m_input_tokens: Option<u64>,
    ephemeral_1h_input_tokens: Option<u64>,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub total: f64,
    pub currency: &'static str,
    pub basis: &'static str,
    pub price_table_version: &'static str,
    pub by_model: BTreeMap<String, Breakdown>,
    pub by_project: BTreeMap<String, Breakdown>,
    pub unpriced_models: BTreeMap<String, TokenTotals>,
    pub by_provider: BTreeMap<String, Breakdown>,
}

pub(crate) fn new_report() -> Report {
    Report {
        currency: "USD",
        basis: "estimated",
        price_table_version: "anthropic-2026-08-28",
        ..Report::default()
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct Breakdown {
    pub amount: f64,
    pub tokens: TokenTotals,
}

#[derive(Debug, Default, Serialize, Clone, PartialEq)]
pub struct TokenTotals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
}

struct ParsedEvent {
    model: String,
    project: String,
    tokens: TokenTotals,
    dedup_key: Option<(String, String)>,
    cache_split: bool,
}

fn is_in_local_month(timestamp: &str, month: &str) -> bool {
    let Ok(parsed) = timestamp.parse::<DateTime<FixedOffset>>() else {
        return false;
    };
    parsed.with_timezone(&Local).format("%Y-%m").to_string() == month
}

fn parse_event(line: &str, month: &str) -> Option<ParsedEvent> {
    let parsed: LogLine = serde_json::from_str(line).ok()?;
    if !is_in_local_month(parsed.timestamp.as_deref()?, month) {
        return None;
    }
    let message = parsed.message?;
    let model = message.model?;
    let usage = message.usage?;
    let cache_write_total = usage.cache_creation_input_tokens.unwrap_or_else(|| {
        usage.cache_creation.as_ref().map_or(0, |cache| {
            cache.ephemeral_5m_input_tokens.unwrap_or_default()
                + cache.ephemeral_1h_input_tokens.unwrap_or_default()
        })
    });
    let (cache_write_5m, cache_write_1h, cache_split) = match (
        usage.cache_creation_input_tokens,
        usage.cache_creation.as_ref(),
    ) {
        (Some(total), Some(cache))
            if cache.ephemeral_5m_input_tokens.is_some()
                && cache.ephemeral_1h_input_tokens.is_some()
                && cache.ephemeral_5m_input_tokens.unwrap_or_default()
                    + cache.ephemeral_1h_input_tokens.unwrap_or_default()
                    == total =>
        {
            (
                cache.ephemeral_5m_input_tokens,
                cache.ephemeral_1h_input_tokens,
                true,
            )
        }
        _ => (None, None, false),
    };
    Some(ParsedEvent {
        model,
        project: parsed.cwd.unwrap_or_else(|| "sem projeto".to_string()),
        tokens: TokenTotals {
            input: usage.input_tokens.unwrap_or_default(),
            output: usage.output_tokens.unwrap_or_default(),
            cache_read: usage.cache_read_input_tokens.unwrap_or_default(),
            cache_write: cache_write_total,
            cache_write_5m: cache_write_5m.unwrap_or_default(),
            cache_write_1h: cache_write_1h.unwrap_or_default(),
        },
        dedup_key: message
            .message_id
            .zip(parsed.request_id.or(message.request_id)),
        cache_split,
    })
}

pub fn aggregate<'a>(lines: impl IntoIterator<Item = &'a str>, month: &str) -> Report {
    let mut report = new_report();
    let mut seen = HashSet::new();

    for line in lines {
        add_line(&mut report, &mut seen, line, month);
    }
    report
}

pub(crate) fn add_line(
    report: &mut Report,
    seen: &mut HashSet<(String, String)>,
    line: &str,
    month: &str,
) {
    let Some(event) = parse_event(line, month) else {
        return;
    };
    if let Some(key) = &event.dedup_key {
        if !seen.insert(key.clone()) {
            return;
        }
    }
    let usage = TokenUsage {
        input: event.tokens.input,
        output: event.tokens.output,
        cache_read: event.tokens.cache_read,
        cache_write: event.tokens.cache_write,
        cache_write_5m: event.cache_split.then_some(event.tokens.cache_write_5m),
        cache_write_1h: event.cache_split.then_some(event.tokens.cache_write_1h),
        reasoning: 0,
    };
    let Some(cost) = calculate_cost(&event.model, usage) else {
        add_tokens(
            report.unpriced_models.entry(event.model).or_default(),
            &event.tokens,
        );
        return;
    };
    let breakdown = Breakdown {
        amount: cost.amount,
        tokens: event.tokens,
    };
    report.total += breakdown.amount;
    add_breakdown(report.by_model.entry(event.model).or_default(), &breakdown);
    add_breakdown(
        report.by_project.entry(event.project).or_default(),
        &breakdown,
    );
    add_breakdown(
        report
            .by_provider
            .entry("Claude Code".to_string())
            .or_default(),
        &breakdown,
    );
}

fn add_breakdown(target: &mut Breakdown, source: &Breakdown) {
    target.amount += source.amount;
    add_tokens(&mut target.tokens, &source.tokens);
}

fn add_tokens(target: &mut TokenTotals, source: &TokenTotals) {
    target.input += source.input;
    target.output += source.output;
    target.cache_read += source.cache_read;
    target.cache_write += source.cache_write;
    target.cache_write_5m += source.cache_write_5m;
    target.cache_write_1h += source.cache_write_1h;
}

#[cfg(test)]
mod tests {
    use super::aggregate;

    const FIXTURE: &str = include_str!("../tests/fixtures/claude_usage.jsonl");

    #[test]
    fn deduplicates_repeated_usage_for_same_message_and_request() {
        let report = aggregate(FIXTURE.lines().take(3), "2026-08");
        assert_eq!(report.by_model["claude-sonnet-5"].tokens.input, 1_000_000);
    }

    #[test]
    fn counts_distinct_message_ids() {
        let report = aggregate(FIXTURE.lines().skip(3).take(3), "2026-08");
        assert_eq!(report.by_model["claude-sonnet-5"].tokens.input, 3_000_000);
    }

    #[test]
    fn prices_one_hour_and_five_minute_cache_separately() {
        let report = aggregate(FIXTURE.lines().skip(6).take(1), "2026-08");
        assert_eq!(
            report.by_model["claude-sonnet-5"].tokens.cache_write_5m,
            1_000_000
        );
        assert_eq!(
            report.by_model["claude-sonnet-5"].tokens.cache_write_1h,
            1_000_000
        );
        assert_eq!(report.by_model["claude-sonnet-5"].amount, 6.5);
    }

    #[test]
    fn missing_cache_creation_uses_five_minute_fallback() {
        let line = r#"{"timestamp":"2026-08-28T12:00:00Z","cwd":"/work/app","message":{"model":"claude-sonnet-5","usage":{"cache_creation_input_tokens":1000000}}}"#;
        let report = aggregate([line], "2026-08");
        assert_eq!(report.by_model["claude-sonnet-5"].amount, 2.5);
    }

    #[test]
    fn missing_cwd_falls_back_to_visible_bucket() {
        let line = r#"{"timestamp":"2026-08-28T12:00:00Z","cwd":null,"sessionId":"secret-session","message":{"model":"claude-sonnet-5","usage":{"input_tokens":1}}}"#;
        let report = aggregate([line], "2026-08");
        assert!(report.by_project.contains_key("sem projeto"));
        assert!(!report.by_project.contains_key("secret-session"));
    }
}
