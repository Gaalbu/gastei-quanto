mod pricing;
mod claude_logs;

use claude_logs::{add_line, new_report, Report};
use chrono::Local;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

#[tauri::command]
fn current_month_report(month: String) -> Result<Report, String> {
    if month.len() != 7 || month.as_bytes().get(4) != Some(&b'-') {
        return Err("month must use YYYY-MM format".to_string());
    }

    report_for_month(&month)
}

fn report_for_month(month: &str) -> Result<Report, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "home directory is unavailable".to_string())?;
    let log_root = home.join(".claude").join("projects");
    let mut report = new_report();
    let mut seen = HashSet::new();
    collect_jsonl(&log_root, &mut report, &mut seen, month).map_err(|error| error.to_string())?;
    Ok(report)
}

fn collect_jsonl(
    root: &Path,
    report: &mut Report,
    seen: &mut HashSet<(String, String)>,
    month: &str,
) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_jsonl(&path, report, seen, month)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            let file = BufReader::new(File::open(path)?);
            for line in file.lines() {
                add_line(report, seen, &line?, month);
            }
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
            cache_write_5m: None,
            cache_write_1h: None,
            reasoning: 0,
        };

        let cost = calculate_cost("claude-sonnet-5", usage).expect("known model");

        assert_eq!(cost.amount, 14.7);
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
            cache_write_5m: None,
            cache_write_1h: None,
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
        .setup(|app| {
            let open = MenuItemBuilder::with_id("open", "Abrir painel").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Sair").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&open, &quit]).build()?;
            let tray_result = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Gastei quanto?")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app);
            let Ok(tray) = tray_result else {
                let error = tray_result.err().expect("tray result already checked");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_title("Gastei quanto? — bandeja indisponível");
                    let _ = window.show();
                }
                eprintln!("tray unavailable; using the main window: {error}");
                return Ok(());
            };
            let refresh_tray = tray.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let month = Local::now().format("%Y-%m").to_string();
                    if let Ok(report) = report_for_month(&month) {
                        let title = format!("US$ {:.0}", report.total);
                        let _ = refresh_tray.set_title(Some(&title));
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(900)).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
