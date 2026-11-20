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
