# 🦀 Eita

Autocomplete contextual para Zsh, escrito em Rust. Combina [Frizbee](https://github.com/saghen/frizbee), com tolerância a erros de digitação, e [Ratatui](https://ratatui.rs/), para uma interface compacta no terminal.

> **Seletor opt-in:** sugestões acionadas por atalho, sem substituir completions nativas nem executar automaticamente o comando escolhido.

## ⚡ Instalação pelo GitHub Release

Depois de publicar a primeira release, instale o binário apropriado para **Linux x64/ARM64** ou **macOS Intel/Apple Silicon** com um único comando:

```sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | sh
```

O instalador detecta a plataforma, baixa o artefato do último release, verifica o **SHA-256**, instala o executável em `~/.local/bin/eita` e o plugin em `~/.local/share/eita/eita.plugin.zsh`. Não precisa instalar Rust.

**Experimente imediatamente, mesmo sem configurar o PATH:**

```sh
"$HOME/.local/bin/eita" suggest --line "git comit --am"
```

Para permitir `eita` como comando global e habilitar o atalho no Zsh, coloque no seu `~/.zshrc`:

```zsh
export PATH="$HOME/.local/bin:$PATH"
source "$HOME/.local/share/eita/eita.plugin.zsh"
```

Depois, abra um novo Zsh e experimente `Ctrl+Espaço`. O plugin é opcional e o instalador não modifica `~/.zshrc` automaticamente.

**Personalização e versão fixa:**

Em um pipeline, as variáveis de instalação devem ser passadas ao processo `sh`:

```sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | EITA_INSTALL_DIR="$HOME/bin" sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | EITA_VERSION=main-COLE_A_TAG sh
```

Para mais detalhes sobre os artefatos, confira [Builds e Releases](docs/RELEASES.md).


### 🔗 Links curtos e previews de pull requests

O [Shell Workbench](https://shell.migos.club) oferece um instalador permanente, sem precisar copiar URLs do GitHub:

~~~sh
curl -fsSL https://shell.migos.club/install/eita | sh
~~~

O build de um PR pode ser instalado usando o número do PR, e o endereço não muda quando novos commits são enviados:

~~~sh
curl -fsSL https://shell.migos.club/install/eita/pr/2 | sh
~~~

Você também pode baixar o arquivo diretamente, sem executar o instalador:

~~~sh
curl -fL https://shell.migos.club/install/eita/pr/3/artifact -o eita-pr-3.tar.gz
curl -fL https://shell.migos.club/install/eita/pr/3/checksum -o eita-pr-3.tar.gz.sha256
~~~

O link do arquivo binário e o link curto são publicados automaticamente em um comentário atualizado do PR quando a compilação e a verificação dos checksums terminam. Os previews são **prereleases** `pr-<número>` e não alteram o `latest` da main. Previews são removidos quando o PR é fechado. A publicação exige um PR interno ao repositório e a permissão `contents: write` do GitHub Actions.

**Nota:** os links curtos dependem também da rota `/install/eita/*` publicada no repositório `rodkisten/shell`. Antes dos merges correspondentes, use o instalador via `raw.githubusercontent.com`.

## Compilar a partir do código-fonte

Requer Rust 1.89+ e Zsh.

~~~sh
cargo build --release
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/eita "$HOME/.local/bin/eita"
source integrations/zsh/eita.plugin.zsh
~~~

O binário precisa estar no seu `PATH` antes de carregar o plugin. Para iniciar o seletor, pressione **Ctrl+Espaço**. Se seu terminal não transmitir essa combinação, configure outra:

~~~zsh
bindkey '^X^P' eita-pick
~~~

Cancelar com Esc preserva a linha de comando. Tab ou Enter apenas preenchem o buffer, sem executá-lo. `Ctrl+X Ctrl+P` funciona como atalho alternativo.

O widget passa `LBUFFER` e `RBUFFER` ao Eita para completar o segmento atual sem apagar comandos anteriores nem o texto à direita do cursor. Ele também fornece aliases do Zsh como dados, sem executá-los.

## CLI

~~~sh
eita suggest --line 'git comit --am'
eita suggest --line 'got' --format lines
eita pick --line 'git com'
eita suggest --left 'echo ok | git com' --right 'it --amend' --format json
eita bench --iterations 1000
~~~

`suggest` imprime JSON com `value`, `description`, `source`, `score`, `replacement` e `cursor`. `pick` aceita `--output /caminho` para escrever a seleção em um arquivo, usado pelo plugin ZLE.

## Fontes do catálogo

O Eita reúne comandos internos, executáveis no `PATH` e comandos personalizados em `~/.config/eita/specs.tsv`. Defina `EITA_SPECS` para ler outro arquivo. Cada linha tem um comando, um TAB literal e uma descrição. Veja [o exemplo](examples/specs.tsv).

A descoberta no `PATH` consulta arquivos executáveis e **não executa nenhum programa**.

## 🧠 Indexação das completions nativas do Zsh

O Eita pode **ler as definições de completion presentes no seu próprio `fpath`**, extrair flags e subcomandos descritos literalmente em `_arguments` e em listas `commands=(...)`, e armazenar um índice local em JSON.

Depois de carregar o plugin Zsh, execute:

~~~zsh
eita-index
# Alternativa: Ctrl+X Ctrl+I no prompt interativo
~~~

Para conferir o cache e forçar uma nova indexação:

~~~sh
eita index --status
eita index --refresh --fpath /usr/share/zsh/functions/Completion
eita suggest --line "git --ver"
~~~

O caminho padrão é `~/.cache/eita/zsh-completions.json` (ou `$XDG_CACHE_HOME/eita/zsh-completions.json`), personalizável com `EITA_COMPLETION_CACHE`. A indexação é **explícita**, para não aumentar a latência do autocomplete a cada tecla. O comando `suggest` carrega o cache automaticamente.

Por segurança, o indexador apenas lê arquivos locais, aplica limites de tamanho, extrai **literais simples** e não executa funções Zsh, aliases, comandos externos, `eval` ou `--help`. As completions dinâmicas, funções aninhadas e especificações muito complexas continuam sendo responsabilidade do sistema nativo de completions do Zsh. Esse recurso é um enriquecimento progressivo, não um substituto para `compinit`.

## ✨ Inline e ranking local

Agora há um **ghost text opt-in** (sem substituir Tab), um ranking opcional por templates
e um benchmark comparativo de Frizbee vs Nucleo.

~~~zsh
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-index
eita-inline-on

# Escreva "git com" no prompt e aceite com Ctrl+X Ctrl+E
# Ctrl+Espaço continua abrindo o seletor Ratatui.
eita-usage-on    # opcional: frequência de templates, nunca argumentos
eita-usage-off
eita-inline-off
~~~

Também pela CLI:

~~~sh
eita inline --left "git com"
eita bench --iterations 1000 --compare
eita usage status
~~~

Os dois modos são desativados por padrão. Veja o [guia de autocomplete inline](docs/INLINE.md)
para opções de cor, hooks ZLE, segurança, limites e exemplos.

## 📱 Abrir o popup no iPhone

`eita inline --left 'git com'` **não é um popup**. Ele apenas retorna o
sufixo `mit` que o ZLE pode mostrar como ghost text depois de `eita-inline-on`.

Para **abrir o menu real do Ratatui** diretamente no terminal HostZera:

~~~bash
"$HOME/.local/bin/eita" popup --line 'git com'
~~~

Ou, com plugin Zsh carregado:

~~~zsh
eita-popup 'git com'
eita-keys
~~~

Com uma linha sendo editada no Zsh, pressione **Ctrl+X, depois Ctrl+P**
ou **Ctrl+Espaço**, se o aplicativo transmitir essa combinação.
Dentro do seletor: ↑/↓ para escolher, Enter para aplicar e Esc para sair.

O popup é **acionado sob demanda**. A ativação `eita-inline-on` mostra
sugestões visuais no prompt, mas não abre um Ratatui modal a cada tecla.
A viewport é compacta (8 linhas, ajustável via `--height 6..12`) para
funcionar melhor com teclado virtual.

### Debian sem GLIBC 2.39

A partir deste bloco, o target Linux x64 distribuído passa a ser
`x86_64-unknown-linux-musl`, com binário estático. O build gerado no Ubuntu
24.04 dependia de símbolos de GLIBC indisponíveis no seu Debian.
O instalador verifica se `eita --version` inicia antes de declarar sucesso.
Ao instalar uma prerelease, baixe o `install.sh` da mesma branch do PR,
porque o instalador antigo da main ainda seleciona o binário GNU.

## 🧠 Correção automática de erros de digitação

Quando você executa `gti status` e o Zsh não encontra `gti`,
o Eita mostra um **popup Ratatui compacto antes do próximo prompt**,
com os comandos mais próximos que existem na sessão atual:

~~~text
zsh: command not found: gti
🦀 Eita: 'gti' não existe. Quis dizer?
  1  git    [command] Executável disponível no PATH
↑↓ selecionar · Enter editar · Esc sair
~~~

O Eita consulta **nomes reais** de executáveis no PATH, aliases, funções,
builtins e palavras reservadas. Os nomes de variáveis também podem ser
verificados explicitamente, mas seus valores nunca são acessados.

Ao escolher uma correção, o Eita coloca o comando e os argumentos originais
com escape no próximo buffer de edição (`print -z`). **Não executa nada
sozinho**. O erro original mantém status **127**.

### Comandos de teste

~~~zsh
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-recover-on        # ativado automaticamente quando carrega o plugin
gti status             # deve abrir o popup após erro
eita-recover-off       # desliga e restaura o handler anterior
eita-what 'EDTIOR' variable
~~~

O handler é instalado automaticamente em sessões Zsh interativas que carregam
o plugin, a menos que você defina `EITA_RECOVER_ENABLED=0` antes do `source`.
Se já existia `command_not_found_handler`, o Eita o preserva para quando não
existem sugestões. Esc mantém o erro 127 sem executar comandos. Em scripts ou terminais sem TTY,
não inicia a interface.

**Variáveis:** um `$EDTIOR` inexistente costuma expandir para vazio e
não dispara o handler de comando. Consulte com `eita-what`.

**Subcomandos:** `git comit` executa `git` normalmente e pode falhar com
outro código. Para esse caso, continue utilizando o picker manual:
`eita popup --line "git comit"`.

### API Rust

~~~sh
eita recover --word gti --symbols-file symbols.tsv --format json
eita recover --word '$EDTIOR' --kind variable --symbols-file symbols.tsv
eita recover --word gti --popup --symbols-file symbols.tsv --height 8
~~~

`symbols.tsv` possui linhas `tipo<TAB>nome<TAB>descrição` e é criado
temporariamente pelo plugin. O ranking usa distância de edição
**Optimal String Alignment** (substituição, inserção, remoção e
transposição adjacente), bônus por tipo, limites conservadores de distância
e remoção de duplicatas. Nunca executa funções, aliases ou comandos
durante a coleta dos nomes.

O Ratatui usa uma viewport inline **entre a mensagem de erro e o próximo
prompt**, preservando o scrollback. Ele não é um overlay gráfico fixo
no topo da tela do iPhone.

## 🎛️ Popup personalizável

~~~sh
eita config init             # cria ~/.config/eita/config.toml
eita config show             # opções efetivas
eita popup --line 'git com'  # caixa visualmente compacta, tema editável
~~~

Configure largura, altura, tema, alinhamento, atalhos e gatilho automático
pelo arquivo TOML. Veja [configuração e atalhos](docs/CONFIGURATION.md).
O serviço de índice em memória ainda é um
[bloco de arquitetura planejado](docs/DAEMON.md), não um daemon já instalado.

## Desenvolvimento

~~~sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
~~~

O subcomando `bench` mede somente o matcher após a construção do catálogo; não inclui inicialização do binário, I/O do shell e renderização. Veja a [arquitetura](docs/ARCHITECTURE.md).

## Roteiro

- [x] CLI e catálogo combinado (builtin, PATH, TSV)
- [x] Busca fuzzy tolerante a pequenos erros
- [x] Seletor Ratatui inline, navegação e cancelamento
- [x] Plugin ZLE opt-in
- [x] CI, smoke tests e benchmark local
- [x] Parsing conservador de cursor, aspas, pipes e argumentos
- [x] Aliases Zsh
- [x] Indexação estática e cache das definições nativas de completion do Zsh
- [x] Texto fantasma opcional no ZLE, com aceitação explícita
- [ ] Menu de sugestões contínuo no próprio prompt
- [x] Correções de comandos inexistentes com popup e símbolos reais do Zsh
- [x] Ranking por frequência de templates locais, opt-in
- [x] Benchmark comparável Frizbee × Nucleo, com dataset idêntico
- [ ] Benchmark CLI do FZF, latência de inicialização e memória

Licença MIT.
