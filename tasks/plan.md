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

## Fatia 2 — consumo local do Codex

1. Extrair o contrato de relatório compartilhado e acrescentar a quebra por
   provedor sem criar um segundo painel.
2. Adicionar preços OpenAI versionados para `gpt-5.6-sol`, `gpt-5.6-terra`,
   `gpt-5.6-luna`, `gpt-5.5`, `gpt-5.4` e `gpt-5.4-mini`, usando a tarifa
   Standard oficial. Modelos sem preço continuam explícitos e não entram no
   total estimado.
3. Processar `~/.codex/sessions/**/*.jsonl` em streaming. O parser acompanha
   `turn_context`, atribui modelo/projeto e consome apenas eventos
   `event_msg/token_count` com `info` válido.
4. Deduplicar dentro de cada turno pela fotografia cumulativa de
   `total_token_usage`; se ela não existir, usar `last_token_usage`. Somar
   `last_token_usage` de cada evento único, pois um turno pode conter várias
   chamadas de modelo por causa de ferramentas. Reiniciar a deduplicação ao
   mudar de turno.
5. Precificar input não armazenado, cached input, cache write e output. Tokens
   de raciocínio já estão incluídos em output e não serão somados novamente.
6. Unificar Claude e Codex no relatório, no total da bandeja e no painel
   existente, incluindo quebra por provedor e exposição de `gpt-reserve` e
   `codex-auto-review` como modelos sem preço.
7. Cobrir parser, deduplicação, preços, virada de mês local e integração com
   fixtures; executar testes Rust, build web, revisão de qualidade e CI dos
   três sistemas.

### Premissas validadas

- Os logs locais não informam modalidade comercial, região nem autenticação;
  portanto o valor continua sendo uma estimativa pela tarifa Standard, sem
  tentar inferir cobrança de assinatura, Batch, Flex ou Priority.
- `reasoning_output_tokens` é subconjunto de `output_tokens` nos eventos reais.
- Deduplicar uma única vez por `turn_id` descartaria chamadas legítimas. A
  deduplicação é, por isso, escopada ao turno e identifica apenas fotografias
  cumulativas repetidas.

## Fora desta fatia

Cost APIs, projeção, gráfico de 30 dias, orçamento, início automático e
persistência local.
