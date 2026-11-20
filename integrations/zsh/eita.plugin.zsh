# Eita: opt-in cursor-aware Zsh Line Editor completion.
# No suggestions are executed; works independently from compinit and syntax highlighting.
[[ -o interactive ]] || return 0
(( $+commands[eita] )) || return 0


# Typed configuration is read once at startup. Do not eval TOML or shell code.
_eita_load_settings() {
  emulate -L zsh
  local key value
  while IFS=$'\t' read -r key value; do
    case "$key" in
      EITA_POPUP_WIDTH|EITA_POPUP_HEIGHT|EITA_POPUP_THEME|EITA_POPUP_ALIGN|EITA_POPUP_POSITION|EITA_RECOVER_ENABLED|EITA_INLINE_ENABLED|EITA_KEY_POPUP|EITA_KEY_POPUP_FALLBACK|EITA_KEY_ACCEPT|EITA_KEY_REINDEX)
        [[ -n "$value" ]] || continue
        if [[ -z "${(P)key}" ]]; then
          typeset -gx "$key=$value"
        fi
        ;;
    esac
  done < <(command eita config zsh 2>/dev/null)
}
_eita_load_settings

_eita_pick_widget() {
  emulate -L zsh
  setopt localoptions no_aliases

  local temp_dir selected_file cursor_file aliases_file alias_name status
  temp_dir=$(command mktemp -d) || return 1
  selected_file="$temp_dir/selected"
  cursor_file="$temp_dir/cursor"
  aliases_file="$temp_dir/aliases.tsv"

  # Collect aliases from Zsh itself, without parsing alias output or running them.
  : > "$aliases_file"
  for alias_name in ${(k)aliases}; do
    [[ "$alias_name" == *$'\t'* || "$alias_name" == *$'\n'* ]] && continue
    [[ "${aliases[$alias_name]}" == *$'\n'* ]] && continue
    print -r -- "${alias_name}"$'\t'"Alias: ${aliases[$alias_name]}" >> "$aliases_file"
  done

  local -x EITA_ALIASES_FILE="$aliases_file"
  # Release ZLE display before allowing Ratatui to draw the inline picker.
  zle -I
  command eita pick \
    --left "$LBUFFER" \
    --right "$RBUFFER" \
    --output "$selected_file" \
    --cursor-output "$cursor_file" \
    </dev/tty >/dev/tty
  status=$?

  if (( status == 0 )) && [[ -s "$selected_file" && -s "$cursor_file" ]]; then
    BUFFER=$(<"$selected_file")
    CURSOR=$(<"$cursor_file")
  fi
  command rm -rf -- "$temp_dir"
  zle redisplay
  return 0
}

zle -N eita-pick _eita_pick_widget
# Ctrl+Space can be intercepted by the terminal; Ctrl+X Ctrl+P is a fallback.
bindkey "${EITA_KEY_POPUP:-^@}" eita-pick
bindkey "${EITA_KEY_POPUP_FALLBACK:-^X^P}" eita-pick


# Build an offline cache of literal option and subcommand specifications from fpath.
# Compinit, shell functions and completion definitions are never evaluated by Eita.
eita-index() {
  emulate -L zsh
  local -x EITA_FPATH="${(j.:.)fpath}"
  command eita index --refresh "$@"
}

_eita_index_widget() {
  emulate -L zsh
  zle -I
  eita-index </dev/tty >/dev/tty
  zle redisplay
}
zle -N eita-reindex _eita_index_widget
bindkey "${EITA_KEY_REINDEX:-^X^I}" eita-reindex


# ── Opt-in ghost text ────────────────────────────────────────────────────────
# POSTDISPLAY contains only a visual suffix. It never mutates BUFFER or runs a
# command. Avoid interfering with other POSTDISPLAY owners or the native Tab.
typeset -g _eita_ghost=''
typeset -g _eita_cached_left=''
typeset -g _eita_cached_suffix=''

