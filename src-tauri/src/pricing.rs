use serde::Serialize;

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

pub fn anthropic_price(model: &str) -> Option<ModelPrice> {
    match model {
        "claude-opus-5" => Some(ModelPrice {
            input_per_million: 5.0,
            output_per_million: 25.0,
            cache_read_multiplier: 0.10,
            cache_write_multiplier: 1.25,
            cache_write_1h_multiplier: 2.0,
        }),
        "claude-sonnet-5" => Some(ModelPrice {
            input_per_million: 2.0,
            output_per_million: 10.0,
            cache_read_multiplier: 0.10,
            cache_write_multiplier: 1.25,
            cache_write_1h_multiplier: 2.0,
        }),
        "claude-haiku-4-5" => Some(ModelPrice {
            input_per_million: 1.0,
            output_per_million: 5.0,
            cache_read_multiplier: 0.10,
            cache_write_multiplier: 1.25,
            cache_write_1h_multiplier: 2.0,
        }),
        _ => None,
    }
}

pub fn openai_price(model: &str) -> Option<ModelPrice> {
    let (input, cached, output) = match model {
        "gpt-5.6-sol" => (4.0, 0.40, 20.0),
        "gpt-5.6-terra" => (2.0, 0.20, 12.0),
        "gpt-5.6-luna" => (0.20, 0.02, 1.20),
        "gpt-5.5" => (5.0, 0.50, 30.0),
        "gpt-5.4" => (2.50, 0.25, 15.0),
        "gpt-5.4-mini" => (0.75, 0.075, 4.50),
        _ => return None,
    };
    Some(ModelPrice {
        input_per_million: input,
        output_per_million: output,
        cache_read_multiplier: cached / input,
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
