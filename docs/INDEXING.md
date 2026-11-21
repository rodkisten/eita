# 🦀 Indexação incremental do Eita

## Objetivo

Construir um catálogo local de comandos, subcomandos e flags, com
mínimo I/O durante a digitação. **A cada tecla, o Eita não lê manpages
nem analisa scripts de completions.** Essas fontes são interpretadas
somente no `eita index --refresh` e os candidatos ficam no cache em disco,
carregado uma vez pelo daemon Rust.

## Fontes e prioridade

| Fonte | Modo de obtenção | Segurança |
| --- | --- | --- |
| Zsh `fpath` | Leitura textual de `_arguments` e `commands=(...)` | Nunca executa funções |
| Manpages `.1/.1p/.8` | Leitura roff estática, inclusive gzip | Opt-in, sem `man`/pager |
| Templates aprendidos | Nome/verbos whitelistados no `preexec` | Sem argumentos nem segredos |
| PATH | Apenas nomes de executáveis | Nunca roda `--help` |
| Specs personalizadas | TSV do usuário | Somente dados |

## Comandos de operação

~~~zsh
eita-index                                     # Zsh fpath (auto-exportado pelo plugin)
eita-index --man-dir /usr/share/man            # Adiciona manpages locais
eita index --refresh --fpath /opt/zsh/site-functions --man-dir /usr/share/man
eita index --status                            # Exibe versão e tamanho do cache
eita daemon reload                             # Recarrega o catálogo em RAM
~~~

`--man-dir` aceita caminhos separados por `:`. O indexador procura o
diretório especificado e seus subdiretórios `man1` e `man8`. Use
`EITA_MANPATH` para predefinir fontes locais, mas evite diretórios
não confiáveis.

## Cache versionado

O formato JSON `v2` contém os registros consolidados e um manifesto
de snapshots por arquivo, com `path`, `size`, `modified_ns` e
candidatos desse arquivo. O cache `v1` continua legível para consultas.
A primeira reconstrução converte para `v2`.

Cada refresh faz somente `readdir + stat` de arquivos permitidos.
Arquivos com tamanho e timestamp idênticos reutilizam os candidatos
já analisados; os demais são reprocessados. Um arquivo apagado sai
do catálogo. Quando tudo está inalterado, **não há escrita de disco** e
o daemon não é recarregado.

Exemplo de relatório:

~~~json
{
  "version": 2,
  "scanned_files": 2,
  "reused_files": 2,
  "updated_files": 0,
  "removed_files": 0,
  "unchanged": true
}
~~~

Tamanho máximo do cache: 8 MiB. Fontes individuais têm limite de
256 KiB, manpages descompactadas 512 KiB; número total de arquivos
e candidatos também é limitado para controlar o uso de recursos.

A estratégia `mtime + size` não é um hash criptográfico: alterações
que preservem ambos podem exigir `rm ~/.cache/eita/zsh-completions.json`
seguido de `eita-index`. A versão futura poderá usar hash incremental
e watchers em sistema de arquivos, sem bloquear o prompt.

## Privacidade e limites

- O manifesto é escrito com permissões `0600`.
- O parser de manpages entende somente alguns padrões roff; não tenta
  reproduzir `groff`, mandoc, macros executáveis ou preprocessadores.
- Não importa automaticamente `zsh_history`.
- O motor de sugestões nunca executa uma conclusão selecionada.
- Nem toda flag será encontrada; fontes imprecisas podem exigir uma
  inspeção antes da execução pelo usuário.

## Benchmark

~~~zsh
eita daemon start
eita daemon bench         # p50/p95/p99 pelo socket
eita bench --iterations 500 --compare
eita daemon status        # hits, misses e entradas do cache quente
~~~

O primeiro benchmark mede IPC + matcher aquecido; o segundo compara
algoritmos. Nenhum deles inclui o redraw completo do app SSH no iPhone.
