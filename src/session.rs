use crate::ais;
use crate::analyzer;
use crate::berth;
use crate::cargo;
use crate::codec;
use crate::dictionary::StringDictionary;
use crate::error::{ParseError, ParseResult};
use crate::frame::{self, Frame, FrameKind};
use crate::ledger::VesselLedger;
use crate::manifest;
use crate::model::{ExchangeStats, HarborEvent, PortSnapshot};
use crate::notice;
use crate::pilot;
use crate::route;
use crate::template::TemplateRegistry;
use crate::tide;
use crate::tlv::{self, Tag};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct StreamSummary {
    pub stream_id: u16,
    pub snapshot: PortSnapshot,
    pub stats: ExchangeStats,
    pub ledger_materialized: Vec<String>,
    pub frame_sequences: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct StreamSession {
    stream_id: u16,
    dictionary: StringDictionary,
    templates: TemplateRegistry,
    ledger: VesselLedger,
    snapshot: PortSnapshot,
    stats: ExchangeStats,
    segments: BTreeMap<u32, codec::ReassemblyBuffer>,
    frame_sequences: Vec<u32>,
}

impl StreamSession {
    pub fn new(stream_id: u16) -> Self {
        Self {
            stream_id,
            dictionary: StringDictionary::new(),
            templates: TemplateRegistry::new(),
            ledger: VesselLedger::new(),
            snapshot: PortSnapshot::new(),
            stats: ExchangeStats::default(),
            segments: BTreeMap::new(),
            frame_sequences: Vec::new(),
        }
    }

    pub fn apply_frame(&mut self, frame: &Frame<'_>) -> ParseResult<()> {
        self.stats.frames += 1;
        self.stats.bytes_in += frame.payload.len();
        self.frame_sequences.push(frame.sequence);
        let (payload, codec_stats) = if frame.is_compressed() {
            self.stats.compressed_sections += 1;
            codec::decode_payload(frame.payload, frame.codec_flags(), &self.dictionary)?
        } else {
            codec::decode_payload(frame.payload, 0, &self.dictionary)?
        };
        self.stats.bytes_decoded += codec_stats.output_bytes;
        match frame.kind {
            FrameKind::Dictionary => {
                self.dictionary.apply_delta(&payload, frame.offset)?;
                self.stats.dictionary_entries = self.dictionary.len();
            }
            FrameKind::Template => {
                self.templates.parse_section(&payload, frame.offset)?;
                self.stats.templates = self.templates.len();
            }
            FrameKind::Records => {
                self.apply_records(&payload, frame.offset)?;
            }
            FrameKind::Ledger => {
                let findings = self.ledger.apply_program(&payload, frame.offset)?;
                for finding in findings {
                    let _ = finding;
                }
            }
            FrameKind::Segment => {
                self.apply_segment(&payload, frame.sequence)?;
            }
            FrameKind::Manifest => {
                let _ = manifest::decode_manifest(&payload)?;
            }
            FrameKind::Heartbeat => {}
            FrameKind::Unknown(_) => {
                if frame.flags & 0x40 != 0 {
                    return Err(ParseError::unknown_section(frame.offset, "frame"));
                }
            }
        }
        Ok(())
    }

    pub fn apply_records(&mut self, payload: &[u8], base: usize) -> ParseResult<()> {
        let entries = tlv::parse_all(payload, base)?;
        self.stats.sections += entries.len();
        for entry in entries {
            match entry.tag {
                Tag::Vessel => {
                    for vessel in ais::parse_vessels(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Vessel(vessel));
                    }
                }
                Tag::Berth => {
                    for plan in berth::parse_berths(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Berth(plan));
                    }
                }
                Tag::Cargo => {
                    for lot in cargo::parse_cargo(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Cargo(lot));
                    }
                }
                Tag::Tide => {
                    for window in tide::parse_tides(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Tide(window));
                    }
                }
                Tag::Route => {
                    for leg in route::parse_routes(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Route(leg));
                    }
                }
                Tag::Pilot => {
                    for order in pilot::parse_pilots(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Pilot(order));
                    }
                }
                Tag::Notice => {
                    for notice in notice::parse_notices(entry.value, entry.offset + 4)? {
                        self.snapshot.push(HarborEvent::Notice(notice));
                    }
                }
                Tag::Dictionary => {
                    self.dictionary.apply_delta(entry.value, entry.offset + 4)?;
                    self.stats.dictionary_entries = self.dictionary.len();
                }
                Tag::Template => {
                    self.templates.parse_section(entry.value, entry.offset + 4)?;
                    self.stats.templates = self.templates.len();
                }
                Tag::Ledger => {
                    let _ = self.ledger.apply_program(entry.value, entry.offset + 4)?;
                }
                Tag::Segment => {
                    self.apply_segment(entry.value, entry.offset as u32)?;
                }
                Tag::Signature | Tag::Unknown => {
                    if entry.is_critical() {
                        return Err(ParseError::unknown_section(entry.offset, entry.tag.name()));
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_segment(&mut self, payload: &[u8], fallback_id: u32) -> ParseResult<()> {
        let mut cursor = crate::cursor::ByteCursor::new(payload);
        let group = cursor.read_u32().unwrap_or(fallback_id);
        let part = cursor.read_u16().unwrap_or(0) as usize;
        let total = cursor.read_u16().unwrap_or(1) as usize;
        if total == 0 || total > 512 {
            return Err(ParseError::limit(0, "segment total"));
        }
        let chunk = cursor.take_remaining().to_vec();
        let entry = self
            .segments
            .entry(group)
            .or_insert_with(|| codec::ReassemblyBuffer::new(total));
        entry.insert(part, chunk)?;
        if entry.complete() {
            if let Some(buffer) = self.segments.remove(&group) {
                let assembled = buffer.assemble()?;
                self.apply_records(&assembled, 0)?;
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> ParseResult<StreamSummary> {
        self.snapshot.sort_by_time();
        let materialized = self.ledger.materialized().to_vec();
        Ok(StreamSummary {
            stream_id: self.stream_id,
            snapshot: self.snapshot,
            stats: self.stats,
            ledger_materialized: materialized,
            frame_sequences: self.frame_sequences,
        })
    }
}

pub fn decode_stream(data: &[u8]) -> ParseResult<StreamSummary> {
    if data.starts_with(b"PWX1") {
        let (header, frames) = frame::parse_stream_frames(data)?;
        let mut session = StreamSession::new(header.stream_id);
        for frame in frames {
            session.apply_frame(&frame)?;
        }
        session.finish()
    } else {
        let mut session = StreamSession::new(0);
        session.apply_records(data, 0)?;
        session.finish()
    }
}

pub fn decode_and_analyze(data: &[u8]) -> ParseResult<analyzer::HarborReport> {
    let summary = decode_stream(data)?;
    analyzer::analyze_exchange(summary)
}
