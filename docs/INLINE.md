# 🦀 Eita inline + ranking local + benchmark

## Como ativar e testar

Este bloco adiciona um modo **opt-in** de sugestões cinzas no próprio Zsh, além do
picker Ratatui que já existia. O modo inline usa a variável ZLE `POSTDISPLAY`
para mostrar um sufixo *visual* após o cursor e não altera `BUFFER` até você
aceitar explicitamente a sugestão.

~~~zsh
# Depois de instalar a build do PR:
export PATH="$HOME/.local/bin:$PATH"
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-index
eita-inline-on

# Digite: git com
# O Eita pode mostrar o restante de "git commit".
# Ctrl+X Ctrl+E aceita a sugestão
# Ctrl+Espaço ou Ctrl+X Ctrl+P abre o Ratatui
eita-inline-off
~~~

**Tab e setas** não são sobrescritos. O Eita usa `add-zle-hook-widget`
(`line-pre-redraw` e `line-finish`), que compõe com outros plugins modernos;
quando outro widget já controla `POSTDISPLAY`, o Eita não assume esse espaço.

O modo inline permanece desligado por padrão porque inicia um processo Rust por
buffer novo. Limites padrão: 3 a 400 caracteres de entrada e 160 caracteres
de sugestão. Duplicatas de redesenho reutilizam o último sufixo.
O motor inline pula a varredura de `PATH` a cada tecla, mas carrega o cache de
completions Zsh. Não há daemon, nem solicitação de rede.

### Opções

| Variável | Padrão | Uso |
| --- | --- | --- |
| `EITA_INLINE_ENABLED` | `0` | `eita-inline-on` ou `eita-inline-off` |
| `EITA_INLINE_MIN_CHARS` | `3` | Mínimo de caracteres antes de sugerir |
| `EITA_INLINE_COLOR` | `8` | Cor de realce ZLE (paleta terminal) |
| `EITA_COMPLETION_CACHE` | XDG | Local do cache das completions Zsh |
| `EITA_USAGE_ENABLED` | `0` | Ranking local opcional |
| `EITA_USAGE_FILE` | XDG | Local do JSON de frequência, privado |

### Modo CLI para depuração

~~~sh
eita inline --left "git com"
eita suggest --line "git com" --format json
~~~

`inline` imprime **somente o sufixo adicionável**, nunca correções de typo.
Por exemplo, `got` pode ser corrigido pelo picker, mas não vira texto fantasma,
porque a correção substituiria caracteres digitados.

## 🧠 Ranking local opcional

O Eita registra apenas templates reconhecidos do próprio catálogo, sem persistir
o comando bruto e seus argumentos. `git commit -m "senha"` registra somente
`git commit`. Nada é registrado por padrão.

~~~zsh
eita-usage-on         # habilita hook preexec da sessão
eita-usage-off        # desabilita o hook
eita usage status     # mostra até 50 templates mais frequentes
eita usage clear      # apaga o arquivo de histórico local
~~~

Também é possível registrar uma amostra manualmente:

~~~sh
EITA_USAGE_ENABLED=1 eita usage record --line "git commit -m demo"
EITA_USAGE_ENABLED=1 eita usage status
~~~

O ranking influencia apenas a ordem entre candidatos que passaram pelo fuzzy
matcher; a bonificação por frequência é limitada. Os dados ficam no dispositivo
e podem ser apagados. Nunca são enviados à API do GitHub.

## ⚡ Comparação Frizbee vs Nucleo

~~~sh
eita bench --iterations 1000 --compare
~~~

Gera JSON com p50, p95, p99 e quantidade de matches por motor. Ambos usam as
mesmas strings em memória, os mesmos padrões e 20 iterações de aquecimento;
a medição inclui a criação dos padrões e busca, mas **não** inclui startup,
indexação do catálogo, IO do shell, TUI e custo de subir o binário.
A semântica dos algoritmos é diferente (Nucleo não corrige todos os mesmos
typos de Frizbee), portanto contagem de resultados e latência devem ser
interpretadas conjuntamente. Não comparar pontuações brutas entre motores.

## 🛡️ Segurança e compatibilidade

- O modo inline não executa comandos nem realiza autoaceitação.
- Com cursor no meio da linha, o ghost text não aparece; use o picker Ratatui.
- O modo inline não substitui `compinit`, highlighting, Tab nem setas.
- O ranking é explicitamente opt-in e limita o que é persistido.
- O preview pode ser revertido com `eita-inline-off` e `eita-usage-off`.
- Todos os comandos de diagnóstico funcionam mesmo sem o modo inline ativo.


## 📱 Popup real versus texto fantasma

O comando `eita inline --left "git com"` só retorna `mit`; não abre
uma interface. Para abrir o seletor Ratatui no terminal iOS:

~~~sh
eita popup --line "git com"
~~~

Ou use `eita-popup "git com"` após `source` do plugin no Zsh.
Pressione Ctrl+X e depois Ctrl+P dentro do prompt para abrir o
seletor sobre a linha atual. As teclas de navegação do popup são
setas, Enter e Esc. O menu possui modo compacto com `--height 8`.

O modo inline e o seletor interativo são **experiências distintas**:
`eita-inline-on` não abre o popup a cada tecla. Um popup que siga
automaticamente o cursor precisa de uma integração com controle
coordenado da TUI e do ZLE; iniciar Ratatui a cada tecla bloquearia
a própria edição da linha.
