use crate::checksum;
use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Dictionary,
    Template,
    Records,
    Ledger,
    Segment,
    Manifest,
    Heartbeat,
    Unknown(u8),
}

impl FrameKind {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x01 => FrameKind::Dictionary,
            0x02 => FrameKind::Template,
            0x03 => FrameKind::Records,
            0x04 => FrameKind::Ledger,
            0x05 => FrameKind::Segment,
            0x06 => FrameKind::Manifest,
            0x07 => FrameKind::Heartbeat,
            other => FrameKind::Unknown(other),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FrameKind::Dictionary => "dictionary",
            FrameKind::Template => "template",
            FrameKind::Records => "records",
            FrameKind::Ledger => "ledger",
            FrameKind::Segment => "segment",
            FrameKind::Manifest => "manifest",
            FrameKind::Heartbeat => "heartbeat",
            FrameKind::Unknown(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StreamHeader {
    pub version: u8,
    pub flags: u8,
    pub stream_id: u16,
    pub declared_frames: u16,
}

#[derive(Debug, Clone)]
pub struct Frame<'a> {
    pub kind: FrameKind,
    pub raw_kind: u8,
    pub flags: u8,
    pub sequence: u32,
    pub checksum: u32,
    pub payload: &'a [u8],
    pub offset: usize,
}

impl<'a> Frame<'a> {
    pub fn is_compressed(&self) -> bool {
        self.flags & 0x01 != 0
    }

    pub fn has_checksum(&self) -> bool {
        self.flags & 0x80 != 0
    }

    pub fn codec_flags(&self) -> u8 {
        (self.flags >> 1) & 0x03
    }
}

pub fn parse_stream_frames(data: &[u8]) -> ParseResult<(StreamHeader, Vec<Frame<'_>>)> {
    let mut cursor = ByteCursor::new(data);
    cursor.expect_magic(b"PWX1", "stream magic")?;
    let version = cursor.read_u8()?;
    if version != 1 {
        return Err(ParseError::unsupported_version(
            cursor.absolute_position().saturating_sub(1),
            "stream version",
        ));
    }
    let flags = cursor.read_u8()?;
    let stream_id = cursor.read_u16()?;
    let declared_frames = cursor.read_u16()?;
    let header_crc = cursor.read_u16()?;
    let header = &data[..10.min(data.len())];
    if flags & 0x80 != 0 && checksum::crc16_ccitt(&header[..8]) != header_crc {
        return Err(ParseError::invalid_checksum(8, "stream header"));
    }
    let header = StreamHeader {
        version,
        flags,
        stream_id,
        declared_frames,
    };
    let mut frames = Vec::new();
    while !cursor.is_empty() {
        frames.push(read_frame(&mut cursor)?);
        if declared_frames != 0 && frames.len() >= declared_frames as usize {
            break;
        }
    }
    Ok((header, frames))
}

pub fn read_frame<'a>(cursor: &mut ByteCursor<'a>) -> ParseResult<Frame<'a>> {
    let offset = cursor.absolute_position();
    let raw_kind = cursor.read_u8()?;
    let flags = cursor.read_u8()?;
    let sequence = cursor.read_u32()?;
    let len = cursor.read_u32()? as usize;
    let checksum = cursor.read_u32()?;
    if len > (1 << 22) {
        return Err(ParseError::limit(offset, "frame payload"));
    }
    let payload = cursor.read_slice(len, "frame payload")?;
    let frame = Frame {
        kind: FrameKind::from_byte(raw_kind),
        raw_kind,
        flags,
        sequence,
        checksum,
        payload,
        offset,
    };
    if frame.has_checksum() && checksum::crc32_iso(payload) != checksum {
        return Err(ParseError::invalid_checksum(offset, "frame payload"));
    }
    Ok(frame)
}

pub fn build_frame(kind: u8, flags: u8, sequence: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(kind);
    out.push(flags | 0x80);
    out.extend_from_slice(&sequence.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&checksum::crc32_iso(payload).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn build_stream(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"PWX1");
    out.push(1);
    out.push(0);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    for frame in frames {
        out.extend_from_slice(frame);
    }
    out
}
