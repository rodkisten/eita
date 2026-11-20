# Arquitetura do Eita

## Componentes

1. **Adaptador ZLE**: widget opt-in que lê `LBUFFER` e `RBUFFER`, consulta aliases nativos e envia buffers ao seletor. A resposta contém texto completo e posição de `CURSOR` em caracteres, sem executar sugestões.
2. **CLI**: `suggest --left --right` (ou `--line`), `pick --left --right --output --cursor-output` e `bench`.
3. **Catálogo**: lista interna, comandos encontrados no PATH sem execução e TSV do usuário.
4. **Context**: parser conservador de segmentos Zsh (`;`, `|`, `&` e newline), com estado de aspas/escapes; preserva comandos anteriores e a parte à direita do cursor. A engine identifica raiz do comando, restringe candidatos à família quando possível, usa Frizbee e ajusta ranking de prefixos e matches exatos.
5. **TUI**: Ratatui + Crossterm com viewport inline de 10 linhas.

## Segurança e limites

- Sugestões são apenas texto; nada é executado automaticamente.
- O plugin não intercepta cada tecla nem substitui `compinit`.
- O seletor aplica a substituição ao segmento sob o cursor, preservando os demais segmentos do buffer e reconstruindo a posição de cursor em caracteres. Strings com aspas não fechadas e escapes pendentes não são editadas.
- O parser é conservador e **ainda não interpreta completamente** substituições complexas de comando, here-docs, redirecionamentos e expansões aninhadas; essas situações exigirão um lexer mais completo antes de sugestões automáticas.
- Descrições de man pages e completions reais ainda não são coletadas.
- `bench` não inclui startup, coleta do PATH, I/O do ZLE e renderização.

## Próximos blocos

### P1: contexto do cursor

Primeira versão concluída para segmentos simples, aspas, pipes, escapes e posição do cursor. Próximo passo: lexer completo para expansões, redirecionamentos, heredocs e cursor em qualquer contexto aninhado.

### P2: fontes de conhecimento

Coletar aliases, subcomandos e completions Zsh sob demanda, indexar man pages e `--help` com política segura e cache persistente.

### P3: experiência inline

Texto fantasma via widget ZLE, popup Ratatui opt-in para lista e integração sem disputar o cursor com o próprio Zsh.

### P4: desempenho

Datasets congelados, cold/warm start, p50/p95/p99, memória e qualidade do ranking comparando Frizbee, Nucleo e FZF.

## Distribuição

A pipeline [Build & Release](../.github/workflows/release.yml) valida fontes, compila quatro targets nativos, cria archives com SHA-256, anexa artefatos ao workflow e publica GitHub Release por SHA de commit na `main`. O instalador [install.sh](../install.sh) seleciona a arquitetura e exige verificação de checksum. Consulte [docs/RELEASES.md](RELEASES.md).
