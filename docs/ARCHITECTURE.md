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

A pipeline [Build & Release](../.github/workflows/release.yml) valida fontes, compila os targets habilitados na matriz, cria archives com SHA-256, anexa artefatos ao workflow e publica GitHub Release por SHA de commit na `main`. O instalador [install.sh](../install.sh) seleciona a arquitetura e exige verificação de checksum. Consulte [docs/RELEASES.md](RELEASES.md).

## Edição contextual ZLE

O `Context` reconhece separadores de comandos fora de aspas e substitui o segmento do comando ativo, preservando texto posterior ao cursor. O widget recebe `LBUFFER` e `RBUFFER`, e o binário devolve `replacement` e `cursor` para que não seja necessário executar nem avaliar nenhum comando sugerido. Expressões complexas não são reescritas enquanto tiverem aspas incompletas.


## Índice estático das completions do Zsh

`src/completion.rs` lê caminhos de `EITA_FPATH` (ou `FPATH`) e considera arquivos cujo nome começa com `_`. A diretiva `#compdef` fornece nomes de comandos; na ausência dela, usa-se o nome do arquivo. O parser extrai somente argumentos literais de `_arguments` e listas simples de subcomandos, sem avaliar funções. Limites de tamanho e quantidade evitam indexação sem fronteiras.

`eita index --refresh` atualiza um único JSON em `XDG_CACHE_HOME`, usando escrita temporária e rename; `eita index --status` exibe métricas sem reconstruir. O loader só lê versões compatíveis e falha silenciosamente para caches ausentes ou inválidos, deixando os comandos embutidos e os executáveis do PATH disponíveis normalmente.

A indexação é opt-in para preservar a velocidade do shell: o widget `eita-index` fornece o `fpath` da sessão e pode ser chamado a qualquer momento. O índice não interpreta scripts, não executa `--help` e pode ignorar specs dinâmicas; a compatibilidade completa com `compinit` permanece um trabalho separado.


## Inline e ranking

`src/inline.rs` calcula sufixos por *prefixo exato*: fuzzy matching pode sugerir
correções no picker, mas o modo ghost só completa caracteres adicionais.
`src/usage.rs` permite registrar, sob opt-in explícito, templates do catálogo,
sem persistir a linha completa. `src/benchmark.rs` compara Nucleo e Frizbee
com as mesmas entradas em memória, excluindo IO, startup e renderização.

O plugin ZLE usa `POSTDISPLAY` e `region_highlight` apenas quando
`EITA_INLINE_ENABLED=1` e o cursor está ao final de `BUFFER`. O sufixo só é
aplicado com Ctrl+X Ctrl+E. Não assume a variável de exibição quando outro
plugin a controla; as callbacks são registradas com `add-zle-hook-widget`,
sem substituir `compinit` ou o Tab. Veja `docs/INLINE.md`.


## Recuperação de comandos inexistentes (Zsh)

O plugin usa `command_not_found_handler` para detectar falhas reais
na resolução de comandos. Coleta somente **nomes**, sem valores, de
executáveis, aliases, funções, builtins, palavras reservadas e parâmetros
do Zsh e envia-os por TSV temporário ao CLI `eita recover`.

`src/recover.rs` valida nomes, aplica distância Optimal String Alignment,
ranqueia as correções e evita sugestões muito distantes. O popup Ratatui
é renderizado na viewport inline **antes do próximo prompt**.
Ao selecionar, o plugin chama `print -z` para preparar o comando
corrigido como texto editável, **sem executá-lo**. O status permanece 127.

Se um handler anterior existia, ele é restaurado com `eita-recover-off`.
O plugin não mostra o popup em sessões não interativas ou sem TTY.
Variáveis requerem a consulta explícita `eita-what ... variable`,
porque expansões de variáveis desconhecidas não ativam o handler.


## Configuração e renderização compacta

`src/config.rs` lê `config.toml` via Serde/TOML, valida intervalos, gera
um arquivo padrão e exporta um conjunto fechado de pares TSV para o
plugin Zsh. Nenhum `eval` é necessário. `src/ui.rs` desenha uma caixa
com `popup.width`, `popup.align` e paleta pré-definida dentro do
`Viewport::Inline`: a área **externa continua usando a largura total
do terminal**. `position` alinha dentro dessa área, não em relação
às coordenadas absolutas do cursor. Um renderizador ancorado ao
ZLE e um processo de sugestões persistente estão descritos em
`docs/DAEMON.md`.


## Motor persistente e cache

`src/daemon.rs` hospeda um serviço opcional em socket Unix de uso local,
com catálogo carregado uma vez, limite de 256 respostas em cache,
timeouts e recarga por alteração dos arquivos de completions/aprendizado.
O ZLE usa `zsocket` e `zselect` para consultar sem inicializar Rust a
cada tecla (fallback CLI em caso de erro). `src/usage.rs` mantém somente
templates aprovados, com bônus de frequência limitado e sem persistir
argumentos. Veja `docs/DAEMON.md`.
