mod pricing;
mod claude_logs;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg(test)]
mod tests {
    use super::pricing::{anthropic_price, calculate_cost, TokenUsage};

    #[test]
    fn calculates_input_output_and_cache_at_separate_rates() {
        let usage = TokenUsage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 1_000_000,
            cache_write: 1_000_000,
            reasoning: 0,
        };

        let cost = calculate_cost("claude-sonnet-5", usage).expect("known model");

        assert_eq!(cost.amount, 13.35);
        assert_eq!(cost.basis, "estimated");
        assert_eq!(cost.price_table_version, "anthropic-2026-08-28");
    }

    #[test]
    fn reports_unknown_models_without_cost() {
        let usage = TokenUsage {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
            reasoning: 5,
        };

        assert!(anthropic_price("a-model-not-in-the-table").is_none());
        assert!(calculate_cost("a-model-not-in-the-table", usage).is_none());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
