mod pricing;
mod claude_logs;

use claude_logs::{aggregate, Report};
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
fn current_month_report(month: String) -> Result<Report, String> {
    if month.len() != 7 || month.as_bytes().get(4) != Some(&b'-') {
        return Err("month must use YYYY-MM format".to_string());
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "home directory is unavailable".to_string())?;
    let log_root = home.join(".claude").join("projects");
    let mut lines = Vec::new();
    collect_jsonl(&log_root, &mut lines).map_err(|error| error.to_string())?;
    Ok(aggregate(lines.iter().map(String::as_str), &month))
}

fn collect_jsonl(root: &Path, lines: &mut Vec<String>) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_jsonl(&path, lines)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            lines.extend(fs::read_to_string(path)?.lines().map(str::to_owned));
        }
    }
    Ok(())
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
        .invoke_handler(tauri::generate_handler![current_month_report])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
