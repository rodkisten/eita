# 🦀 Eita

Autocomplete contextual para Zsh, escrito em Rust. Combina [Frizbee](https://github.com/saghen/frizbee), com tolerância a erros de digitação, e [Ratatui](https://ratatui.rs/), para uma interface compacta no terminal.

> **Protótipo inicial:** sugestões acionadas pelo usuário, sem substituir o sistema nativo de completions do Zsh e sem executar o comando sugerido.

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

Cancelar com Esc preserva a linha de comando. Tab ou Enter apenas preenchem o buffer, sem executá-lo.

## CLI

~~~sh
eita suggest --line 'git comit --am'
eita suggest --line 'got' --format lines
eita pick --line 'git com'
eita bench --iterations 1000
~~~

`suggest` imprime JSON com `value`, `description`, `source` e `score`. `pick` aceita `--output /caminho` para escrever a seleção em um arquivo, usado pelo plugin ZLE.

## Fontes do catálogo

O Eita reúne comandos internos, executáveis no `PATH` e comandos personalizados em `~/.config/eita/specs.tsv`. Defina `EITA_SPECS` para ler outro arquivo. Cada linha tem um comando, um TAB literal e uma descrição. Veja [o exemplo](examples/specs.tsv).

A descoberta no `PATH` consulta arquivos executáveis e **não executa nenhum programa**.

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
- [ ] Parsing de cursor, aspas, pipes e argumentos
- [ ] Completions nativas e aliases Zsh
- [ ] Texto fantasma, menus no próprio prompt e cache persistente
- [ ] Ranking contextual por histórico, opt-in
- [ ] Benchmark reproduzível de Frizbee, Nucleo e FZF

Licença MIT.
