# Eita: opt-in cursor-aware Zsh Line Editor completion.
# No suggestions are executed; works independently from compinit and syntax highlighting.
[[ -o interactive ]] || return 0
(( $+commands[eita] )) || return 0

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
bindkey '^@' eita-pick
bindkey '^X^P' eita-pick


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
bindkey '^X^I' eita-reindex


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
bindkey '^X^E' eita-inline-accept

# Use native hook composition. No monkey-patching zle-line-pre-redraw.
autoload -Uz add-zle-hook-widget
if (( $+functions[add-zle-hook-widget] )); then
  add-zle-hook-widget -d line-pre-redraw _eita_inline_pre_redraw 2>/dev/null
  add-zle-hook-widget -d line-finish _eita_inline_finish 2>/dev/null
  add-zle-hook-widget line-pre-redraw _eita_inline_pre_redraw 2>/dev/null
  add-zle-hook-widget line-finish _eita_inline_finish 2>/dev/null
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
  command eita popup --line "$query" --height 8
}

eita-keys() {
  emulate -L zsh
  print -r -- '🦀 Ctrl+Space / Ctrl+X Ctrl+P = picker Ratatui'
  print -r -- '   Ctrl+X Ctrl+E = accept ghost text'
  bindkey '^@'
  bindkey '^X^P'
  bindkey '^X^E'
}
