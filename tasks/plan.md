# Plano de implementação

## Fatia 1 — MVP Anthropic local

1. Base Tauri v2, comandos de desenvolvimento, documentação e CI.
2. Contrato de dados, tabela versionada de preços Anthropic e cálculo de custo.
3. Parser tolerante dos JSONL do Claude Code, com atribuição de projeto,
   sessão e branch.
4. Comando Rust que encontra os logs locais e retorna um relatório JSON
   agregado, sem rede e sem credenciais.
5. Painel TypeScript com total mensal, modelo, projeto, estimativa e data da
   tabela.
6. Bandeja Tauri e fallback explícito para janela quando o ambiente não oferece
   bandeja.
7. Fixtures, testes, documentação operacional e instaladores multiplataforma.

Cada etapa deve passar pelos testes focados, `npm run build` quando aplicável,
`git diff --check` e uma revisão de qualidade antes de ser commitada e enviada.

## Fora desta fatia

Codex CLI, cost APIs, projeção, gráfico de 30 dias, orçamento, início automático
e persistência local.
