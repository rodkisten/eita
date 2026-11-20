# 🎛️ Configuração do Eita

Configuração local, tipada e **sem eval**, em `~/.config/eita/config.toml`
(respeita `XDG_CONFIG_HOME` ou `EITA_CONFIG`).

~~~sh
eita config init
eita config show
eita config zsh
~~~

`config init` não substitui um arquivo existente sem `--force`.
Reabra o Zsh (ou faça `source` novamente) após modificar os atalhos.
Variáveis `EITA_*` já definidas no ambiente antes de carregar o plugin têm
prioridade sobre as chaves correspondentes do TOML para a sessão Zsh.

## Exemplo completo

~~~toml
[popup]
width = 48
height = 8
max_results = 5
theme = "violet"
align = "left"
position = "auto"

[keys]
popup = "^@"
popup_fallback = "^X^P"
accept = "^X^E"
reindex = "^X^I"

[behavior]
trigger = "on-error"
inline = false
~~~

### Opções

| Configuração | Valores aceitos | Significado |
| --- | --- | --- |
| `popup.width` | 24–160 | Largura **visual** da caixa, limitada pelo terminal |
| `popup.height` | 6–14 | Altura da caixa |
| `popup.max_results` | 1–10 | Quantas sugestões mostrar |
| `popup.theme` | `violet`, `graphite`, `light`, `amber` | Paleta do Ratatui |
| `popup.align` | `left`, `center`, `right` | Alinhamento horizontal |
| `popup.position` | `auto`, `above`, `below` | Alinhamento **dentro da área reservada** |
| `behavior.trigger` | `on-error`, `manual`, `off` | Popup de correção automática, manual ou desligado |
| `behavior.inline` | true/false | Ghost text visual enquanto digita |
| `keys.*` | Sequência `bindkey` do Zsh | Atalhos editáveis |

**Limitação importante:** o modo `Viewport::Inline` do Ratatui
sempre reserva a **largura total do terminal**. O Eita agora desenha
somente uma caixa compacta com largura personalizada. As opções
`above`/`below` mudam o alinhamento **dentro da faixa inline reservada**,
não permitem posicionar um overlay livremente acima do cursor.
Um popup realmente ancorado no cursor requer um renderizador ZLE dedicado.

O modo `on-error` abre o seletor quando o Zsh não encontra um comando,
preservando o status 127 e sem executar a correção. `manual` não instala
a recuperação automática, mas preserva o seletor acionado por atalho.
`off` desativa a recuperação automática; para desligar o ghost text,
defina também `inline = false`. Popup automático a cada tecla **não está
implementado**: o ghost text é o modo existente enquanto digita.

## Teclas no iPhone

`Ctrl+Espaço` pode ser interceptado pelo aplicativo SSH. O fallback
`Ctrl+X` e depois `Ctrl+P` funciona em mais teclados.
Para alterar o atalho, troque a sequência em `[keys]`, reabra o Zsh e
execute `eita-keys` para conferir os bindings instalados.

## Como testar

~~~zsh
eita config init
eita config show
eita config zsh
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-keys
eita popup --line 'git com'
gti status
~~~

Para um teste isolado, sem alterar o arquivo do usuário:

~~~sh
EITA_CONFIG=/tmp/eita-demo.toml eita config init
EITA_CONFIG=/tmp/eita-demo.toml eita config show
~~~
