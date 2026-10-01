//! Minimal dependency-free JSON reader for the committed conformance fixtures.
//!
//! The crate architecture test forbids any `[dev-dependencies]`, so this parses
//! only the subset the fixtures use: objects, arrays, numbers, strings and the
//! booleans (`true`/`false`). Numbers may use exponents (`1.2e-05`), which
//! CPython's `json.dump` emits for small magnitudes.

#[derive(Debug, Clone)]
pub enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    Number(f64),
    String(String),
    Bool(bool),
}

impl Json {
    /// Borrow an object entry, panicking with a clear message when missing.
    pub fn field(&self, key: &str) -> &Json {
        self.get(key)
            .unwrap_or_else(|| panic!("missing JSON field `{key}`"))
    }

    /// Look up an object entry, returning `None` when absent.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries
                .iter()
                .find(|(entry, _)| entry == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// Object entries (empty for non-objects).
    pub fn as_object(&self) -> &[(String, Json)] {
        match self {
            Json::Object(entries) => entries,
            _ => &[],
        }
    }

    /// Array items (empty for non-arrays).
    pub fn as_array(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            _ => &[],
        }
    }

    pub fn number(&self) -> f64 {
        match self {
            Json::Number(value) => *value,
            other => panic!("expected JSON number, found {other:?}"),
        }
    }

    pub fn usize(&self) -> usize {
        let value = self.number();
        assert!(
            value >= 0.0 && value.fract() == 0.0,
            "expected non-negative integer, found {value}"
        );
        value as usize
    }

    pub fn text(&self) -> &str {
        match self {
            Json::String(value) => value,
            other => panic!("expected JSON string, found {other:?}"),
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Json::Bool(value) => *value,
            other => panic!("expected JSON boolean, found {other:?}"),
        }
    }

    pub fn vector(&self) -> Vec<f64> {
        match self {
            Json::Array(items) => items.iter().map(Json::number).collect(),
            other => panic!("expected JSON array, found {other:?}"),
        }
    }

    pub fn matrix(&self) -> Vec<Vec<f64>> {
        match self {
            Json::Array(items) => items.iter().map(Json::vector).collect(),
            other => panic!("expected JSON matrix, found {other:?}"),
        }
    }
}

/// Parse a complete JSON document.
pub fn parse(input: &str) -> Result<Json, String> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(format!("trailing data at byte {}", parser.pos));
    }
    Ok(value)
}

/// Parse a committed fixture, panicking on malformed JSON.
pub fn parse_or_panic(input: &str) -> Json {
    parse(input).expect("fixture must be valid JSON")
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(_) => self.number(),
            None => Err("unexpected end of JSON input".to_string()),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(format!("invalid literal at byte {}", self.pos))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1; // consume '{'
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected ':' at byte {}", self.pos));
            }
            self.pos += 1;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.pos)),
            }
        }
        Ok(Json::Object(entries))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1; // consume '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.value()?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.pos)),
            }
        }
        Ok(Json::Array(items))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.peek() != Some(b'"') {
            return Err(format!("expected '\"' at byte {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while let Some(byte) = self.peek() {
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escaped = self.peek().ok_or("unterminated escape")?;
                    self.pos += 1;
                    out.push(match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        other => other as char,
                    });
                }
                other => out.push(other as char),
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "invalid UTF-8 in JSON number".to_string())?;
        text.parse::<f64>()
            .map(Json::Number)
            .map_err(|_| format!("invalid JSON number `{text}`"))
    }
}
