use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{TideWindow, Timestamp};

pub fn parse_tides(data: &[u8], base: usize) -> ParseResult<Vec<TideWindow>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 8192 {
        return Err(ParseError::limit(cursor.absolute_position(), "tide count"));
    }
    let mut tides = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        tides.push(parse_tide(&mut cursor)?);
    }
    Ok(tides)
}

pub fn parse_tide(cursor: &mut ByteCursor<'_>) -> ParseResult<TideWindow> {
    let basin_code = cursor.read_u16()?;
    let start = Timestamp(cursor.read_u64()?);
    let duration = cursor.read_u32()? as u64;
    let min_depth_cm = cursor.read_u16()?;
    let max_current_cms = cursor.read_u16()?;
    let direction_degrees = cursor.read_u16()?;
    let confidence = cursor.read_u8()?;
    Ok(TideWindow {
        basin_code,
        start,
        end: start.saturating_add(duration),
        min_depth_cm,
        max_current_cms,
        direction_degrees,
        confidence,
    })
}

pub fn best_window(
    tides: &[TideWindow],
    basin_code: u16,
    earliest: Timestamp,
    latest: Timestamp,
    draught_cm: u16,
) -> Option<&TideWindow> {
    tides
        .iter()
        .filter(|tide| tide.basin_code == basin_code)
        .filter(|tide| tide.overlaps(earliest, latest))
        .filter(|tide| tide.supports_draught(draught_cm))
        .max_by_key(|tide| (tide.confidence, tide.min_depth_cm))
}

pub fn merge_adjacent(mut tides: Vec<TideWindow>) -> Vec<TideWindow> {
    tides.sort_by_key(|tide| (tide.basin_code, tide.start.0, tide.end.0));
    let mut out: Vec<TideWindow> = Vec::new();
    for tide in tides {
        if let Some(last) = out.last_mut() {
            if last.basin_code == tide.basin_code
                && last.end.0 >= tide.start.0
                && last.min_depth_cm == tide.min_depth_cm
            {
                last.end = Timestamp(last.end.0.max(tide.end.0));
                last.confidence = last.confidence.max(tide.confidence);
                continue;
            }
        }
        out.push(tide);
    }
    out
}
