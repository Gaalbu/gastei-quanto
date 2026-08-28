use serde::Serialize;

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
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
}

pub fn anthropic_price(model: &str) -> Option<ModelPrice> {
    match model {
        "claude-opus-5" => Some(ModelPrice {
            input_per_million: 5.0,
            output_per_million: 25.0,
            cache_read_multiplier: 0.10,
            cache_write_multiplier: 1.25,
        }),
        "claude-sonnet-5" => Some(ModelPrice {
            input_per_million: 2.0,
            output_per_million: 10.0,
            cache_read_multiplier: 0.10,
            cache_write_multiplier: 1.25,
        }),
        "claude-haiku-4-5" => Some(ModelPrice {
            input_per_million: 1.0,
            output_per_million: 5.0,
            cache_read_multiplier: 0.10,
            cache_write_multiplier: 1.25,
        }),
        _ => None,
    }
}

pub fn calculate_cost(model: &str, usage: TokenUsage) -> Option<Cost> {
    let price = anthropic_price(model)?;
    let million = 1_000_000.0;
    let amount = usage.input as f64 * price.input_per_million / million
        + usage.output as f64 * price.output_per_million / million
        + usage.cache_read as f64 * price.input_per_million * price.cache_read_multiplier / million
        + usage.cache_write as f64
            * price.input_per_million
            * price.cache_write_multiplier
            / million;

    Some(Cost {
        amount: (amount * 100_000_000.0).round() / 100_000_000.0,
        currency: "USD",
        basis: "estimated",
        price_table_version: "anthropic-2026-08-28",
    })
}
