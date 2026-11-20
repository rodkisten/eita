# ⚡ Arquitetura proposta: motor persistente Eita

## Situação atual (não confundir com um serviço já funcionando)

- O cache de completions é **JSON em disco**: `eita index` lê specs
  estáticas no `fpath` e escreve `$XDG_CACHE_HOME/eita/zsh-completions.json`.
- O ranking opt-in tem um JSON privado separado, e o ZLE guarda o
  último texto fantasma apenas **em memória da própria sessão**.
- O `eita inline` ainda inicia um processo Rust por novo buffer de edição.
- **Ainda não há daemon**, socket Unix, process manager ou cache global
  em memória para todas as sessões.

## Próximo bloco de implementação

`eita daemon start|stop|status|reload`, no Linux e macOS,
com Unix Domain Socket protegido em `$XDG_RUNTIME_DIR/eita/engine.sock`
(ou diretório exclusivo do usuário com modo 0700). Comunicação
JSON Lines com tamanho máximo por mensagem e timeout rigoroso.

O processo único mantém um índice em memória:
`PATH` + completions estáticas + templates de usuário.
Não executa `--help` nem funções do Zsh. Cada sessão Zsh continua
responsável por enviar somente os **nomes** locais de aliases,
funções e variáveis, nunca seus valores ou os argumentos de execução.
O daemon combina o índice global com nomes da sessão **por request**,
sem compartilhar símbolos privados entre shells diferentes.

O daemon é **opt-in**, inicia sob demanda, possui tempo ocioso
configurável e desliga sem deixar sockets órfãos. O cliente sempre
funciona sem ele. A invalidação do cache observará mudanças do
`PATH`, schema e arquivos de completion e oferecerá `reload` explícito.
Sem APIs de rede, telemetria ou acesso externo.

### Critérios de aceite

1. `eita daemon status` mostra PID, uptime, tamanho do índice, cache hits
   e latência p50/p95.
2. `eita inline` responde via socket quando saudável, com fallback
   transparente ao processo local caso contrário.
3. O processo rejeita clientes indevidos; diretório/socket só acessíveis
   pelo usuário local. Limites de memória, tempo e tamanho das mensagens.
4. Testes com duas sessões Zsh diferentes não vazam variáveis,
   funções ou aliases de uma para a outra.
5. Reinstalações de previews preservam a compatibilidade do protocolo
   ou reiniciam o daemon de forma controlada.
6. Benchmarks medem **end-to-end** (startup, IPC, scoring e renderização),
   não apenas o matcher.

## Popup acima/abaixo do cursor

O Ratatui permanece como widget de lista, mas o renderizador terá de
cooperar com o ciclo de vida do ZLE. A estratégia precisa considerar
PTY/SSH, cursor, scrollback, teclado iOS e resize da janela.
Não vamos usar movimentos ANSI que sobrescrevem conteúdo do terminal
sem restauração garantida. `Viewport::Inline` continua como fallback
seguro, inclusive se o daemon estiver desligado.
