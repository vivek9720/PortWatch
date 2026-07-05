use crate::checksum;
use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::tlv::{self, Tag};

#[derive(Debug, Clone)]
pub struct HarborManifest {
    pub version: u8,
    pub flags: u8,
    pub created_at: u64,
    pub sections: Vec<ManifestSection>,
    pub tlv_count: usize,
    pub declared_size: u32,
}

#[derive(Debug, Clone)]
pub struct ManifestSection {
    pub kind: ManifestSectionKind,
    pub flags: u16,
    pub offset: u32,
    pub len: u32,
    pub checksum: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestSectionKind {
    Stream,
    Dictionary,
    Cargo,
    Route,
    Attachment,
    Signature,
    Unknown(u8),
}

impl ManifestSectionKind {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x01 => ManifestSectionKind::Stream,
            0x02 => ManifestSectionKind::Dictionary,
            0x03 => ManifestSectionKind::Cargo,
            0x04 => ManifestSectionKind::Route,
            0x05 => ManifestSectionKind::Attachment,
            0x06 => ManifestSectionKind::Signature,
            other => ManifestSectionKind::Unknown(other),
        }
    }
}

pub fn decode_manifest(data: &[u8]) -> ParseResult<HarborManifest> {
    if data.starts_with(b"PWMF") {
        decode_binary_manifest(data)
    } else {
        decode_tlv_manifest(data)
    }
}

pub fn decode_binary_manifest(data: &[u8]) -> ParseResult<HarborManifest> {
    let mut cursor = ByteCursor::new(data);
    cursor.expect_magic(b"PWMF", "manifest magic")?;
    let version = cursor.read_u8()?;
    if version != 1 {
        return Err(ParseError::unsupported_version(
            cursor.absolute_position().saturating_sub(1),
            "manifest version",
        ));
    }
    let flags = cursor.read_u8()?;
    let section_count = cursor.read_u16()? as usize;
    if section_count > 4096 {
        return Err(ParseError::limit(cursor.absolute_position(), "manifest sections"));
    }
    let created_at = cursor.read_u64()?;
    let declared_size = cursor.read_u32()?;
    let mut sections = Vec::with_capacity(section_count.min(64));
    for _ in 0..section_count {
        let raw_kind = cursor.read_u8()?;
        let flags = cursor.read_u16()?;
        let offset = cursor.read_u32()?;
        let len = cursor.read_u32()?;
        let checksum = cursor.read_u32()?;
        let section = ManifestSection {
            kind: ManifestSectionKind::from_byte(raw_kind),
            flags,
            offset,
            len,
            checksum,
        };
        verify_section(data, &section)?;
        sections.push(section);
    }
    Ok(HarborManifest {
        version,
        flags,
        created_at,
        sections,
        tlv_count: 0,
        declared_size,
    })
}

fn verify_section(data: &[u8], section: &ManifestSection) -> ParseResult<()> {
    let start = section.offset as usize;
    let end = start
        .checked_add(section.len as usize)
        .ok_or_else(|| ParseError::invalid_length(start, "manifest section span"))?;
    if end > data.len() {
        return Err(ParseError::eof(start, "manifest section"));
    }
    if section.flags & 0x8000 != 0 {
        let actual = checksum::crc32_iso(&data[start..end]);
        if actual != section.checksum {
            return Err(ParseError::invalid_checksum(start, "manifest section"));
        }
    }
    Ok(())
}

pub fn decode_tlv_manifest(data: &[u8]) -> ParseResult<HarborManifest> {
    let entries = tlv::parse_all(data, 0)?;
    tlv::expect_only_known(&entries)?;
    let mut sections = Vec::new();
    for entry in &entries {
        let kind = match entry.tag {
            Tag::Segment => ManifestSectionKind::Attachment,
            Tag::Dictionary => ManifestSectionKind::Dictionary,
            Tag::Cargo => ManifestSectionKind::Cargo,
            Tag::Route => ManifestSectionKind::Route,
            Tag::Signature => ManifestSectionKind::Signature,
            _ => ManifestSectionKind::Unknown(entry.raw_tag),
        };
        sections.push(ManifestSection {
            kind,
            flags: entry.flags as u16,
            offset: entry.offset as u32,
            len: entry.value.len() as u32,
            checksum: checksum::crc32_iso(entry.value),
        });
    }
    Ok(HarborManifest {
        version: 1,
        flags: 0,
        created_at: 0,
        sections,
        tlv_count: entries.len(),
        declared_size: data.len() as u32,
    })
}

impl HarborManifest {
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn attachments(&self) -> impl Iterator<Item = &ManifestSection> {
        self.sections
            .iter()
            .filter(|section| section.kind == ManifestSectionKind::Attachment)
    }

    pub fn verified_sections(&self) -> usize {
        self.sections
            .iter()
            .filter(|section| section.flags & 0x8000 != 0)
            .count()
    }

    pub fn total_declared_bytes(&self) -> u64 {
        self.sections.iter().map(|section| section.len as u64).sum()
    }
}
