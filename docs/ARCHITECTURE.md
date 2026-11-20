# Arquitetura do Eita

## Componentes

1. **Adaptador ZLE**: widget opt-in que lê `BUFFER`, entrega a linha ao seletor e reposiciona `CURSOR` na escolha. Não executa sugestões.
2. **CLI**: comandos `suggest`, `pick` e `bench`.
3. **Catálogo**: lista interna, comandos encontrados no PATH sem execução e TSV do usuário.
4. **Engine**: identifica o primeiro token, restringe candidatos à família do comando quando possível, usa Frizbee e ajusta ranking para prefixo e matches exatos.
5. **TUI**: Ratatui + Crossterm com viewport inline de 10 linhas.

## Segurança e limites

- Sugestões são apenas texto; nada é executado automaticamente.
- O plugin não intercepta cada tecla nem substitui `compinit`.
- O seletor atualmente altera o `BUFFER` inteiro. A preservação do trecho à direita do cursor é uma melhoria obrigatória antes de ativar sugestões automáticas.
- Descrições de man pages e completions reais ainda não são coletadas.
- `bench` não inclui startup, coleta do PATH, I/O do ZLE e renderização.

## Próximos blocos

### P1: contexto do cursor

Lexer shell-aware que identifica tokens, aspas, pipes, escapes e a palavra atual. Preservar o sufixo do buffer e completar só o intervalo de edição.

### P2: fontes de conhecimento

Coletar aliases, subcomandos e completions Zsh sob demanda, indexar man pages e `--help` com política segura e cache persistente.

### P3: experiência inline

Texto fantasma via widget ZLE, popup Ratatui opt-in para lista e integração sem disputar o cursor com o próprio Zsh.

### P4: desempenho

Datasets congelados, cold/warm start, p50/p95/p99, memória e qualidade do ranking comparando Frizbee, Nucleo e FZF.

## Distribuição

A pipeline [Build & Release](../.github/workflows/release.yml) valida fontes, compila quatro targets nativos, cria archives com SHA-256, anexa artefatos ao workflow e publica GitHub Release por SHA de commit na `main`. O instalador [install.sh](../install.sh) seleciona a arquitetura e exige verificação de checksum. Consulte [docs/RELEASES.md](RELEASES.md).
