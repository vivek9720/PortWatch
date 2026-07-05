use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Vessel = 0x01,
    Berth = 0x02,
    Cargo = 0x03,
    Tide = 0x04,
    Route = 0x05,
    Pilot = 0x06,
    Notice = 0x07,
    Dictionary = 0x08,
    Template = 0x09,
    Ledger = 0x0a,
    Segment = 0x0b,
    Signature = 0x0c,
    Unknown = 0xff,
}

impl Tag {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x01 => Tag::Vessel,
            0x02 => Tag::Berth,
            0x03 => Tag::Cargo,
            0x04 => Tag::Tide,
            0x05 => Tag::Route,
            0x06 => Tag::Pilot,
            0x07 => Tag::Notice,
            0x08 => Tag::Dictionary,
            0x09 => Tag::Template,
            0x0a => Tag::Ledger,
            0x0b => Tag::Segment,
            0x0c => Tag::Signature,
            _ => Tag::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tag::Vessel => "vessel",
            Tag::Berth => "berth",
            Tag::Cargo => "cargo",
            Tag::Tide => "tide",
            Tag::Route => "route",
            Tag::Pilot => "pilot",
            Tag::Notice => "notice",
            Tag::Dictionary => "dictionary",
            Tag::Template => "template",
            Tag::Ledger => "ledger",
            Tag::Segment => "segment",
            Tag::Signature => "signature",
            Tag::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tlv<'a> {
    pub tag: Tag,
    pub raw_tag: u8,
    pub flags: u8,
    pub value: &'a [u8],
    pub offset: usize,
}

impl<'a> Tlv<'a> {
    pub fn cursor(&self) -> ByteCursor<'a> {
        ByteCursor::with_base(self.value, self.offset + 4)
    }

    pub fn is_critical(&self) -> bool {
        self.flags & 0x80 != 0
    }

    pub fn is_compressed(&self) -> bool {
        self.flags & 0x40 != 0
    }

    pub fn is_fragmented(&self) -> bool {
        self.flags & 0x20 != 0
    }
}

pub fn parse_all(data: &[u8], base: usize) -> ParseResult<Vec<Tlv<'_>>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let mut out = Vec::new();
    while !cursor.is_empty() {
        out.push(read_tlv(&mut cursor)?);
    }
    Ok(out)
}

pub fn read_tlv<'a>(cursor: &mut ByteCursor<'a>) -> ParseResult<Tlv<'a>> {
    let offset = cursor.absolute_position();
    let raw_tag = cursor.read_u8()?;
    let flags = cursor.read_u8()?;
    let len = cursor.read_u16()? as usize;
    let value = cursor.read_slice(len, "tlv value")?;
    Ok(Tlv {
        tag: Tag::from_byte(raw_tag),
        raw_tag,
        flags,
        value,
        offset,
    })
}

pub fn read_tlv32<'a>(cursor: &mut ByteCursor<'a>) -> ParseResult<Tlv<'a>> {
    let offset = cursor.absolute_position();
    let raw_tag = cursor.read_u8()?;
    let flags = cursor.read_u8()?;
    let len = cursor.read_u32()? as usize;
    let value = cursor.read_slice(len, "tlv32 value")?;
    Ok(Tlv {
        tag: Tag::from_byte(raw_tag),
        raw_tag,
        flags,
        value,
        offset,
    })
}

pub fn expect_only_known(entries: &[Tlv<'_>]) -> ParseResult<()> {
    for entry in entries {
        if entry.tag == Tag::Unknown && entry.is_critical() {
            return Err(ParseError::unknown_section(entry.offset, "critical tlv"));
        }
    }
    Ok(())
}

pub fn first<'a>(entries: &'a [Tlv<'a>], tag: Tag) -> Option<Tlv<'a>> {
    entries.iter().copied().find(|entry| entry.tag == tag)
}

pub fn values_for<'a>(entries: &'a [Tlv<'a>], tag: Tag) -> Vec<Tlv<'a>> {
    entries
        .iter()
        .copied()
        .filter(|entry| entry.tag == tag)
        .collect()
}
