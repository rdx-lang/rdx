/// LaTeX math tokenizer.
///
/// Converts a raw LaTeX math string (without `$` delimiters) into a flat sequence of
/// [`Token`] values that the parser can consume.
/// A single lexical unit from a LaTeX math string.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    // Grouping
    LBrace,
    RBrace,

    // Scripts and alignment
    Caret,
    Underscore,
    Ampersand,
    Tilde, // ~ non-breaking space

    // Delimiter characters (when appearing bare, not after \left/\right)
    LParen,
    RParen,
    LBracket,
    RBracket,
    Pipe,

    // Single-character operators
    Plus,
    Minus,
    Equals,
    LessThan,
    GreaterThan,
    Comma,
    Semicolon,
    Colon,
    Bang,
    Prime, // '

    // Commands: \word  (alphabetic sequences)
    Command(String),

    // \\ row separator
    DoubleBackslash,

    // Spacing shorthand: \, \; \: \!  (these are also commands but special-cased)
    ThinSpace,    // \,
    MedSpace,     // \; or \:
    NegThinSpace, // \!

    // Literals
    Letter(char), // a-z A-Z
    Digit(char),  // 0-9
    Dot,          // .

    // Compound tokens produced by the tokenizer when it sees \begin{name} or \end{name}
    Begin(String),
    End(String),

    // Collapsed whitespace
    Whitespace,

    // End of input
    Eof,
}

/// Positional information attached to a token.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Span {
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character.
    pub end: usize,
}

/// A token together with its source span.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub span: Span,
}

