//! The subset of TOML the parameter data is written in.
//!
//! Upstream reads `params/*.toml` with a full TOML library. The four files this
//! port consumes use tables with numeric, negative and quoted keys, dotted
//! keys, trailing comments, literal and triple-literal strings, integers,
//! floats, booleans and arrays. That is the subset implemented here, and
//! `tests/param.rs` parses the real files rather than fixtures.

use std::collections::BTreeMap;

/// A TOML value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Table(BTreeMap<String, Value>),
}

impl Value {
    /// The table this value is, when it is one.
    #[must_use]
    pub const fn as_table(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Table(table) => Some(table),
            _ => None,
        }
    }

    /// The number this value is, when it is one.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            Self::Int(value) => Some(*value as f64),
            _ => None,
        }
    }

    /// The string this value is, when it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(value) => Some(value),
            _ => None,
        }
    }

    /// The boolean this value is, when it is one.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

/// Parses a document.
///
/// # Errors
///
/// Returns a message naming the byte offset when the text is not in the subset.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        position: 0,
    };
    let mut root = BTreeMap::new();
    let mut current: Vec<String> = Vec::new();
    loop {
        reader.skip_whitespace_and_comments();
        if reader.position >= reader.bytes.len() {
            break;
        }
        if reader.peek() == Some(b'[') {
            reader.advance();
            let table = reader.parse_table_header()?;
            current = table;
            continue;
        }
        let keys = reader.parse_key_path()?;
        reader.skip_inline_space();
        reader.expect(b'=')?;
        reader.skip_inline_space();
        let value = reader.parse_value()?;
        let mut path = current.clone();
        path.extend(keys);
        insert(&mut root, &path, value)?;
    }
    Ok(Value::Table(root))
}

fn insert(root: &mut BTreeMap<String, Value>, path: &[String], value: Value) -> Result<(), String> {
    let Some((head, rest)) = path.split_first() else {
        return Err("empty key".to_string());
    };
    if rest.is_empty() {
        if root.contains_key(head) {
            return Err(format!("{head} is defined twice"));
        }
        root.insert(head.clone(), value);
        return Ok(());
    }
    let entry = root
        .entry(head.clone())
        .or_insert_with(|| Value::Table(BTreeMap::new()));
    let Value::Table(table) = entry else {
        return Err(format!("{head} is a value and a table"));
    };
    insert(table, rest, value)
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    const fn advance(&mut self) {
        self.position += 1;
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.peek() == Some(byte) {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {:?} at {}", byte as char, self.position))
        }
    }

    fn skip_inline_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.advance();
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.advance(),
                Some(b'#') => {
                    while !matches!(self.peek(), None | Some(b'\n')) {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn parse_table_header(&mut self) -> Result<Vec<String>, String> {
        let mut path = Vec::new();
        loop {
            path.push(self.parse_table_key()?);
            match self.peek() {
                Some(b'.') => self.advance(),
                Some(b']') => {
                    self.advance();
                    return Ok(path);
                }
                other => {
                    return Err(format!(
                        "unexpected {:?} in a table header at {}",
                        other.map(|b| b as char),
                        self.position
                    ));
                }
            }
        }
    }

    /// A table key may be bare, quoted, or a number, and a negative number is
    /// the shape `shapeSlopeAngle.blend.-15` takes.
    fn parse_table_key(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(b'"' | b'\'') => self.parse_string(),
            Some(b'-' | b'0'..=b'9') => {
                let start = self.position;
                if self.peek() == Some(b'-') {
                    self.advance();
                }
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.advance();
                }
                Ok(String::from_utf8_lossy(&self.bytes[start..self.position]).to_string())
            }
            _ => self.parse_bare_key(),
        }
    }

    fn parse_bare_key(&mut self) -> Result<String, String> {
        let start = self.position;
        while matches!(self.peek(), Some(b) if b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            self.advance();
        }
        if self.position == start {
            return Err(format!("expected a key at {}", self.position));
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.position]).to_string())
    }

    fn parse_key_path(&mut self) -> Result<Vec<String>, String> {
        let mut path = vec![self.parse_dotted_key()?];
        loop {
            self.skip_inline_space();
            if self.peek() != Some(b'.') {
                return Ok(path);
            }
            self.advance();
            self.skip_inline_space();
            path.push(self.parse_dotted_key()?);
        }
    }

    fn parse_dotted_key(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(b'"' | b'\'') => self.parse_string(),
            _ => self.parse_bare_key(),
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'"' | b'\'') => Ok(Value::Str(self.parse_string()?)),
            Some(b'[') => self.parse_array(),
            Some(b't' | b'f') => self.parse_bool(),
            _ => self.parse_number(),
        }
    }

    fn parse_bool(&mut self) -> Result<Value, String> {
        if self.bytes[self.position..].starts_with(b"true") {
            self.position += 4;
            Ok(Value::Bool(true))
        } else if self.bytes[self.position..].starts_with(b"false") {
            self.position += 5;
            Ok(Value::Bool(false))
        } else {
            Err(format!("expected a boolean at {}", self.position))
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.position;
        if self.peek() == Some(b'-') || self.peek() == Some(b'+') {
            self.advance();
        }
        let mut float = false;
        while let Some(byte) = self.peek() {
            match byte {
                b'0'..=b'9' => self.advance(),
                b'.' | b'e' | b'E' => {
                    float = true;
                    self.advance();
                }
                b'+' | b'-' if matches!(self.bytes[self.position - 1], b'e' | b'E') => {
                    self.advance();
                }
                _ => break,
            }
        }
        let text = String::from_utf8_lossy(&self.bytes[start..self.position]).to_string();
        if float {
            text.parse::<f64>()
                .map(Value::Float)
                .map_err(|error| format!("{text}: {error}"))
        } else {
            text.parse::<i64>()
                .map(Value::Int)
                .map_err(|error| format!("{text}: {error}"))
        }
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            match self.peek() {
                Some(b']') => {
                    self.advance();
                    return Ok(Value::Array(items));
                }
                None => return Err("an array was not closed".to_string()),
                _ => {}
            }
            items.push(self.parse_value()?);
            self.skip_whitespace_and_comments();
            if self.peek() == Some(b',') {
                self.advance();
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        let quote = self.peek().expect("a quote");
        let triple = self.bytes[self.position..].starts_with(&[quote, quote, quote]);
        self.position += if triple { 3 } else { 1 };
        let mut out = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err("a string was not closed".to_string());
            };
            if triple && self.bytes[self.position..].starts_with(&[quote, quote, quote]) {
                self.position += 3;
                return Ok(out);
            }
            if !triple && byte == quote {
                self.advance();
                return Ok(out);
            }
            if byte == b'\n' && !triple {
                return Err("a newline inside a single-line string".to_string());
            }
            if byte == b'\\' && quote == b'"' {
                self.advance();
                let escape = self.peek().ok_or("a dangling escape")?;
                self.advance();
                out.push(match escape {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    other => other as char,
                });
                continue;
            }
            if triple && byte == b'\n' {
                self.advance();
                while matches!(self.peek(), Some(b' ' | b'\t')) {
                    self.advance();
                }
                out.push('\n');
                continue;
            }
            self.advance();
            out.push(byte as char);
        }
    }
}
