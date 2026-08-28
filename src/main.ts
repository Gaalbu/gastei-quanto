import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Tokens = { input: number; output: number; cache_read: number; cache_write: number };
type Breakdown = { amount: number; tokens: Tokens };
type Report = { total: number; basis: string; price_table_version: string; by_model: Record<string, Breakdown>; by_project: Record<string, Breakdown>; unpriced_models: Record<string, Tokens> };
const month = new Date().toISOString().slice(0, 7);
const money = (amount: number) => `US$ ${amount.toFixed(2)}`;
const count = (value: number) => value.toLocaleString("pt-BR");
const escapeHtml = (value: string) => value.replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character] ?? character);
function rows(items: Record<string, Breakdown>) { return Object.entries(items).sort(([, a], [, b]) => b.amount - a.amount).map(([name, item]) => `<li><span>${escapeHtml(name)}</span><strong>${money(item.amount)}</strong></li>`).join("") || '<li class="muted">Nenhum evento precificado neste mês</li>'; }
function unknownRows(items: Record<string, Tokens>) { return Object.entries(items).map(([name, item]) => `<li><span>${escapeHtml(name)}</span><strong>${count(item.input + item.output + item.cache_read + item.cache_write)} tokens</strong></li>`).join("") || '<li class="muted">Nenhum</li>'; }
function render(report: Report) {
  document.querySelector<HTMLDivElement>("#app")!.innerHTML = `<main class="panel"><header><div><p class="eyebrow">GASTEI QUANTO?</p><h1>Este mês</h1><p class="period">${month}</p></div><span class="badge">estimativa</span></header><section class="hero"><p>Total estimado</p><strong>${money(report.total)}</strong><small>Somente logs locais do Claude Code</small></section><div class="meta"><span>Tabela ${escapeHtml(report.price_table_version)}</span><span>Base: ${report.basis}</span></div><section class="grid"><article><h2>Por modelo</h2><ul>${rows(report.by_model)}</ul></article><article><h2>Por projeto</h2><ul>${rows(report.by_project)}</ul></article></section><section class="unknown"><h2>Sem preço</h2><p>Tokens preservados fora do total até a tabela conhecer o modelo.</p><ul>${unknownRows(report.unpriced_models)}</ul></section></main>`;
}
async function load() { try { render(await invoke<Report>("current_month_report", { month })); } catch (error) { document.querySelector<HTMLDivElement>("#app")!.innerHTML = `<main class="panel error"><p class="eyebrow">GASTEI QUANTO?</p><h1>Não foi possível ler os logs</h1><p>${escapeHtml(String(error))}</p><small>Confira se a pasta ~/.claude/projects existe e tente novamente.</small></main>`; } }
load();
