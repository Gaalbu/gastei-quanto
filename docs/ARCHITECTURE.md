# Architecture

## Overview
Tauri v2 app: TypeScript frontend (Vite) + Rust backend. Offline-first, lê JSONL locais, estima custo, exibe painel.

## Frontend (TypeScript)
- `src/main.ts`: helper `element<K>` cria DOM via `textContent` (XSS-safe), renderiza `Report` em hero + 3 cards + seção unpriced.
- Ordenação no frontend (`by_provider/model/project` por `amount` desc).
- Hero clicável copia total pro clipboard.
- `src/styles.css`: design tokens em `:root` (`--color-hero-bg`, `--radius-card`, etc), layout `.grid` 2-col → 1-col @600px.

## Backend (Rust)
Pipeline:
1. **Collect** (`lib.rs:collect_jsonl` + `codex_logs::collect_jsonl`): varre `~/.claude/projects` e `~/.codex/sessions` recursivamente por `*.jsonl`.
2. **Parse**:
   - Claude: `claude_logs::parse_event` extrai `model`, `usage`, `timestamp`, `cwd`.
   - Codex: `CodexParser` (state: `model`, `project`, `seen`) processa `turn_context` → `token_count`.
3. **Filter**: `is_in_local_month` / `local_month` converte timestamp para `Local` e compara `YYYY-MM`.
4. **Deduplicate**:
   - Claude: `HashSet<(message_id, request_id)>`
   - Codex: `HashSet<Usage>` por `total_token_usage` (fallback `last_token_usage`), reset por `turn_context`.
5. **Price**: `pricing::calculate_cost` lookup em `pricing-tables.json` (via `include_str!` + `OnceLock`), aplica `input/output/cache_read/cache_write` com split 5m/1h para Anthropic; OpenAI usa `cached_per_million / input_per_million` como multiplier unificado.
6. **Report**: agrega `Report { total, by_model, by_project, by_provider, unpriced_models }`.

## Pricing
- `src-tauri/pricing-tables.json` versionado (`anthropic-2026-08-28`, `openai-2026-08-28`). Editar JSON + `cargo test` valida. Histórico via git.
- `pricing.rs` carrega com `OnceLock<PricingTables>`; `anthropic_price` / `openai_price` retornam `ModelPrice`.

## Tray
- `lib.rs:run` cria tray com menu Abrir/Sair; loop async a cada 15min (`tokio::time::sleep(900)`) atualiza título com `US$ {total:.0}` e loga erro em `eprintln!` se falhar.

## Logs schema (exemplos)
- Claude: `{"timestamp":"2026-08-28T12:00:00Z","cwd":"/work/app","requestId":"req-1","message":{"id":"msg-1","model":"claude-sonnet-5","usage":{"input_tokens":1}}}` + `cache_creation` variantes.
- Codex: `{"type":"turn_context","payload":{"type":"turn_context","turn_id":"t1","model":"gpt-5.6-luna","cwd":"/work"}}` + `{"type":"event_msg","timestamp":"...","payload":{"type":"token_count","info":{"last_token_usage":{...}}}}`.

## Threading
- Tauri command `current_month_report` síncrono (IO bloqueante ok para MVP). Tray usa `tauri::async_runtime::spawn`.

## Troubleshooting
- `~/.claude/projects` ou `~/.codex/sessions` inexistente → `collect_jsonl` retorna `Ok(())` (sem erro).
- Timestamp inválido → `eprintln!` + evento ignorado (não entra no total).
- Model sem preço → vai para `unpriced_models`, preservado visível.
