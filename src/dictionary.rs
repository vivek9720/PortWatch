use crate::checksum;
use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct StringDictionary {
    entries: BTreeMap<u16, DictEntry>,
    aliases: BTreeMap<u16, u16>,
    generation: u32,
    revision_tag: u32,
}

#[derive(Debug, Clone)]
struct DictEntry {
    text: String,
    generation: u32,
    flags: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct NameLease {
    ptr: *const u8,
    len: usize,
    code: u16,
    generation: u32,
    checksum: u32,
}

impl NameLease {
    pub fn empty(code: u16, generation: u32) -> Self {
        Self {
            ptr: std::ptr::NonNull::<u8>::dangling().as_ptr(),
            len: 0,
            code,
            generation,
            checksum: 0,
        }
    }

    pub fn code(self) -> u16 {
        self.code
    }

    pub fn generation(self) -> u32 {
        self.generation
    }

    pub fn checksum(self) -> u32 {
        self.checksum
    }

    pub fn len(self) -> usize {
        self.len
    }

    pub fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn bytes(self) -> Vec<u8> {
        if self.len == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(self.ptr, self.len).to_vec() }
    }

    pub fn text_lossy(self) -> String {
        String::from_utf8_lossy(&self.bytes()).into_owned()
    }
}

impl StringDictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn revision_tag(&self) -> u32 {
        self.revision_tag
    }

    pub fn insert(&mut self, code: u16, text: String, flags: u16) {
        self.revision_tag = checksum::rolling_tag(self.revision_tag ^ code as u32, text.as_bytes());
        self.entries.insert(
            code,
            DictEntry {
                text,
                generation: self.generation,
                flags,
            },
        );
    }

    pub fn alias(&mut self, alias: u16, target: u16) {
        self.aliases.insert(alias, target);
    }

    pub fn resolve_code(&self, code: u16) -> u16 {
        let mut current = code;
        for _ in 0..8 {
            match self.aliases.get(&current) {
                Some(next) if *next != current => current = *next,
                _ => break,
            }
        }
        current
    }

    pub fn get(&self, code: u16) -> Option<&str> {
        let resolved = self.resolve_code(code);
        self.entries.get(&resolved).map(|entry| entry.text.as_str())
    }

    pub fn flags(&self, code: u16) -> u16 {
        let resolved = self.resolve_code(code);
        self.entries
            .get(&resolved)
            .map(|entry| entry.flags)
            .unwrap_or_default()
    }

    pub fn lease(&self, code: u16) -> Option<NameLease> {
        let resolved = self.resolve_code(code);
        self.entries.get(&resolved).map(|entry| NameLease {
            ptr: entry.text.as_ptr(),
            len: entry.text.len(),
            code: resolved,
            generation: entry.generation,
            checksum: checksum::rolling_tag(resolved as u32, entry.text.as_bytes()),
        })
    }

    pub fn apply_delta(&mut self, data: &[u8], base: usize) -> ParseResult<()> {
        let mut cursor = ByteCursor::with_base(data, base);
        let op_count = cursor.read_u16()? as usize;
        if op_count > 16_384 {
            return Err(ParseError::limit(cursor.absolute_position(), "dictionary op count"));
        }
        for _ in 0..op_count {
            let op = cursor.read_u8()?;
            match op {
                0x01 => {
                    let code = cursor.read_u16()?;
                    let flags = cursor.read_u16()?;
                    let bytes = cursor.read_length_prefixed("dictionary text")?;
                    let text = std::str::from_utf8(bytes)
                        .map_err(|_| ParseError::invalid_utf8(cursor.absolute_position(), "dict"))?
                        .to_owned();
                    self.insert(code, text, flags);
                }
                0x02 => {
                    let alias = cursor.read_u16()?;
                    let target = cursor.read_u16()?;
                    self.alias(alias, target);
                }
                0x03 => {
                    let code = cursor.read_u16()?;
                    self.entries.remove(&code);
                    self.aliases.retain(|alias, target| *alias != code && *target != code);
                }
                0x04 => {
                    let threshold = cursor.read_u16()?;
                    self.compact(threshold);
                }
                0x05 => {
                    let salt = cursor.read_u32()?;
                    self.revision_tag ^= salt.rotate_left((self.generation % 31) + 1);
                }
                _ => {
                    return Err(ParseError::invalid_tag(
                        cursor.absolute_position().saturating_sub(1),
                        "dictionary op",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn replace_from(&mut self, data: &[u8], base: usize) -> ParseResult<()> {
        self.entries.clear();
        self.aliases.clear();
        self.generation = self.generation.wrapping_add(1);
        self.apply_delta(data, base)
    }

    pub fn compact(&mut self, keep_flags: u16) {
        self.generation = self.generation.wrapping_add(1);
        self.entries.retain(|_, entry| entry.flags & keep_flags != 0);
        for entry in self.entries.values_mut() {
            let mut text = String::with_capacity(entry.text.len());
            for ch in entry.text.chars() {
                if !ch.is_control() {
                    text.push(ch);
                }
            }
            entry.text = text;
            entry.generation = self.generation;
        }
        let live: BTreeSet<u16> = self.entries.keys().copied().collect();
        self.aliases.retain(|_, target| live.contains(target));
    }

    pub fn parse_full(data: &[u8], base: usize) -> ParseResult<Self> {
        let mut dictionary = StringDictionary::new();
        dictionary.apply_delta(data, base)?;
        Ok(dictionary)
    }

    pub fn snapshot_codes(&self) -> Vec<u16> {
        self.entries.keys().copied().collect()
    }

    pub fn normalized_texts(&self) -> Vec<(u16, String)> {
        self.entries
            .iter()
            .map(|(code, entry)| (*code, normalize_name(&entry.text)))
            .collect()
    }
}

pub fn parse_dictionary(data: &[u8], base: usize) -> ParseResult<StringDictionary> {
    StringDictionary::parse_full(data, base)
}

pub fn normalize_name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
            last_space = false;
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !last_space && !out.is_empty() {
                out.push(' ');
            }
            last_space = true;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

pub fn name_similarity(left: &str, right: &str) -> u16 {
    let left = normalize_name(left);
    let right = normalize_name(right);
    if left == right {
        return 1000;
    }
    let mut score = 0u16;
    for token in left.split(' ') {
        if !token.is_empty() && right.contains(token) {
            score = score.saturating_add(80);
        }
    }
    score.min(1000)
}
