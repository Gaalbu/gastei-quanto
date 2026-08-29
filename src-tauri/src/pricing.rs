use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cache_write_5m: Option<u64>,
    pub cache_write_1h: Option<u64>,
    pub reasoning: u64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Cost {
    pub amount: f64,
    pub currency: &'static str,
    pub basis: &'static str,
    pub price_table_version: &'static str,
}

#[derive(Clone, Copy)]
pub struct ModelPrice {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cache_read_multiplier: f64,
    pub cache_write_multiplier: f64,
    pub cache_write_1h_multiplier: f64,
}

// Pricing tables are versioned and stored in `pricing-tables.json`.
// Updating prices: edit the JSON and run `cargo test` to validate totals.
// History is tracked via git; keep version keys as YYYY-MM-DD.
const PRICING_TABLES_JSON: &str = include_str!("../pricing-tables.json");

#[derive(serde::Deserialize)]
struct AnthropicEntry {
    input_per_million: f64,
    output_per_million: f64,
    cache_read_multiplier: f64,
    cache_write_multiplier: f64,
    cache_write_1h_multiplier: f64,
}

#[derive(serde::Deserialize)]
struct OpenAiEntry {
    input_per_million: f64,
    output_per_million: f64,
    cached_per_million: f64,
}

#[derive(serde::Deserialize)]
struct PricingTables {
    #[serde(rename = "anthropic-2026-08-28")]
    anthropic: HashMap<String, AnthropicEntry>,
    #[serde(rename = "openai-2026-08-28")]
    openai: HashMap<String, OpenAiEntry>,
}

fn tables() -> &'static PricingTables {
    static TABLES: OnceLock<PricingTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        serde_json::from_str(PRICING_TABLES_JSON).expect("pricing-tables.json must be valid")
    })
}

pub fn anthropic_price(model: &str) -> Option<ModelPrice> {
    let entry = tables().anthropic.get(model)?;
    Some(ModelPrice {
        input_per_million: entry.input_per_million,
        output_per_million: entry.output_per_million,
        cache_read_multiplier: entry.cache_read_multiplier,
        cache_write_multiplier: entry.cache_write_multiplier,
        cache_write_1h_multiplier: entry.cache_write_1h_multiplier,
    })
}

pub fn openai_price(model: &str) -> Option<ModelPrice> {
    let entry = tables().openai.get(model)?;
    Some(ModelPrice {
        input_per_million: entry.input_per_million,
        output_per_million: entry.output_per_million,
        // OpenAI: unified cache, price = input * (cached / input)
        cache_read_multiplier: entry.cached_per_million / entry.input_per_million,
        // OpenAI has no split cache durations; use 5m multiplier for unified write
        cache_write_multiplier: 1.25,
        cache_write_1h_multiplier: 1.25,
    })
}

pub fn calculate_cost(model: &str, usage: TokenUsage) -> Option<Cost> {
    let (price, version) = anthropic_price(model)
        .map(|price| (price, "anthropic-2026-08-28"))
        .or_else(|| openai_price(model).map(|price| (price, "openai-2026-08-28")))?;
    let million = 1_000_000.0;
    let cache_write_amount = match (usage.cache_write_5m, usage.cache_write_1h) {
        (Some(cache_write_5m), Some(cache_write_1h)) => {
            cache_write_5m as f64 * price.input_per_million * price.cache_write_multiplier / million
                + cache_write_1h as f64 * price.input_per_million * price.cache_write_1h_multiplier
                    / million
        }
        _ => {
            usage.cache_write as f64 * price.input_per_million * price.cache_write_multiplier
                / million
        }
    };
    let amount = usage.input as f64 * price.input_per_million / million
        + usage.output as f64 * price.output_per_million / million
        + usage.cache_read as f64 * price.input_per_million * price.cache_read_multiplier / million
        + cache_write_amount;

    Some(Cost {
        amount: (amount * 100_000_000.0).round() / 100_000_000.0,
        currency: "USD",
        basis: "estimated",
        price_table_version: version,
    })
}