_eita_inline_clear() {
  emulate -L zsh
  region_highlight=(${region_highlight:#*memo=eita-ghost})
  if [[ -n "$_eita_ghost" && "$POSTDISPLAY" == "$_eita_ghost" ]]; then
    POSTDISPLAY=''
  fi
  _eita_ghost=''
}

_eita_inline_pre_redraw() {
  emulate -L zsh
  setopt localoptions no_aliases
  local left ghost start finish min
  _eita_inline_clear
  [[ "${EITA_INLINE_ENABLED:-0}" == 1 ]] || return 0
  [[ -z "$RBUFFER" && -z "$POSTDISPLAY" ]] || return 0
  left="$LBUFFER"
  min="${EITA_INLINE_MIN_CHARS:-3}"
  [[ "$min" == <-> ]] || min=3
  (( ${#left} >= min && ${#left} <= 400 )) || return 0

  if [[ "$left" == "$_eita_cached_left" ]]; then
    ghost="$_eita_cached_suffix"
  else
    ghost=$(command eita inline --left "$left" 2>/dev/null) || ghost=''
    _eita_cached_left="$left"
    _eita_cached_suffix="$ghost"
  fi
  (( ${#ghost} > 0 && ${#ghost} <= 160 )) || return 0
  _eita_ghost="$ghost"
  POSTDISPLAY="$ghost"
  start=$(( ${#PREDISPLAY} + ${#BUFFER} ))
  finish=$(( start + ${#ghost} ))
  region_highlight+=("P${start} ${finish} fg=${EITA_INLINE_COLOR:-8} memo=eita-ghost")
}

_eita_inline_finish() {
  _eita_inline_clear
  _eita_cached_left=''
  _eita_cached_suffix=''
}

_eita_inline_accept() {
  emulate -L zsh
  if [[ -n "$_eita_ghost" && "$POSTDISPLAY" == "$_eita_ghost" ]]; then
    LBUFFER+="$_eita_ghost"
    _eita_ghost=''
    POSTDISPLAY=''
    _eita_cached_left=''
    _eita_cached_suffix=''
    zle redisplay
  else
    zle beep
  fi
}
zle -N eita-inline-accept _eita_inline_accept
# Never intercept native Tab or Right Arrow. Ctrl+X Ctrl+E accepts ghost text.
bindkey "${EITA_KEY_ACCEPT:-^X^E}" eita-inline-accept

# Use native hook composition. No monkey-patching zle-line-pre-redraw.
autoload -Uz add-zle-hook-widget
if (( $+functions[add-zle-hook-widget] )); then
  add-zle-hook-widget -d line-pre-redraw _eita_inline_pre_redraw 2>/dev/null || true
  add-zle-hook-widget -d line-finish _eita_inline_finish 2>/dev/null || true
  add-zle-hook-widget line-pre-redraw _eita_inline_pre_redraw 2>/dev/null || true
  add-zle-hook-widget line-finish _eita_inline_finish 2>/dev/null || true
fi

eita-inline-on() {
  export EITA_INLINE_ENABLED=1
  print -r -- '🦀 Eita inline ON · Ctrl+X Ctrl+E accepts a suggestion'
}

eita-inline-off() {
  export EITA_INLINE_ENABLED=0
  _eita_cached_left=''
  _eita_cached_suffix=''
  print -r -- '🦀 Eita inline OFF'
}

# ── Optional local ranking; only known command templates are stored ─────────
_eita_usage_preexec() {
  [[ "${EITA_USAGE_ENABLED:-0}" == 1 ]] || return 0
  command eita usage record --line "$1" >/dev/null 2>&1
}

eita-usage-on() {
  autoload -Uz add-zsh-hook
  export EITA_USAGE_ENABLED=1
  add-zsh-hook -d preexec _eita_usage_preexec 2>/dev/null
  add-zsh-hook preexec _eita_usage_preexec
  print -r -- '🦀 Eita ranking ON · only recognized command templates are stored locally'
}

eita-usage-off() {
  export EITA_USAGE_ENABLED=0
  autoload -Uz add-zsh-hook
  add-zsh-hook -d preexec _eita_usage_preexec 2>/dev/null
  print -r -- '🦀 Eita ranking OFF'
}


# Direct interactive demo for mobile SSH terminals.
# Unlike the inline/bench commands, this opens a real Ratatui picker.
eita-popup() {
  emulate -L zsh
  local query="${*:-git com}"
  command eita popup --line "$query"
}

eita-keys() {
  emulate -L zsh
  print -r -- "🦀 Popup: ${EITA_KEY_POPUP:-^@}, fallback ${EITA_KEY_POPUP_FALLBACK:-^X^P}"
  print -r -- "   Accept ghost: ${EITA_KEY_ACCEPT:-^X^E}"
  bindkey "${EITA_KEY_POPUP:-^@}"
  bindkey "${EITA_KEY_POPUP_FALLBACK:-^X^P}"
  bindkey "${EITA_KEY_ACCEPT:-^X^E}"
}


# ── Automatic typo recovery after command-not-found ──────────────────────────
# Only symbol NAMES are shared with Rust. No alias body, function body, variable
# value or command argument is indexed. Nothing is executed after selecting.
_eita_recover_symbols() {
  emulate -L zsh
  setopt localoptions no_aliases
  local destination="$1" name kind
  : > "$destination" || return 1
  zmodload zsh/parameter 2>/dev/null || true
  for kind in command alias function builtin reserved variable; do
    case "$kind" in
      command)
        for name in ${(k)commands}; do
          [[ "$name" == *$'\t'* || "$name" == *$'\n'* ]] && continue
          print -r -- "command"$'\t'"$name"$'\t''Executable disponível no PATH'
        done ;;
      alias)
        for name in ${(k)aliases}; do
          [[ "$name" == *$'\t'* || "$name" == *$'\n'* ]] && continue
          print -r -- "alias"$'\t'"$name"$'\t''Alias do Zsh'
        done ;;
      function)
        for name in ${(k)functions}; do
          [[ "$name" == *$'\t'* || "$name" == *$'\n'* ]] && continue
          print -r -- "function"$'\t'"$name"$'\t''Função do Zsh'
        done ;;
      builtin)
        for name in ${(k)builtins}; do
          [[ "$name" == *$'\t'* || "$name" == *$'\n'* ]] && continue
          print -r -- "builtin"$'\t'"$name"$'\t''Comando interno'
        done ;;
      reserved)
        for name in ${(k)reswords}; do
          [[ "$name" == *$'\t'* || "$name" == *$'\n'* ]] && continue
          print -r -- "reserved"$'\t'"$name"$'\t''Palavra reservada'
        done ;;
      variable)
        for name in ${(k)parameters}; do
          [[ "$name" == *$'\t'* || "$name" == *$'\n'* ]] && continue
          print -r -- "variable"$'\t'"$name"$'\t''Variável definida'
        done ;;
    esac
  done > "$destination"
}

_eita_original_not_found() {
  emulate -L zsh
  if (( $+functions[_eita_previous_command_not_found_handler] )); then
    _eita_previous_command_not_found_handler "$@"
  else
    print -ru2 -- "zsh: command not found: $1"
  fi
  return 127
}

_eita_command_not_found_handler() {
  emulate -L zsh
  setopt localoptions no_aliases
  local missing="$1" temp_dir symbols selected status corrected arg
  shift

  # Do not start a TUI in scripts, redirected shells or non-tty sessions.
  if [[ "${EITA_RECOVER_ENABLED:-0}" != 1 || ! -t 0 || ! -t 1 ]] ||
    (( ! $+commands[eita] )); then
    _eita_original_not_found "$missing" "$@"
    return 127
  fi
  temp_dir=$(command mktemp -d) || {
    _eita_original_not_found "$missing" "$@"
    return 127
  }
  symbols="$temp_dir/symbols.tsv"
  selected="$temp_dir/selected"
  _eita_recover_symbols "$symbols"
  if [[ ! -s "$symbols" ]]; then
    command rm -rf -- "$temp_dir"
    _eita_original_not_found "$missing" "$@"
    return 127
  fi

  # Only take over if the matcher found a plausible, real shell symbol.
  # Otherwise keep the user's existing command-not-found integration working.
  if ! command eita recover --word "$missing" --kind command \
      --symbols-file "$symbols" --format lines > "$temp_dir/matches" ||
      [[ ! -s "$temp_dir/matches" ]]; then
    command rm -rf -- "$temp_dir"
    _eita_original_not_found "$missing" "$@"
    return 127
  fi

  # Keep the error in scrollback; Ratatui renders before the next Zsh prompt.
  print -ru2 -- "zsh: command not found: $missing"
  command eita recover --word "$missing" --kind command \
    --symbols-file "$symbols" --popup --output "$selected" \
    </dev/tty >/dev/tty
  status=$?
  if (( status == 0 )) && [[ -s "$selected" ]]; then
    corrected=$(<"$selected")
    # Restore the original arguments as shell-quoted text, for editing only.
    local -a rest=()
    for arg in "$@"; do
      rest+=("${(q)arg}")
    done
    if (( ${#rest} )); then
      corrected+=" ${(j: :)rest}"
    fi
    print -z -- "$corrected"
    print -ru2 -- "🦀 Eita: correção preparada no próximo prompt, sem executar."
  fi
  command rm -rf -- "$temp_dir"
  return 127
}

eita-recover-on() {
  emulate -L zsh
  setopt localoptions no_aliases
  if [[ "${_eita_recover_installed:-0}" == 1 ]]; then
    export EITA_RECOVER_ENABLED=1
    return 0
  fi
  zmodload zsh/parameter 2>/dev/null || true
  if (( $+functions[command_not_found_handler] )) &&
    (( ! $+functions[_eita_previous_command_not_found_handler] )); then
    functions -c command_not_found_handler _eita_previous_command_not_found_handler
  fi
  functions -c _eita_command_not_found_handler command_not_found_handler
  typeset -g _eita_recover_installed=1
  export EITA_RECOVER_ENABLED=1
}

eita-recover-off() {
  emulate -L zsh
  export EITA_RECOVER_ENABLED=0
  [[ "${_eita_recover_installed:-0}" == 1 ]] || return 0
  typeset -g _eita_recover_installed=0
  if (( $+functions[_eita_previous_command_not_found_handler] )); then
    functions -c _eita_previous_command_not_found_handler command_not_found_handler
    unfunction _eita_previous_command_not_found_handler
  else
    unfunction command_not_found_handler 2>/dev/null
  fi
}

# Debug arbitrary command names or typos in variable names. Values stay private.
eita-what() {
  emulate -L zsh
  local word="$1" kind="${2:-all}" temp_dir
  [[ -n "$word" ]] || {
    print -ru2 -- 'Usage: eita-what WORD [command|variable|all]'
    return 2
  }
  temp_dir=$(command mktemp -d) || return 1
  _eita_recover_symbols "$temp_dir/symbols.tsv"
  command eita recover --word "$word" --kind "$kind" \
    --symbols-file "$temp_dir/symbols.tsv" --format json
  local status=$?
  command rm -rf -- "$temp_dir"
  return $status
}

# Default to recovery for interactive shells, unless explicitly opted out.
if [[ "${EITA_RECOVER_ENABLED:-1}" == 1 ]]; then
  eita-recover-on
fi
