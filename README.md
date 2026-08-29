# Gastei quanto?

Aplicativo local de bandeja para estimar o custo de uso de APIs de modelos,
atribuído por provedor, modelo, projeto e sessão. O MVP não lê credenciais nem
faz requisições de rede: reprocessa os logs locais do Claude Code e do Codex e marca todo
valor como `estimated`.

## Estado

Este repositório implementa a primeira fatia vertical do plano em
`/home/gaalbu/Documents/ideias-projetos/gastei-quanto.md`: Tauri v2, interface
TypeScript e núcleo Rust. Cost APIs, orçamento e proxy ficam para
fases posteriores.

## Desenvolvimento

```bash
npm install
npm run dev
npm run build
npm run tauri dev
```

O desenvolvimento nativo exige Rust e os pré-requisitos do Tauri para o sistema
operacional. O CI produz artefatos para Ubuntu, Windows e macOS.

## Princípios

- estimativa e custo faturado nunca são misturados;
- modelos sem preço ficam visíveis e não entram no total;
- o painel separa Claude Code e Codex por provedor;
- os logs do Codex usam as tarifas Standard oficiais como estimativa: os
  arquivos locais não informam modalidade, região ou autenticação;
- a data da tabela de preços acompanha todo total exibido;
- nenhuma credencial é lida, persistida ou enviada pela versão local.

## Arquitetura

Ver [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para pipeline Collect→Parse→Deduplicate→Price→Report, schema de logs e detalhes de tray.

## Tabela de preços

`src-tauri/pricing-tables.json` (versionada por data). Para atualizar: edite o JSON, rode `cargo test` (valida `14.70` para 4M tokens e fixtures), e versione com git. O binário embute o arquivo via `include_str!`.

## Troubleshooting

- Pastas `~/.claude/projects` ou `~/.codex/sessions` ausentes: o app retorna total 0 sem erro.
- Timestamp inválido: logado em stderr e evento ignorado.
- Modelo sem preço: aparece em "Sem preço" com contagem de tokens, fora do total.

## Roadmap

Fase 1 (este MVP): logs locais, estimativa, bandeja.
Fase 2: Cost APIs reais, orçamento, proxy opcional.

## Licença

Este projeto está licenciado sob a [MIT License](LICENSE).
