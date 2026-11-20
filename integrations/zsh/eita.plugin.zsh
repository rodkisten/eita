# Eita: opt-in Ratatui picker for the Zsh Line Editor.
# Safe to source repeatedly. Never automatically executes a suggestion.
[[ -o interactive ]] || return 0
(( $+commands[eita] )) || return 0

_eita_pick_widget() {
  emulate -L zsh
  setopt localoptions no_aliases
  local selected_file status selected
  selected_file=$(command mktemp) || return 1

  # Release prompt rendering before taking over the tty.
  zle -I
  command eita pick --line "$BUFFER" --output "$selected_file" </dev/tty >/dev/tty
  status=$?
  if (( status == 0 )) && [[ -s "$selected_file" ]]; then
    selected=$(<"$selected_file")
    BUFFER="$selected"
    CURSOR=${#BUFFER}
  fi
  command rm -f -- "$selected_file"
  zle redisplay
  return 0
}
zle -N eita-pick _eita_pick_widget
# Ctrl+Space maps to NUL in many terminals; rebind if necessary.
bindkey '^@' eita-pick