/// Tokenize a LaTeX math string into a `Vec<Spanned>`.
///
/// The returned vector always ends with a [`Token::Eof`] entry.
pub(crate) fn tokenize(input: &str) -> Vec<Spanned> {
    let bytes = input.as_bytes();
    let len = input.len();
    let mut pos = 0usize;
    let mut out: Vec<Spanned> = Vec::new();

    macro_rules! push {
        ($start:expr, $tok:expr) => {
            out.push(Spanned {
                token: $tok,
                span: Span {
                    start: $start,
                    end: pos,
                },
            });
        };
    }

    while pos < len {
        let start = pos;
        // SAFETY: `pos < len` is guaranteed by the while condition, and `input` is
        // valid UTF-8, so there is always at least one char remaining.
        let Some(ch) = input[pos..].chars().next() else {
            break;
        };
        let ch_len = ch.len_utf8();

        match ch {
            // Whitespace: collapse runs
            c if c.is_ascii_whitespace() => {
                while pos < len && input[pos..].starts_with(|c: char| c.is_ascii_whitespace()) {
                    pos += 1;
                }
                push!(start, Token::Whitespace);
            }

            '{' => {
                pos += 1;
                push!(start, Token::LBrace);
            }
            '}' => {
                pos += 1;
                push!(start, Token::RBrace);
            }
            '^' => {
                pos += 1;
                push!(start, Token::Caret);
            }
            '_' => {
                pos += 1;
                push!(start, Token::Underscore);
            }
            '&' => {
                pos += 1;
                push!(start, Token::Ampersand);
            }
            '~' => {
                pos += 1;
                push!(start, Token::Tilde);
            }
            '(' => {
                pos += 1;
                push!(start, Token::LParen);
            }
            ')' => {
                pos += 1;
                push!(start, Token::RParen);
            }
            '[' => {
                pos += 1;
                push!(start, Token::LBracket);
            }
            ']' => {
                pos += 1;
                push!(start, Token::RBracket);
            }
            '|' => {
                pos += 1;
                push!(start, Token::Pipe);
            }
            '+' => {
                pos += 1;
                push!(start, Token::Plus);
            }
            '-' => {
                pos += 1;
                push!(start, Token::Minus);
            }
            '=' => {
                pos += 1;
                push!(start, Token::Equals);
            }
            '<' => {
                pos += 1;
                push!(start, Token::LessThan);
            }
            '>' => {
                pos += 1;
                push!(start, Token::GreaterThan);
            }
            ',' => {
                pos += 1;
                push!(start, Token::Comma);
            }
            ';' => {
                pos += 1;
                push!(start, Token::Semicolon);
            }
            ':' => {
                pos += 1;
                push!(start, Token::Colon);
            }
            '!' => {
                pos += 1;
                push!(start, Token::Bang);
            }
            '\'' => {
                pos += 1;
                push!(start, Token::Prime);
            }
            '.' => {
                pos += 1;
                push!(start, Token::Dot);
            }

            '\\' => {
                pos += 1; // consume backslash
                if pos >= len {
                    // Trailing backslash — emit as error via an unknown command
                    push!(start, Token::Command("".to_string()));
                    continue;
                }

                let Some(next) = input[pos..].chars().next() else {
                    push!(start, Token::Command("".to_string()));
                    continue;
                };

                if next == '\\' {
                    // \\ — double backslash (row separator)
                    pos += 1;
                    push!(start, Token::DoubleBackslash);
                } else if next == ',' {
                    pos += 1;
                    push!(start, Token::ThinSpace);
                } else if next == ';' || next == ':' {
                    pos += 1;
                    push!(start, Token::MedSpace);
                } else if next == '!' {
                    pos += 1;
                    push!(start, Token::NegThinSpace);
                } else if next == ' ' {
                    // \ followed by a space — normal space
                    pos += 1;
                    push!(start, Token::Whitespace);
                } else if next.is_ascii_alphabetic() {
                    // Collect alphabetic command name
                    let name_start = pos;
                    while pos < len {
                        let Some(c) = input[pos..].chars().next() else {
                            break;
                        };
                        if c.is_ascii_alphabetic() {
                            pos += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let name = &input[name_start..pos];

                    if name == "begin" || name == "end" {
                        // Consume optional whitespace then {env_name}
                        skip_whitespace(input, &mut pos);
                        if pos < len && bytes[pos] == b'{' {
                            pos += 1; // {
                            let env_start = pos;
                            while pos < len && bytes[pos] != b'}' {
                                pos += 1;
                            }
                            let env_name = input[env_start..pos].trim().to_string();
                            if pos < len {
                                pos += 1; // }
                            }
                            if name == "begin" {
                                push!(start, Token::Begin(env_name));
                            } else {
                                push!(start, Token::End(env_name));
                            }
                        } else {
                            // Malformed: \begin without {
                            let tok = if name == "begin" {
                                Token::Begin(String::new())
                            } else {
                                Token::End(String::new())
                            };
                            push!(start, tok);
                        }
                    } else {
                        push!(start, Token::Command(name.to_string()));
                    }
                } else {
                    // Non-alpha single character after backslash: \{ \} \| \[ \] etc.
                    let sym = next.to_string();
                    pos += next.len_utf8();
                    push!(start, Token::Command(sym));
                }
            }

            c if c.is_ascii_alphabetic() => {
                pos += ch_len;
                push!(start, Token::Letter(c));
            }

            c if c.is_ascii_digit() => {
                pos += ch_len;
                push!(start, Token::Digit(c));
            }

            // Skip non-ASCII, non-special characters by emitting as a Letter if they are Unicode
            // math letters; otherwise advance past them to avoid infinite loops.
            c => {
                pos += ch_len;
                // Emit as Letter so the parser can at least produce an Ident node.
                push!(start, Token::Letter(c));
            }
        }
    }

    // Always terminate with Eof
    out.push(Spanned {
        token: Token::Eof,
        span: Span {
            start: len,
            end: len,
        },
    });

    out
}

/// Convert a token back to its raw LaTeX representation (used for raw-string extraction).
pub(crate) fn token_to_raw_str(tok: &Token) -> String {
    match tok {
        Token::LBrace => "{".to_string(),
        Token::RBrace => "}".to_string(),
        Token::Caret => "^".to_string(),
        Token::Underscore => "_".to_string(),
        Token::Ampersand => "&".to_string(),
        Token::Tilde => "~".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::Pipe => "|".to_string(),
        Token::Plus => "+".to_string(),
        Token::Minus => "-".to_string(),
        Token::Equals => "=".to_string(),
        Token::LessThan => "<".to_string(),
        Token::GreaterThan => ">".to_string(),
        Token::Comma => ",".to_string(),
        Token::Semicolon => ";".to_string(),
        Token::Colon => ":".to_string(),
        Token::Bang => "!".to_string(),
        Token::Prime => "'".to_string(),
        Token::Dot => ".".to_string(),
        Token::Command(c) => format!("\\{c}"),
        Token::DoubleBackslash => "\\\\".to_string(),
        Token::ThinSpace => "\\,".to_string(),
        Token::MedSpace => "\\;".to_string(),
        Token::NegThinSpace => "\\!".to_string(),
        Token::Letter(c) => c.to_string(),
        Token::Digit(c) => c.to_string(),
        Token::Begin(e) => format!("\\begin{{{e}}}"),
        Token::End(e) => format!("\\end{{{e}}}"),
        Token::Whitespace => " ".to_string(),
        Token::Eof => String::new(),
    }
}

fn skip_whitespace(input: &str, pos: &mut usize) {
    while *pos < input.len() {
        let Some(c) = input[*pos..].chars().next() else {
            break;
        };
        if c.is_ascii_whitespace() {
            *pos += c.len_utf8();
        } else {
            break;
        }
    }
}

// ─── TokenStream ──────────────────────────────────────────────────────────────

/// A cursor over a `Vec<Spanned>` that the parser uses.
pub(crate) struct TokenStream {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl TokenStream {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Peek at the current token without consuming it.
    pub fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    /// Peek at the token `offset` positions ahead (0 = current).
    pub fn peek_ahead(&self, offset: usize) -> &Token {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].token
    }

    /// Consume and return the current token.
    pub fn next(&mut self) -> Token {
        let tok = self.tokens[self.pos].token.clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    /// Byte offset of the current token.
    #[allow(dead_code)]
    pub fn current_offset(&self) -> usize {
        self.tokens[self.pos].span.start
    }

    /// Skip whitespace tokens.
    pub fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Token::Whitespace) {
            self.next();
        }
    }

    /// Returns `true` if the stream is at end-of-input (Eof token).
    #[allow(dead_code)]
    pub fn is_eof(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    /// Consume a `{...}` group and return its contents as a raw string,
    /// preserving whitespace exactly.  Returns `None` if the current token
    /// is not `{`.
    pub fn read_raw_brace_string(&mut self) -> Option<String> {
        if !matches!(self.peek(), Token::LBrace) {
            return None;
        }
        self.next(); // consume {

        let mut result = String::new();
        let mut depth = 1usize;

        loop {
            match self.peek().clone() {
                Token::Eof => break,
                Token::LBrace => {
                    depth += 1;
                    result.push('{');
                    self.next();
                }
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.next(); // consume closing }
                        break;
                    }
                    result.push('}');
                    self.next();
                }
                Token::Whitespace => {
                    result.push(' ');
                    self.next();
                }
                tok => {
                    result.push_str(&token_to_raw_str(&tok));
                    self.next();
                }
            }
        }

        Some(result)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &str) -> Vec<Token> {
        tokenize(input).into_iter().map(|s| s.token).collect()
    }

    #[test]
    fn tokenize_simple_letters() {
        let toks = tokens("abc");
        assert_eq!(
            toks,
            vec![
                Token::Letter('a'),
                Token::Letter('b'),
                Token::Letter('c'),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_digits() {
        let toks = tokens("123");
        assert_eq!(
            toks,
            vec![
                Token::Digit('1'),
                Token::Digit('2'),
                Token::Digit('3'),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_command() {
        let toks = tokens(r"\frac");
        assert_eq!(toks, vec![Token::Command("frac".to_string()), Token::Eof]);
    }

    #[test]
    fn tokenize_double_backslash() {
        let toks = tokens(r"\\");
        assert_eq!(toks, vec![Token::DoubleBackslash, Token::Eof]);
    }

    #[test]
    fn tokenize_spacing() {
        let toks = tokens(r"\,\;\!");
        assert_eq!(
            toks,
            vec![
                Token::ThinSpace,
                Token::MedSpace,
                Token::NegThinSpace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_begin_end() {
        let toks = tokens(r"\begin{pmatrix}");
        assert_eq!(toks, vec![Token::Begin("pmatrix".to_string()), Token::Eof]);
    }

    #[test]
    fn tokenize_end_env() {
        let toks = tokens(r"\end{pmatrix}");
        assert_eq!(toks, vec![Token::End("pmatrix".to_string()), Token::Eof]);
    }

    #[test]
    fn tokenize_scripts() {
        let toks = tokens("x^2_i");
        assert_eq!(
            toks,
            vec![
                Token::Letter('x'),
                Token::Caret,
                Token::Digit('2'),
                Token::Underscore,
                Token::Letter('i'),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_braces() {
        let toks = tokens("{a}");
        assert_eq!(
            toks,
            vec![Token::LBrace, Token::Letter('a'), Token::RBrace, Token::Eof,]
        );
    }

    #[test]
    fn tokenize_whitespace_collapsed() {
        let toks = tokens("a   b");
        assert_eq!(
            toks,
            vec![
                Token::Letter('a'),
                Token::Whitespace,
                Token::Letter('b'),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_backslash_brace() {
        // \{ and \} are used for literal brace delimiters in \left\{ ... \right\}
        let toks = tokens(r"\{");
        assert_eq!(toks, vec![Token::Command("{".to_string()), Token::Eof]);
    }

    #[test]
    fn tokenize_pipe() {
        let toks = tokens("|");
        assert_eq!(toks, vec![Token::Pipe, Token::Eof]);
    }

    #[test]
    fn tokenize_operators() {
        let toks = tokens("+-=<>");
        assert_eq!(
            toks,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Equals,
                Token::LessThan,
                Token::GreaterThan,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn token_stream_peek_and_next() {
        let ts_tokens = tokenize("ab");
        let mut ts = TokenStream::new(ts_tokens);
        assert_eq!(ts.peek(), &Token::Letter('a'));
        ts.next();
        assert_eq!(ts.peek(), &Token::Letter('b'));
        ts.next();
        assert_eq!(ts.peek(), &Token::Eof);
    }

    #[test]
    fn token_stream_skip_whitespace() {
        let ts_tokens = tokenize("a   b");
        let mut ts = TokenStream::new(ts_tokens);
        ts.next(); // consume 'a'
        ts.skip_whitespace();
        assert_eq!(ts.peek(), &Token::Letter('b'));
    }
}
