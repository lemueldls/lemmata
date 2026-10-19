//! The PTL frontend: upstream's reader output, expanded into something Rust can
//! translate.
//!
//! r[impl port.strategy]
//!
//! The glyph definitions are written in PTL, a Lisp with infix operators and
//! indentation, read by `patel` on top of Patrisika. Reimplementing that reader
//! would be work with no return, so the reference step takes its output:
//! `patel-c --dump-ast` writes the reader's S-expressions as JSON, and this
//! crate starts from there.
//!
//! What is left is expansion. Almost all of PTL's surface is macros — 23 of them
//! in `meta/macros.ptl` — over a fixed set of builtin forms, and the expander is
//! where the language lives. [`inventory`] counts the heads a corpus of reader
//! output actually uses, which is what sizes that work rather than guessing at
//! it.
//!
//! Across the 337 definition files that corpus names 2,530 distinct heads, but
//! only twenty of them are macros and thirteen are builtins. The rest are
//! ordinary calls, which the translator handles without expanding anything. The
//! hot ones are `include` at 8,232 uses and `glyph-proc` at 3,245, so those two
//! carry the design; `Just` at 624 is the only procedural macro in wide use.

// The suggestions here are for `const fn`, and the accessors cannot be const:
// returning a `&str` from a `String` field is a deref coercion.
#![allow(clippy::missing_const_for_fn)]

use std::collections::BTreeMap;
use std::fmt;

