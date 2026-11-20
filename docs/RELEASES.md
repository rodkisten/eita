# 📦 Builds, artefatos e GitHub Releases

## Quando acontece?

O workflow [📦 Eita · Build & Release](../.github/workflows/release.yml) é executado:

- Após **todo push na branch `main`**, sem filtro de caminhos. Até um commit apenas no README gera release.
- Manualmente em **Actions → 📦 Eita · Build & Release → Run workflow → main**.

**Importante:** o workflow só publica a partir da `main`. Execuções manuais com outra branch selecionada são ignoradas por segurança. Releases estáveis são publicadas apenas a partir da `main`.

## Plataformas e artefatos

| Arquivo | Runner | Sistema |
| --- | --- | --- |
| `eita-x86_64-unknown-linux-gnu.tar.gz` | `ubuntu-24.04` | Linux x86-64 |
| `eita-aarch64-unknown-linux-gnu.tar.gz` | `ubuntu-24.04-arm` | Linux ARM64 |
| `eita-x86_64-apple-darwin.tar.gz` | `macos-15-intel` | macOS Intel |
| `eita-aarch64-apple-darwin.tar.gz` | `macos-15` | macOS Apple Silicon |

Cada arquivo `.tar.gz` contém `eita`, `README.md`, `LICENSE` e `integrations/zsh/eita.plugin.zsh`. Cada archive possui um `.tar.gz.sha256` correspondente.

O workflow sobe os pacotes como **Actions artifacts** (retenção de 30 dias) e publica os pares arquivo/checksum dos targets habilitados no GitHub Release, acessível em [Releases](https://github.com/rodkisten/eita/releases). Linux usa o target GNU da distribuição e pode exigir uma versão compatível de glibc. Compatibilidade com musl/Alpine não está garantida.

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


## 🔗 Builds de PR e links curtos

A pipeline também roda em PRs e publica uma **prerelease mutável** `pr-<número>` quando todos os builds passam. O PR recebe ou atualiza um único comentário contendo o instalador e os links de download.

~~~sh
# Estável da main
curl -fsSL https://shell.migos.club/install/eita | sh

# Build mais recente do PR #2
curl -fsSL https://shell.migos.club/install/eita/pr/2 | sh

# Alternativa sem domínio próprio (após o preview existir)
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | EITA_VERSION=pr-2 sh
~~~

Para baixar o pacote diretamente com um link estável de PR, sem executar nada:

~~~sh
curl -fL https://shell.migos.club/install/eita/pr/3/artifact -o eita.tar.gz
curl -fL https://shell.migos.club/install/eita/pr/3/checksum -o eita.tar.gz.sha256
~~~

O link aceita `?target=aarch64-unknown-linux-gnu` (ou uma das arquiteturas disponibilizadas pela matriz); binários ausentes retornarão 404 no GitHub.

A rota curta `/install/eita` é implementada pelo Cloudflare Worker do repositório `rodkisten/shell`; a variante `/pr/N` prefixa a variável de ambiente `EITA_VERSION=pr-N` antes de executar o instalador oficial. Os previews são públicos, **não são promovidos à release estável**, e são apagados quando o PR fecha.

Os arquivos comuns do GitHub Actions continuam disponíveis durante o período de retenção, mas não são usados como URL pública estável porque exigem autenticação ou expiram. Publicar como prerelease torna o download compatível com `curl`.

**Cobertura de plataformas:** o workflow distribui somente os targets atualmente habilitados na matriz de `release.yml`. Adicionar um novo target à matriz passa a produzir seu artefato de forma automática; a validação compara a quantidade de arquivos e checksums antes de publicar.

## Segurança e validação

- Cargo fmt, Clippy e testes devem passar antes da compilação dos quatro targets.
- Os jobs que publicam releases da `main` e previews usam `contents: write`; o preview aceita somente PRs originados do mesmo repositório.
- A publicação exige um arquivo SHA-256 válido para cada tarball dos targets efetivamente habilitados.
- O instalador não depende de Rust, gh CLI, jq ou sudo.
- A interface interativa depende de Zsh e terminal compatível; o binário CLI pode ser chamado diretamente.


## 🛠️ Build Linux portátil para Debian

O arquivo Linux x64 agora usa o target `x86_64-unknown-linux-musl`.
A pipeline instala `musl-tools` e compila com Rust para Musl,
eliminando a dependência da GLIBC do sistema.
O smoke test verifica a ausência do segmento `INTERP` no ELF
e executa o binário no runner.

O instalador Linux escolhe o archive Musl e deve encerrar com erro
se a execução de `eita --version` falhar. Prévia do PR:
execute o instalador da branch que contém essa mudança, não o
`install.sh` ainda antigo na `main`.
