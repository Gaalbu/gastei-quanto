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

## Licença

A definir antes da primeira distribuição pública.
