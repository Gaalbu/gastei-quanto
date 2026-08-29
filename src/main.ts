import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Tokens = { input: number; output: number; cache_read: number; cache_write: number };
type Breakdown = { amount: number; tokens: Tokens };
type Report = {
  total: number;
  basis: string;
  price_table_version: string;
  by_model: Record<string, Breakdown>;
  by_project: Record<string, Breakdown>;
  by_provider: Record<string, Breakdown>;
  unpriced_models: Record<string, Tokens>;
};

const now = new Date();
const month = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;

const money = (amount: number) => `US$ ${amount.toFixed(2)}`;
const count = (value: number) => value.toLocaleString("pt-BR");

// Safe DOM helper: uses textContent (no innerHTML) to avoid XSS.
const element = <K extends keyof HTMLElementTagNameMap>(tag: K, text?: string) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  return node;
};

function sortedEntries<T>(record: Record<string, T>, compare: (a: T, b: T) => number): [string, T][] {
  return Object.entries(record).sort(([, a], [, b]) => compare(a, b));
}

function list(items: Record<string, Breakdown>) {
  const ul = element("ul");
  for (const [name, item] of sortedEntries(items, (a, b) => b.amount - a.amount)) {
    const li = element("li");
    li.append(element("span", name), element("strong", money(item.amount)));
    ul.append(li);
  }
  if (!ul.childElementCount) {
    const empty = element("li", "Nenhum evento precificado neste mês");
    empty.className = "muted";
    ul.append(empty);
  }
  return ul;
}

function unknownList(items: Record<string, Tokens>) {
  const ul = element("ul");
  for (const [name, item] of Object.entries(items)) {
    const total = item.input + item.output + item.cache_read + item.cache_write;
    const li = element("li");
    li.append(element("span", name), element("strong", `${count(total)} tokens`));
    ul.append(li);
  }
  if (!ul.childElementCount) {
    const empty = element("li", "Nenhum");
    empty.className = "muted";
    ul.append(empty);
  }
  return ul;
}

function card(title: string, content: HTMLElement) {
  const article = element("article");
  article.append(element("h2", title), content);
  return article;
}

function render(report: Report) {
  const app = document.querySelector<HTMLDivElement>("#app")!;
  const panel = element("main");
  panel.className = "panel";

  const header = element("header");
  const title = element("div");
  title.append(element("p", "GASTEI QUANTO?"), element("h1", "Este mês"), element("p", month));
  title.firstElementChild!.className = "eyebrow";
  title.lastElementChild!.className = "period";
  header.append(title, element("span", "estimativa"));
  header.lastElementChild!.className = "badge";

  const hero = element("section");
  hero.className = "hero";
  hero.title = "Clique para copiar";
  hero.append(
    element("p", "Total estimado"),
    element("strong", money(report.total)),
    element("small", "Somente logs locais do Claude Code e Codex"),
  );
  hero.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(money(report.total));
      const hint = hero.querySelector("small")!;
      const prev = hint.textContent;
      hint.textContent = "Copiado!";
      setTimeout(() => (hint.textContent = prev), 1200);
    } catch {
      // clipboard may be unavailable
    }
  });

  const meta = element("div");
  meta.className = "meta";
  meta.append(element("span", `Tabela ${report.price_table_version}`), element("span", `Base: ${report.basis}`));

  const grid = element("section");
  grid.className = "grid";
  grid.append(
    card("Por provedor", list(report.by_provider)),
    card("Por modelo", list(report.by_model)),
    card("Por projeto", list(report.by_project)),
  );

  const unknown = element("section");
  unknown.className = "unknown";
  unknown.append(
    element("h2", "Sem preço"),
    element("p", "Tokens preservados fora do total até a tabela conhecer o modelo."),
    unknownList(report.unpriced_models),
  );

  panel.append(header, hero, meta, grid, unknown);
  app.replaceChildren(panel);
}

async function load() {
  try {
    render(await invoke<Report>("current_month_report", { month }));
  } catch (error) {
    const app = document.querySelector<HTMLDivElement>("#app")!;
    const panel = element("main");
    panel.className = "panel error";
    const eyebrow = element("p", "GASTEI QUANTO?");
    eyebrow.className = "eyebrow";
    panel.append(
      eyebrow,
      element("h1", "Não foi possível ler os logs"),
      element("p", String(error)),
      element("small", "Confira se as pastas ~/.claude/projects e ~/.codex/sessions existem e tente novamente."),
    );
    app.replaceChildren(panel);
  }
}

load();
