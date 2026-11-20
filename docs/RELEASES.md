# 📦 Builds, artefatos e GitHub Releases

## Quando acontece?

O workflow [📦 Eita · Build & Release](../.github/workflows/release.yml) é executado:

- Após **todo push na branch `main`**, sem filtro de caminhos. Até um commit apenas no README gera release.
- Manualmente em **Actions → 📦 Eita · Build & Release → Run workflow → main**.

**Importante:** o workflow só publica a partir da `main`. Execuções manuais com outra branch selecionada são ignoradas por segurança. A primeira release surge depois que esta alteração for mergeada na `main`.

## Plataformas e artefatos

| Arquivo | Runner | Sistema |
| --- | --- | --- |
| `eita-x86_64-unknown-linux-gnu.tar.gz` | `ubuntu-24.04` | Linux x86-64 |
| `eita-aarch64-unknown-linux-gnu.tar.gz` | `ubuntu-24.04-arm` | Linux ARM64 |
| `eita-x86_64-apple-darwin.tar.gz` | `macos-15-intel` | macOS Intel |
| `eita-aarch64-apple-darwin.tar.gz` | `macos-15` | macOS Apple Silicon |

Cada arquivo `.tar.gz` contém `eita`, `README.md`, `LICENSE` e `integrations/zsh/eita.plugin.zsh`. Cada archive possui um `.tar.gz.sha256` correspondente.

O workflow sobe os pacotes como **Actions artifacts** (retenção de 30 dias) e publica os oito arquivos no GitHub Release, acessível em [Releases](https://github.com/rodkisten/eita/releases). Linux usa o target GNU da distribuição e pode exigir uma versão compatível de glibc. Compatibilidade com musl/Alpine não está garantida.

## Tags e `latest`

Cada SHA gera `main-<12 primeiros caracteres do commit>`. Executar manualmente o mesmo commit reaproveita o release e substitui os assets, sem gerar uma segunda tag. Um release antigo nunca assume o `latest` quando já existe um commit mais novo na main: somente uma execução correspondente ao HEAD atual pode promover o release.

## Instalação

~~~sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | sh
"$HOME/.local/bin/eita" --version
"$HOME/.local/bin/eita" suggest --line 'git comit'
~~~

Para instalar uma versão específica, copie sua tag em [Releases](https://github.com/rodkisten/eita/releases):

~~~sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | EITA_VERSION=main-EXEMPLO sh
~~~

Outras variáveis: `EITA_INSTALL_DIR` (diretório do executável), `EITA_PLUGIN_DIR` (diretório para o plugin). Os defaults são `~/.local/bin` e `${XDG_DATA_HOME:-$HOME/.local/share}/eita`. O instalador verifica SHA-256, usa HTTPS e falha quando o archive não coincide com seu checksum. Não altera `PATH` ou `.zshrc` automaticamente.

## Segurança e validação

- Cargo fmt, Clippy e testes devem passar antes da compilação dos quatro targets.
- O workflow de release só tem `contents: write` no job final que publica os assets.
- A publicação exige quatro tarballs e quatro checksums válidos.
- O instalador não depende de Rust, gh CLI, jq ou sudo.
- A interface interativa depende de Zsh e terminal compatível; o binário CLI pode ser chamado diretamente.
