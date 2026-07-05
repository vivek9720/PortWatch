use crate::checksum;
use crate::cursor::ByteCursor;
use crate::dictionary::StringDictionary;
use crate::error::{ParseError, ParseResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecMode {
    Raw,
    Rle,
    Window,
    Dictionary,
}

impl CodecMode {
    pub fn from_flags(flags: u8) -> Self {
        match flags & 0x03 {
            0 => CodecMode::Raw,
            1 => CodecMode::Rle,
            2 => CodecMode::Window,
            _ => CodecMode::Dictionary,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DecodeStats {
    pub literals: usize,
    pub repeats: usize,
    pub backrefs: usize,
    pub dict_refs: usize,
    pub output_bytes: usize,
}

pub fn decode_payload(
    data: &[u8],
    flags: u8,
    dictionary: &StringDictionary,
) -> ParseResult<(Vec<u8>, DecodeStats)> {
    match CodecMode::from_flags(flags) {
        CodecMode::Raw => Ok((
            data.to_vec(),
            DecodeStats {
                literals: data.len(),
                output_bytes: data.len(),
                ..DecodeStats::default()
            },
        )),
        CodecMode::Rle => decode_rle(data),
        CodecMode::Window => decode_window(data),
        CodecMode::Dictionary => decode_dictionary_stream(data, dictionary),
    }
}

pub fn decode_rle(data: &[u8]) -> ParseResult<(Vec<u8>, DecodeStats)> {
    let mut cursor = ByteCursor::new(data);
    let mut out = Vec::new();
    let mut stats = DecodeStats::default();
    while !cursor.is_empty() {
        let control = cursor.read_u8()?;
        if control & 0x80 == 0 {
            let len = (control as usize) + 1;
            let bytes = cursor.read_slice(len, "rle literal")?;
            out.extend_from_slice(bytes);
            stats.literals += len;
        } else {
            let len = ((control & 0x7f) as usize) + 3;
            let value = cursor.read_u8()?;
            out.extend(std::iter::repeat(value).take(len));
            stats.repeats += 1;
        }
        if out.len() > 1 << 20 {
            return Err(ParseError::limit(cursor.absolute_position(), "rle output"));
        }
    }
    stats.output_bytes = out.len();
    Ok((out, stats))
}

pub fn decode_window(data: &[u8]) -> ParseResult<(Vec<u8>, DecodeStats)> {
    let mut cursor = ByteCursor::new(data);
    let mut out = Vec::new();
    let mut stats = DecodeStats::default();
    while !cursor.is_empty() {
        let op = cursor.read_u8()?;
        match op >> 6 {
            0 => {
                let len = (op as usize & 0x3f) + 1;
                let bytes = cursor.read_slice(len, "window literal")?;
                out.extend_from_slice(bytes);
                stats.literals += len;
            }
            1 => {
                let len = (op as usize & 0x3f) + 3;
                let distance = cursor.read_u16()? as usize + 1;
                if distance > out.len() {
                    return Err(ParseError::compression(
                        cursor.absolute_position(),
                        "window backref distance",
                    ));
                }
                for _ in 0..len {
                    let idx = out.len() - distance;
                    let byte = out[idx];
                    out.push(byte);
                }
                stats.backrefs += 1;
            }
            2 => {
                let len = (op as usize & 0x3f) + 4;
                let value = cursor.read_u8()?;
                out.extend(std::iter::repeat(value).take(len));
                stats.repeats += 1;
            }
            _ => {
                let checksum = cursor.read_u32()?;
                if checksum::crc32_iso(&out) != checksum {
                    return Err(ParseError::invalid_checksum(
                        cursor.absolute_position(),
                        "window checkpoint",
                    ));
                }
            }
        }
        if out.len() > 1 << 20 {
            return Err(ParseError::limit(cursor.absolute_position(), "window output"));
        }
    }
    stats.output_bytes = out.len();
    Ok((out, stats))
}

pub fn decode_dictionary_stream(
    data: &[u8],
    dictionary: &StringDictionary,
) -> ParseResult<(Vec<u8>, DecodeStats)> {
    let mut cursor = ByteCursor::new(data);
    let mut out = Vec::new();
    let mut stats = DecodeStats::default();
    while !cursor.is_empty() {
        let op = cursor.read_u8()?;
        match op {
            0x00..=0x3f => {
                let len = op as usize + 1;
                let bytes = cursor.read_slice(len, "dict literal")?;
                out.extend_from_slice(bytes);
                stats.literals += len;
            }
            0x40..=0x7f => {
                let repeat = (op as usize & 0x3f) + 2;
                let code = cursor.read_u16()?;
                if let Some(text) = dictionary.get(code) {
                    for _ in 0..repeat {
                        out.extend_from_slice(text.as_bytes());
                    }
                    stats.dict_refs += 1;
                }
            }
            0x80..=0xbf => {
                let len = (op as usize & 0x3f) + 3;
                let distance = cursor.read_u16()? as usize + 1;
                if distance > out.len() {
                    return Err(ParseError::compression(
                        cursor.absolute_position(),
                        "dict backref distance",
                    ));
                }
                for _ in 0..len {
                    let byte = out[out.len() - distance];
                    out.push(byte);
                }
                stats.backrefs += 1;
            }
            _ => {
                let salt = cursor.read_u32()?;
                let tag = checksum::rolling_tag(salt, &out);
                out.extend_from_slice(&tag.to_le_bytes());
            }
        }
        if out.len() > 1 << 20 {
            return Err(ParseError::limit(cursor.absolute_position(), "dictionary output"));
        }
    }
    stats.output_bytes = out.len();
    Ok((out, stats))
}

pub fn encode_seed_literal(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(32) {
        out.push((chunk.len() - 1) as u8);
        out.extend_from_slice(chunk);
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct ReassemblyBuffer {
    chunks: Vec<Option<Vec<u8>>>,
    total_bytes: usize,
}

impl ReassemblyBuffer {
    pub fn new(parts: usize) -> Self {
        Self {
            chunks: vec![None; parts],
            total_bytes: 0,
        }
    }

    pub fn insert(&mut self, index: usize, data: Vec<u8>) -> ParseResult<()> {
        if index >= self.chunks.len() {
            return Err(ParseError::invalid_value(index, "segment index"));
        }
        if self.chunks[index].is_none() {
            self.total_bytes = self.total_bytes.saturating_add(data.len());
        }
        self.chunks[index] = Some(data);
        Ok(())
    }

    pub fn complete(&self) -> bool {
        self.chunks.iter().all(Option::is_some)
    }

    pub fn assemble(self) -> ParseResult<Vec<u8>> {
        let mut out = Vec::with_capacity(self.total_bytes);
        for chunk in self.chunks {
            let chunk = chunk.ok_or_else(|| ParseError::missing_section(0, "segment"))?;
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }
}
