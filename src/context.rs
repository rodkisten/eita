//! Lightweight cursor-aware shell segment analysis.
//!
//! No evaluation or shell execution: we only identify a safe, unquoted region
//! to replace. The buffer outside that region is preserved.

#[derive(Clone, Debug)]
pub struct Context {
    prefix: String,
    query: String,
    right: String,
    skip_right: usize,
    supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    pub replacement: String,
    pub cursor: usize,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Quote {
    None,
    Single,
    Double,
}

impl Context {
    /// left/right correspond to ZLE's LBUFFER/RBUFFER.
    pub fn new(left: &str, right: &str) -> Self {
        let mut quote = Quote::None;
        let mut escaped = false;
        let mut segment_start = 0;

        for (index, character) in left.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match quote {
                Quote::Single => {
                    if character == '\'' {
                        quote = Quote::None;
                    }
                }
                Quote::Double => match character {
                    '"' => quote = Quote::None,
                    '\\' => escaped = true,
                    _ => {}
                },
                Quote::None => match character {
                    '\\' => escaped = true,
                    '\'' => quote = Quote::Single,
                    '"' => quote = Quote::Double,
                    ';' | '|' | '&' | '\n' => segment_start = index + character.len_utf8(),
                    _ => {}
                },
            }
        }

        let segment = &left[segment_start..];
        let trimmed = segment.trim_start_matches(char::is_whitespace);
        let leading_bytes = segment.len() - trimmed.len();
        let prefix = left[..segment_start + leading_bytes].to_owned();

        // We cannot safely rewrite an incomplete quoted or escaped expression.
        let supported = quote == Quote::None && !escaped && !is_comment(trimmed);

        // A partially typed word to the right of the cursor belongs to the
        // token being replaced; the rest of RBUFFER must stay intact.
        let skip_right = if left.chars().last().is_some_and(is_word_char) {
            right
                .char_indices()
                .find(|(_, character)| !is_word_char(*character))
                .map_or(right.len(), |(index, _)| index)
        } else {
            0
        };

        Self {
            prefix,
            query: trimmed.to_owned(),
            right: right.to_owned(),
            skip_right,
            supported,
        }
    }

    pub fn query(&self) -> Option<&str> {
        self.supported.then_some(&self.query)
    }

    pub fn apply(&self, value: &str) -> Applied {
        let mut replacement =
            String::with_capacity(self.prefix.len() + value.len() + self.right.len());
        replacement.push_str(&self.prefix);
        replacement.push_str(value);
        if self.query.chars().last().is_some_and(char::is_whitespace) {
            let trimmed = self.query.trim_end_matches(char::is_whitespace);
            replacement.push_str(&self.query[trimmed.len()..]);
        }
        let cursor = replacement.chars().count();
        replacement.push_str(&self.right[self.skip_right..]);
        Applied {
            replacement,
            cursor,
        }
    }
}

fn is_word_char(character: char) -> bool {
    !character.is_whitespace()
        && !matches!(
            character,
            ';' | '|' | '&' | '<' | '>' | '\'' | '"' | '(' | ')'
        )
}

fn is_comment(query: &str) -> bool {
    // Conservative handling: do not offer completions inside a comment.
    // In Zsh interactive comments are opt-in; choosing not to rewrite them is safe.
    query.starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_commands_before_pipeline() {
        let context = Context::new("echo hello | git com", " --amend");
        assert_eq!(context.query(), Some("git com"));
        assert_eq!(
            context.apply("git commit"),
            Applied {
                replacement: "echo hello | git commit --amend".into(),
                cursor: "echo hello | git commit".chars().count(),
            }
        );
    }

    #[test]
    fn keeps_quoted_separators_untouched() {
        let context = Context::new("echo 'a | b' && git com", " --amend");
        assert_eq!(context.query(), Some("git com"));
        assert_eq!(
            context.apply("git commit").replacement,
            "echo 'a | b' && git commit --amend"
        );
    }

    #[test]
    fn handles_escaped_delimiters() {
        let context = Context::new(r"echo a\;b; git com", "");
        assert_eq!(context.query(), Some("git com"));
    }

    #[test]
    fn replaces_only_current_word_suffix() {
        let context = Context::new("git com", "it --amend && echo done");
        assert_eq!(
            context.apply("git commit").replacement,
            "git commit --amend && echo done"
        );
    }

    #[test]
    fn does_not_drop_next_argument_after_space() {
        let context = Context::new("git com ", "--amend");
        assert_eq!(
            context.apply("git commit").replacement,
            "git commit --amend"
        );
    }

    #[test]
    fn refuses_unclosed_quotes_and_escapes() {
        assert_eq!(Context::new("git 'com", "").query(), None);
        assert_eq!(Context::new("git com\\", "").query(), None);
    }

    #[test]
    fn supports_unicode_cursor_offsets() {
        let context = Context::new("echo olá; git com", "it");
        assert_eq!(
            context.apply("git commit").cursor,
            "echo olá; git commit".chars().count()
        );
        assert_eq!(
            context.apply("git commit").replacement,
            "echo olá; git commit"
        );
    }

    #[test]
    fn preserves_leading_spaces_and_newlines() {
        let context = Context::new("echo ok\n  git com", "");
        assert_eq!(context.query(), Some("git com"));
        assert_eq!(
            context.apply("git commit").replacement,
            "echo ok\n  git commit"
        );
    }

    #[test]
    fn comments_are_not_suggested() {
        assert_eq!(Context::new("#git com", "").query(), None);
    }
}
