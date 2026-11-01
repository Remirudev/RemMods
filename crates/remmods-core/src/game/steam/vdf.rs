//! Valve KeyValues text reader (VDF/ACF), following tier1 KeyValues.cpp and utlbuffer.cpp:
//! C `isspace` whitespace, `//` comments where a token may start, quoted strings with Valve's
//! escapes, unquoted tokens, `[$COND]` conditionals accepted and ignored, `#include`/`#base`
//! skipped and never followed. Only the first root block is returned.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VdfError {
    pub kind: VdfErrorKind,
    /// 1-based line of the problem; 0 when the bytes were rejected before parsing.
    pub line: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VdfErrorKind {
    UnexpectedEof,
    UnterminatedString,
    ExpectedOpenBrace,
    UnexpectedCloseBrace,
    EmptyKey,
    TooDeep,
    NotUtf8,
    TooLarge,
}

pub const VDF_MAX_BYTES: usize = 1 << 20;
pub const VDF_MAX_DEPTH: usize = 64;

impl VdfErrorKind {
    /// camelCase code the UI translates (`game.notes.details.*`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnexpectedEof => "unexpectedEof",
            Self::UnterminatedString => "unterminatedString",
            Self::ExpectedOpenBrace => "expectedOpenBrace",
            Self::UnexpectedCloseBrace => "unexpectedCloseBrace",
            Self::EmptyKey => "emptyKey",
            Self::TooDeep => "tooDeep",
            Self::NotUtf8 => "notUtf8",
            Self::TooLarge => "tooLarge",
        }
    }
}

impl fmt::Display for VdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at line {}", self.kind.code(), self.line)
    }
}

impl Vdf {
    /// First entry whose key matches ASCII case-insensitively (Valve's FindKey).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        self.entries()
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Vdf::Str(text) => Some(text),
            Vdf::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(entries) => entries,
            Vdf::Str(_) => &[],
        }
    }
}

/// Rejects files over 1 MiB and non-UTF-8 text (line 0), strips a UTF-8 BOM, then parses.
pub fn parse_vdf_bytes(bytes: &[u8]) -> Result<(String, Vdf), VdfError> {
    if bytes.len() > VDF_MAX_BYTES {
        return Err(VdfError {
            kind: VdfErrorKind::TooLarge,
            line: 0,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| VdfError {
        kind: VdfErrorKind::NotUtf8,
        line: 0,
    })?;
    parse_vdf(text)
}

/// Parses the first root block; directives before it are skipped, content after it ignored.
pub fn parse_vdf(text: &str) -> Result<(String, Vdf), VdfError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lexer = Lexer {
        bytes: text.as_bytes(),
        pos: 0,
        line: 1,
    };
    loop {
        match lexer.next_token()? {
            None => return Err(lexer.error(VdfErrorKind::UnexpectedEof)),
            Some(Token::Text(word)) if is_directive(&word) => {
                lexer.next_token()?;
            }
            Some(Token::Text(name)) => {
                let mut open = lexer.next_token()?;
                if open == Some(Token::Conditional) {
                    open = lexer.next_token()?;
                }
                if open != Some(Token::Open) {
                    return Err(lexer.error(VdfErrorKind::ExpectedOpenBrace));
                }
                let body = parse_body(&mut lexer, 1)?;
                return Ok((name, Vdf::Obj(body)));
            }
            Some(Token::Close) => return Err(lexer.error(VdfErrorKind::UnexpectedCloseBrace)),
            Some(Token::Open | Token::Conditional) => {
                return Err(lexer.error(VdfErrorKind::ExpectedOpenBrace));
            }
        }
    }
}

fn is_directive(word: &str) -> bool {
    word.eq_ignore_ascii_case("#include") || word.eq_ignore_ascii_case("#base")
}

/// `{ key [cond] (text | '{' body) }* '}'`; a conditional where a key is expected is skipped.
fn parse_body(lexer: &mut Lexer<'_>, depth: usize) -> Result<Vec<(String, Vdf)>, VdfError> {
    if depth > VDF_MAX_DEPTH {
        return Err(lexer.error(VdfErrorKind::TooDeep));
    }
    let mut entries = Vec::new();
    loop {
        let key = match lexer.next_token()? {
            None => return Err(lexer.error(VdfErrorKind::UnexpectedEof)),
            Some(Token::Close) => return Ok(entries),
            Some(Token::Conditional) => continue,
            Some(Token::Open) => return Err(lexer.error(VdfErrorKind::EmptyKey)),
            Some(Token::Text(key)) if key.is_empty() => {
                return Err(lexer.error(VdfErrorKind::EmptyKey));
            }
            Some(Token::Text(key)) => key,
        };
        let mut value = lexer.next_token()?;
        if value == Some(Token::Conditional) {
            value = lexer.next_token()?;
        }
        match value {
            None => return Err(lexer.error(VdfErrorKind::UnexpectedEof)),
            Some(Token::Open) => {
                let child = parse_body(lexer, depth + 1)?;
                entries.push((key, Vdf::Obj(child)));
            }
            Some(Token::Text(text)) => entries.push((key, Vdf::Str(text))),
            Some(Token::Close | Token::Conditional) => {
                return Err(lexer.error(VdfErrorKind::UnexpectedCloseBrace));
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Open,
    Close,
    Conditional,
    Text(String),
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: u32,
}

impl Lexer<'_> {
    fn error(&self, kind: VdfErrorKind) -> VdfError {
        VdfError {
            kind,
            line: self.line,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        if byte == b'\n' {
            self.line += 1;
        }
        Some(byte)
    }

    fn skip_space_and_comments(&mut self) {
        while let Some(byte) = self.peek() {
            if is_space(byte) {
                self.bump();
            } else if byte == b'/' && self.bytes.get(self.pos + 1) == Some(&b'/') {
                while let Some(byte) = self.bump() {
                    if byte == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, VdfError> {
        self.skip_space_and_comments();
        let Some(byte) = self.bump() else {
            return Ok(None);
        };
        let token = match byte {
            b'{' => Token::Open,
            b'}' => Token::Close,
            b'"' => Token::Text(self.quoted()?),
            _ => self.unquoted(self.pos - 1),
        };
        Ok(Some(token))
    }

    fn quoted(&mut self) -> Result<String, VdfError> {
        let mut out = Vec::new();
        loop {
            match self.bump() {
                None => return Err(self.error(VdfErrorKind::UnterminatedString)),
                Some(b'"') => break,
                Some(b'\\') => match self.bump() {
                    None => return Err(self.error(VdfErrorKind::UnterminatedString)),
                    Some(b'n') => out.push(b'\n'),
                    Some(b't') => out.push(b'\t'),
                    Some(b'r') => out.push(b'\r'),
                    Some(escaped @ (b'\\' | b'"' | b'\'' | b'?')) => out.push(escaped),
                    Some(other) => out.extend_from_slice(&[b'\\', other]),
                },
                Some(byte) => out.push(byte),
            }
        }
        // Splits happen only at ASCII bytes of valid UTF-8 input, so this is never lossy.
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn unquoted(&mut self, start: usize) -> Token {
        while let Some(byte) = self.peek() {
            if is_space(byte) || matches!(byte, b'"' | b'{' | b'}') {
                break;
            }
            self.pos += 1;
        }
        let raw = String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned();
        if raw.len() >= 2 && raw.starts_with('[') && raw.ends_with(']') {
            Token::Conditional
        } else {
            Token::Text(raw)
        }
    }
}

/// C `isspace`: space, \t, \n, \v, \f, \r.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}
