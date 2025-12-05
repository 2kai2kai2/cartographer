use crate::helpers::SplitAtFirst;
use std::fmt::{Display, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextToken<'a> {
    Equal,
    OpenBracket,
    CloseBracket,
    /// Used for all negative numbers
    Int(i64),
    /// Used for all positive numbers
    UInt(u64),
    Float(f64),
    Bool(bool),
    StringQuoted(&'a str),
    StringUnquoted(&'a str),

    /// Only in game files, `@variable` will become `Variable("variable")`
    Variable(&'a str),
    /// Only in game files, `@[expr]` will become `Expr("expr")`
    Expr(&'a str),
}
impl<'a> TextToken<'a> {
    pub fn is_base_scalar(&self) -> bool {
        return matches!(
            self,
            TextToken::Int(_)
                | TextToken::UInt(_)
                | TextToken::Float(_)
                | TextToken::Bool(_)
                | TextToken::StringQuoted(_)
                | TextToken::StringUnquoted(_)
        );
    }
    pub fn token_type_repr(&self) -> &'static str {
        return match self {
            TextToken::Equal => "=",
            TextToken::OpenBracket => "{",
            TextToken::CloseBracket => "}",
            TextToken::Int(_) => "int",
            TextToken::UInt(_) => "uint",
            TextToken::Float(_) => "float",
            TextToken::Bool(_) => "bool",
            TextToken::StringQuoted(_) => "string_quoted",
            TextToken::StringUnquoted(_) => "string_unquoted",
            TextToken::Variable(_) => "variable",
            TextToken::Expr(_) => "expr",
        };
    }
}
impl<'a> Display for TextToken<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            TextToken::Equal => f.write_char('='),
            TextToken::OpenBracket => f.write_char('{'),
            TextToken::CloseBracket => f.write_char('}'),
            TextToken::Int(num) => write!(f, "{num}"),
            TextToken::UInt(num) => write!(f, "{num}"),
            TextToken::Float(num) => write!(f, "{num}"),
            TextToken::Bool(value) => write!(f, "{value}"),
            TextToken::StringQuoted(text) => f.write_fmt(format_args!("\"{text}\"")),
            TextToken::StringUnquoted(text) => f.write_str(text),
            TextToken::Variable(var) => f.write_fmt(format_args!("@{var}")),
            TextToken::Expr(expr) => f.write_fmt(format_args!("@[{expr}]")),
        };
    }
}

#[derive(Clone)]
pub struct TextLexer<'a>(&'a str);
impl<'a> TextLexer<'a> {
    pub fn new(buffer: &'a str) -> TextLexer<'a> {
        return TextLexer(buffer);
    }

    pub fn print_to_string(self) -> String {
        let mut depth: usize = 0;
        let mut out_buf = String::new();

        for token in self {
            if let TextToken::CloseBracket = token {
                depth = depth.saturating_sub(4);
            }
            out_buf.push_str(&format!("{:depth$}{token}\n", ""));
            if let TextToken::OpenBracket = token {
                depth += 4;
            }
        }

        return out_buf;
    }
}
impl<'a> Iterator for TextLexer<'a> {
    type Item = TextToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (peek, rest) = {
            let mut it = self.0.trim_ascii_start().chars();
            let peek = it.next()?;
            (peek, it.as_str())
        };
        match peek {
            '=' => {
                self.0 = rest;
                return Some(TextToken::Equal);
            }
            '{' => {
                self.0 = rest;
                return Some(TextToken::OpenBracket);
            }
            '}' => {
                self.0 = rest;
                return Some(TextToken::CloseBracket);
            }
            '"' => {
                let mut it = rest.char_indices();
                while let Some((i, c)) = it.next() {
                    if c == '\\' {
                        it.next();
                    } else if c == '"' {
                        let (string, rest) = rest.split_at(i);
                        self.0 = &rest[1..];
                        return Some(TextToken::StringQuoted(string));
                    }
                }
                return None; // unclosed string
            }
            '#' => {
                // escape comment
                // may cause internal state change even if it
                // doesn't find a token (if comments at end of file)
                // but this doesn't really matter since it's just comments being skipped.
                // we loop since letting recursion handle it can cause stack overflow

                self.0 = rest;
                loop {
                    let (_, rest) = self.0.split_once('\n')?;
                    self.0 = rest.trim_ascii_start();
                    if !rest.starts_with('#') {
                        break;
                    }
                }

                return self.next();
            }
            '@' => {
                if let Some(rest) = rest.strip_prefix('[') {
                    // TODO: should we handle comments within an expression?
                    let (expr, rest) = rest.split_at_first(|c| *c == ']')?;
                    self.0 = rest;
                    return Some(TextToken::Expr(expr));
                }
                let (var, rest) = rest
                    .split_at_first_inclusive(|&c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(rest.split_at(rest.len()));
                if var.is_empty() {
                    return None;
                }
                self.0 = rest;
                return Some(TextToken::Variable(var));
            }
            _ => {}
        }

        // Otherwise, it is some scalar but we need to figure it out:
        // Use `self.0` instead of `rest` since we want to include `peek`
        let (value, rest) = self
            .0
            .trim_ascii_start()
            .split_at_first_inclusive(char_ends_token)
            .unwrap_or(rest.split_at(rest.len()));

        if value == "yes" {
            self.0 = rest;
            return Some(TextToken::Bool(true));
        } else if value == "no" {
            self.0 = rest;
            return Some(TextToken::Bool(false));
        }

        let mut dot_count: usize = 0;
        let mut signed: bool = false;
        for (i, b) in value.bytes().enumerate() {
            if b == b'-' && i == 0 {
                signed = true;
            } else if b == b'.' {
                dot_count += 1;
                continue;
            } else if b.is_ascii_digit() {
                continue;
            } else {
                self.0 = rest;
                return Some(TextToken::StringUnquoted(value));
            }
        }
        let value = match dot_count {
            0 if signed => TextToken::Int(value.parse().ok()?),
            0 if !signed => TextToken::UInt(value.parse().ok()?),
            1 => TextToken::Float(value.parse().ok()?),
            _ => TextToken::StringUnquoted(value),
        };
        self.0 = rest;
        return Some(value);
    }
}

fn char_ends_token(c: &char) -> bool {
    *c == '=' || *c == '{' || *c == '}' || c.is_ascii_whitespace()
}
