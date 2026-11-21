# ⚡ Eita Engine · aprendizado e cache quente

O Eita possui agora um **daemon Rust opcional** por usuário. Ele mantém o catálogo
de candidatos em memória, atende consultas em um **Unix Domain Socket** local e
armazena no máximo 256 resultados recentes de sugestões. O processo não usa TCP,
HTTP, banco de dados, IA em nuvem ou telemetria.

## Operações

~~~zsh
eita daemon start    # inicia como processo independente, se necessário
eita daemon status   # PID, uptime, entradas, cache hits/misses, socket
eita daemon reload   # recarrega PATH, specs estáticas e templates aprendidos
eita daemon bench    # 200 consultas end-to-end, p50/p95/p99 em microssegundos
eita daemon stop     # encerra o serviço local
eita daemon socket   # caminho do socket Unix
~~~

No Zsh, após carregar o plugin:

~~~zsh
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-daemon-on
eita-inline-on
eita-learn-on
# Digite "git com". O texto fantasma é consultado pelo socket nativo Zsh.
eita-daemon-status
eita-daemon-off   # desliga SOMENTE esta sessão; outras podem usar o daemon
eita daemon stop  # encerra globalmente o daemon do usuário
~~~

O modo de daemon é **opt-in** e não inicia a partir do carregamento do plugin.
Sem daemon, o Eita continua funcionando via CLI normal.

## Caminho quente ultra leve

~~~text
ZLE line-pre-redraw
  │
  ├─ buffer igual ao anterior → reutiliza o sufixo do Zsh
  └─ buffer mudou → zsocket (builtin Zsh)
      │ → "I<TAB>PATH<TAB>LBUFFER\\n"
      └─ engine.sock → catálogo RAM → LRU 256 → Frizbee → sufixo
         │
         ├─ ".<sufixo>" → POSTDISPLAY, sem criar processo adicional
         └─ "!" ou timeout (40ms) → fallback CLI anterior
~~~

O módulo `zsh/net/socket` abre uma conexão Unix por consulta; o módulo
`zsh/zselect` limita o tempo de espera a 40ms. O comando `eita inline`
também pode consultar o daemon via JSON quando `EITA_DAEMON_ENABLED=1`.
O socket nativo elimina a inicialização do binário por tecla, mas não dispensa
conexão nem serialização. Não há promessa de microssegundos antes de medir no
dispositivo real. O benchmark do daemon cobre transporte + matching + serialização,
**não** o tempo de inicialização de um processo nem o redesenho da tela.

## Índices e invalidação

1. **Memória**: catálogo único do processo e cache LRU de 256 chaves.
2. **Disco**: `$XDG_CACHE_HOME/eita/zsh-completions.json` (ou `~/.cache`), 
   criado com `eita index --refresh`, sem interpretar/rodar completions.
3. **Aprendizado**: `$XDG_DATA_HOME/eita/usage.json`, opt-in e privado.
4. O daemon confere a data de modificação dos arquivos de completions e
   frequência no máximo 1 vez por segundo, invalidando a memória quando mudam.
5. `eita daemon reload` invalida imediatamente. Se PATH for diferente do
   ambiente do serviço, o cliente faz fallback local em vez de usar dados errados.

Não existe observação automática de **todas** as alterações nos diretórios
do PATH ou fpath. Reindexe após instalar executáveis ou completions e use
`eita daemon reload` quando necessário.

## Aprendizado seguro de comandos

O Eita **não importa nem salva histórico bruto** do Zsh. A captura opcional
acontece em `preexec`, enviando a linha por **stdin**, sem expô-la como argumento
do processo. Em memória, escolhe apenas templates de comandos conhecidos e
subcomandos de um vocabulário fixo para ferramentas como `git`, `gh`, `cargo`,
`docker`, `kubectl`, `pnpm`, `npm`, `gcloud` e `wrangler`.

Exemplos:

| Digitado | Template que pode ser salvo |
| --- | --- |
| `git commit -m "segredo"` | `git commit` |
| `git worktree add /caminho/privado` | `git worktree` |
| `docker compose up -d` | `docker compose` |
| `ssh host.privado` | nada (não reconhecido) |
| `echo TOKEN=senha` | nada (não reconhecido) |

Templates aprendidos passam a aparecer nas sugestões depois da **segunda
utilização**, e ganham bônus limitado de frequência. O arquivo é legível só
pelo usuário (0600). `eita usage clear` apaga o aprendizado existente;
`eita-learn-off` interrompe novas capturas.

Não são armazenados hosts, caminhos, mensagens de commit, nomes de variáveis
com valores, argumentos livres ou scripts executados.

## Segurança e ciclo de vida

- Socket em `$XDG_RUNTIME_DIR/eita/engine.sock`, quando disponível, senão
  `$XDG_CACHE_HOME/eita/engine.sock`. Suporta `EITA_DAEMON_SOCKET` para teste.
- Diretório 0700, socket 0600, propriedade UID local verificada antes de servir.
- Mensagens limitadas a 4 KiB, timeouts de leitura/escrita 120ms.
- Daemon encerra após 15 minutos sem pedidos, ou com `stop`.
- Qualquer cliente sem socket ou com PATH divergente usa o matcher local.
- Aliases/funções/variáveis específicos da sessão **não são retidos no daemon**.
  O popup de erro continua recebendo esses nomes diretamente do Zsh.

## Teste de integração

~~~bash
export EITA_DAEMON_SOCKET="$HOME/.cache/eita-demo/engine.sock"
export EITA_DAEMON_ENABLED=1
eita daemon start
eita daemon status
eita inline --left 'git com'
eita daemon bench
eita daemon reload
eita daemon stop
~~~

Abra dois shells apontando para o mesmo socket para verificar o cache
compartilhado. Os resultados dependem do hardware, catálogo, SSH, Zsh e
carga do sistema, por isso o CI verifica funcionalidade e isolamento,
não estabelece uma meta fixa de latência.