/// One node of the reader's output.
///
/// The dumps are JSON, so a node is an array, a string, a number, a boolean or
/// null. A program is an array whose first element is the form's name, which is
/// what [`Sexpr::form`] returns.
#[derive(Clone, Debug, PartialEq)]
pub enum Sexpr {
    List(Vec<Sexpr>),
    Atom(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl Sexpr {
    /// The name of the form a list starts with, or `None` for anything else.
    #[must_use]
    pub fn form(&self) -> Option<&str> {
        let Self::List(items) = self else {
            return None;
        };
        items.first()?.atom()
    }

    /// The atom a node is, if it is one.
    #[must_use]
    pub fn atom(&self) -> Option<&str> {
        let Self::Atom(text) = self else {
            return None;
        };

        Some(text)
    }

    /// Every list in the tree, parents before children.
    #[must_use]
    pub fn lists(&self) -> Vec<&Sexpr> {
        let mut found = Vec::new();
        let mut stack = vec![self];
        while let Some(next) = stack.pop() {
            let Self::List(items) = next else {
                continue;
            };
            found.push(next);
            stack.extend(items.iter().rev());
        }

        found
    }
}

impl fmt::Display for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List(items) => {
                write!(f, "[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Self::Atom(text) => write!(f, "{text}"),
            Self::Number(value) => write!(f, "{value}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Null => write!(f, "null"),
        }
    }
}

/// A JSON value, as the dump writes it.
#[derive(Debug)]
pub enum Error {
    Unexpected(char),
    Unterminated,
    Trailing,
    Number(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected(character) => write!(f, "unexpected {character:?}"),
            Self::Unterminated => write!(f, "the input ends inside a value"),
            Self::Trailing => write!(f, "the input continues after the value"),
            Self::Number(text) => write!(f, "{text} is not a number"),
        }
    }
}

/// Reads a JSON value. The dumps only use arrays, strings, numbers, booleans and
/// null, so this reads those and nothing else.
///
/// # Errors
///
/// Returns an error when the input is not JSON of that shape.
pub fn parse(text: &str) -> Result<Sexpr, Error> {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let value = parse_at(bytes, &mut at)?;
    skip_space(bytes, &mut at);
    if at != bytes.len() {
        return Err(Error::Trailing);
    }

    Ok(value)
}

fn skip_space(bytes: &[u8], at: &mut usize) {
    while *at < bytes.len() && bytes[*at].is_ascii_whitespace() {
        *at += 1;
    }
}

fn parse_at(bytes: &[u8], at: &mut usize) -> Result<Sexpr, Error> {
    skip_space(bytes, at);
    let Some(&byte) = bytes.get(*at) else {
        return Err(Error::Unterminated);
    };
    match byte {
        b'[' => {
            *at += 1;
            let mut items = Vec::new();
            skip_space(bytes, at);
            if bytes.get(*at) == Some(&b']') {
                *at += 1;
                return Ok(Sexpr::List(items));
            }
            loop {
                items.push(parse_at(bytes, at)?);
                skip_space(bytes, at);
                match bytes.get(*at) {
                    Some(b',') => *at += 1,
                    Some(b']') => {
                        *at += 1;
                        return Ok(Sexpr::List(items));
                    }
                    Some(&other) => return Err(Error::Unexpected(char::from(other))),
                    None => return Err(Error::Unterminated),
                }
            }
        }
        b'"' => {
            *at += 1;
            let mut text = String::new();
            loop {
                let Some(&next) = bytes.get(*at) else {
                    return Err(Error::Unterminated);
                };
                *at += 1;
                match next {
                    b'"' => return Ok(Sexpr::Atom(text)),
                    b'\\' => {
                        let Some(&escape) = bytes.get(*at) else {
                            return Err(Error::Unterminated);
                        };
                        *at += 1;
                        text.push(match escape {
                            b'n' => '\n',
                            b't' => '\t',
                            b'r' => '\r',
                            b'"' => '"',
                            b'\\' => '\\',
                            b'u' => '?',
                            other => char::from(other),
                        });
                    }
                    other => text.push(char::from(other)),
                }
            }
        }
        b't' | b'f' | b'n' => {
            let word = ["true", "false", "null"]
                .into_iter()
                .find(|word| bytes[*at..].starts_with(word.as_bytes()))
                .ok_or_else(|| Error::Unexpected(char::from(byte)))?;
            *at += word.len();
            Ok(match word {
                "true" => Sexpr::Bool(true),
                "false" => Sexpr::Bool(false),
                _ => Sexpr::Null,
            })
        }
        _ => {
            let start = *at;
            while *at < bytes.len()
                && matches!(bytes[*at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
            {
                *at += 1;
            }
            let text = std::str::from_utf8(&bytes[start..*at]).map_err(|_| Error::Unterminated)?;
            let value = text
                .parse::<f64>()
                .map_err(|_| Error::Number(text.to_string()))?;
            Ok(Sexpr::Number(value))
        }
    }
}

/// How many times each form appears, by name.
///
/// An S-expression whose first element is not an atom is an invocation of a
/// computed form, which PTL does not have, so it is counted under `"<call>"` and
/// named rather than dropped.
#[must_use]
pub fn inventory(program: &Sexpr) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for list in program.lists() {
        let Sexpr::List(items) = list else {
            continue;
        };
        if items.len() < 2 {
            continue;
        }
        let name = items[0]
            .atom()
            .map_or_else(|| "<call>".to_string(), str::to_string);
        *counts.entry(name).or_insert(0) += 1;
    }

    counts
}

/// The macros `meta/macros.ptl` defines, less the operator forms.
///
/// A translator must not treat these as calls: a macro that reaches it unexpanded
/// is a gap in the expander, and emitting `glyph_proc(...)` would hide it behind
/// code that does not compile for a reason nobody can see.
pub const MACROS: &[&str] = &[
    "Just",
    "lift-",
    "with-params",
    "flex-params",
    "glyph-proc",
    "composite-proc",
    "set-width",
    "include",
    "set-mark-anchor",
    "set-base-anchor",
    "eject-contour",
    "new-glyph",
    "create-glyph",
    "create-aliased-glyph",
    "create-forked-glyph",
    "glyph-is-needed",
    "glyph-module",
    "run-glyph-module",
    "glyph-block-import",
    "glyph-block-export",
    "for-width-kinds",
    "end-glyph-block",
    "glyph-block",
];

/// The form a node names, if its head is one of `names`.
fn named<'a>(node: &'a Sexpr, names: &[&str]) -> Option<&'a str> {
    let form = node.form()?;
    names.contains(&form).then_some(form)
}

