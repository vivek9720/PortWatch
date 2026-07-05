pub mod ais;
pub mod analyzer;
pub mod berth;
pub mod cargo;
pub mod catalog;
pub mod checksum;
pub mod codec;
pub mod cursor;
pub mod dictionary;
pub mod error;
pub mod frame;
pub mod graph;
pub mod ledger;
pub mod manifest;
pub mod model;
pub mod notice;
pub mod pilot;
pub mod planner;
pub mod report;
pub mod route;
pub mod session;
pub mod template;
pub mod tide;
pub mod tlv;
pub mod validate;

pub use analyzer::{analyze_exchange, HarborReport};
pub use error::{ParseError, ParseResult};
pub use manifest::{decode_manifest, HarborManifest};
pub use session::{decode_stream, StreamSummary};

pub fn parse(data: &[u8]) -> ParseResult<HarborReport> {
    let summary = decode_stream(data)?;
    analyze_exchange(summary)
}

pub fn parse_manifest(data: &[u8]) -> ParseResult<HarborManifest> {
    decode_manifest(data)
}

pub fn fuzz_stream(data: &[u8]) {
    let _ = parse(data);
}

pub fn fuzz_manifest(data: &[u8]) {
    let _ = parse_manifest(data);
}

pub fn fuzz_ledger(data: &[u8]) {
    let _ = ledger::decode_ledger_program(data);
}
