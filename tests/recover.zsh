#!/usr/bin/env zsh
emulate -L zsh
setopt err_return no_aliases
source integrations/zsh/eita.plugin.zsh
(( $+functions[command_not_found_handler] )) || return 1
[[ "$EITA_RECOVER_ENABLED" == 1 ]] || return 1

alias test_alias_git=git
eita_test_function() { :; }
typeset EITA_TEST_PRIVATE_VALUE='this-is-not-for-indexing'
local tempfile
tempfile=$(mktemp) || return 1
_eita_recover_symbols "$tempfile"
grep -F $'alias\ttest_alias_git\t' "$tempfile" >/dev/null
grep -F $'function\teita_test_function\t' "$tempfile" >/dev/null
grep -F $'variable\tEITA_TEST_PRIVATE_VALUE\t' "$tempfile" >/dev/null
! grep -F 'this-is-not-for-indexing' "$tempfile" >/dev/null
command eita recover --word gti --symbols-file "$tempfile" --format lines | grep -x git
command eita recover --word test_lias_git --kind alias \
  --symbols-file "$tempfile" --format lines | grep -x test_alias_git
command eita recover --word EITA_TEST_PRIVTE_VALUE --kind variable \
  --symbols-file "$tempfile" --format lines | grep -x EITA_TEST_PRIVATE_VALUE
command rm -f -- "$tempfile"

# Re-sourcing cannot copy our own handler, which would recurse on fallback.
source integrations/zsh/eita.plugin.zsh
(( ! $+functions[_eita_previous_command_not_found_handler] )) || return 1

# Preserve third-party command_not_found_handler when opted in.
eita-recover-off
command_not_found_handler() { print -r -- 'legacy-eita-test'; return 127 }
eita-recover-on
(( $+functions[_eita_previous_command_not_found_handler] )) || return 1
eita-recover-off
[[ "${functions[command_not_found_handler]}" == *legacy-eita-test* ]] || return 1
unfunction command_not_found_handler
print -r -- 'Zsh recovery integration OK'