/// Rust for one expanded node.
///
/// # Errors
///
/// Returns the head it cannot translate. A macro head is one of those: the
/// expander's job is to have removed it first.
pub fn translate(node: &Sexpr) -> Result<String, String> {
    match node {
        Sexpr::Atom(text) => Ok(text.clone()),
        Sexpr::Number(value) => Ok(format!("{value}")),
        Sexpr::Bool(value) => Ok(format!("{value}")),
        Sexpr::Null => Ok("None".to_string()),
        Sexpr::List(items) => {
            let Some(head) = node.form() else {
                return Err("<call>".to_string());
            };
            if MACROS.contains(&head) {
                return Err(head.to_string());
            }
            let args =
                || -> Result<Vec<String>, String> { items[1..].iter().map(translate).collect() };
            match head {
                ".quote" => {
                    let quoted = items.get(1).ok_or(".quote without a value")?;
                    translate(quoted)
                }
                "." => {
                    let args = args()?;
                    Ok(args.join("."))
                }
                ".operatorPiece" => {
                    let args = args()?;
                    if args.len() == 3 {
                        Ok(format!("({} {} {})", args[0], args[1], args[2]))
                    } else {
                        Err(".operatorPiece".to_string())
                    }
                }
                "begin" => Ok(format!("{{ {} }}", args()?.join(" "))),
                "local" => {
                    let args = args()?;
                    Ok(format!("let {} = {};", args[0], args[1]))
                }
                "define" => {
                    let args = args()?;
                    Ok(format!("let {} = {};", args[0], args[1]))
                }
                "return" => translate(items.get(1).ok_or("return without a value")?),
                "if" => {
                    let args = args()?;
                    if args.len() == 3 {
                        Ok(format!(
                            "if {} {{ {} }} else {{ {} }}",
                            args[0], args[1], args[2]
                        ))
                    } else {
                        Err("if".to_string())
                    }
                }
                "function" => {
                    let args = args()?;
                    Ok(format!("|{}| {{ {} }}", args[0], args[1]))
                }
                "match" => {
                    // Arms are `[pattern, body]` pairs. The expander has already
                    // removed `Just`, which is identity, so a pattern is a
                    // literal, a name or the wildcard.
                    let subject = translate(items.get(1).ok_or("match without a subject")?)?;
                    let mut arms = Vec::new();
                    for arm in &items[2..] {
                        let Sexpr::List(pair) = arm else {
                            return Err("match".to_string());
                        };
                        let [pattern, body] = pair.as_slice() else {
                            return Err("match".to_string());
                        };
                        let pattern = match pattern {
                            Sexpr::Atom(name) if name == "__" => "_".to_string(),
                            Sexpr::Atom(name) => name.clone(),
                            Sexpr::Number(value) => format!("{value}"),
                            _ => return Err("match".to_string()),
                        };
                        arms.push(format!("{pattern} => {}", translate(body)?));
                    }
                    Ok(format!("match {subject} {{ {} }}", arms.join(", ")))
                }
                "object" => Err("object".to_string()),
                _ if named(node, MACROS).is_some() => Err(head.to_string()),
                _ => Ok(format!("{head}({})", args()?.join(", "))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, translate};

    fn rust(text: &str) -> String {
        translate(&parse(text).expect("parse")).expect("translate")
    }

    #[test]
    fn a_local_becomes_a_let() {
        assert_eq!(
            rust(r#"["begin",["local","sw",["Sw","df"]],["return","sw"]]"#),
            "{ let sw = Sw(df); sw }"
        );
    }

    #[test]
    fn an_operator_piece_keeps_its_grouping() {
        assert_eq!(
            rust(r#"[".operatorPiece","barTop","-",[".","df",[".quote","y"]]]"#),
            "(barTop - df.y)"
        );
    }

    #[test]
    fn a_macro_head_is_refused_rather_than_called() {
        let node = parse(r#"["glyph-proc",["include","x"]]"#).expect("parse");
        assert_eq!(translate(&node).expect_err("refused"), "glyph-proc");
    }

    #[test]
    fn a_match_becomes_a_rust_match() {
        assert_eq!(
            rust(r#"["match","kind",["0",["list","a"]],["__",["list","b"]]]"#),
            "match kind { 0 => list(a), _ => list(b) }"
        );
    }

    #[test]
    fn a_member_access_is_a_dot() {
        assert_eq!(rust(r#"[".","df",[".quote","rightSB"]]"#), "df.rightSB");
    }
}
