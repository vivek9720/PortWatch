use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{Coordinates, MovementStatus, Timestamp, VesselClass, VesselId, VesselSnapshot};

pub fn parse_vessels(data: &[u8], base: usize) -> ParseResult<Vec<VesselSnapshot>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 4096 {
        return Err(ParseError::limit(cursor.absolute_position(), "vessel count"));
    }
    let mut vessels = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        vessels.push(parse_vessel(&mut cursor)?);
    }
    Ok(vessels)
}

pub fn parse_vessel(cursor: &mut ByteCursor<'_>) -> ParseResult<VesselSnapshot> {
    let id = VesselId(cursor.read_u32()?);
    let mmsi = cursor.read_u32()?;
    let name_code = cursor.read_u16()?;
    let callsign_code = cursor.read_u16()?;
    let class = VesselClass::from_byte(cursor.read_u8()?);
    let status = MovementStatus::from_byte(cursor.read_u8()?);
    let lat_e7 = cursor.read_i32()?;
    let lon_e7 = cursor.read_i32()?;
    let speed_tenths = cursor.read_u16()?;
    let heading_degrees = cursor.read_u16()?;
    let timestamp = Timestamp(cursor.read_u64()?);
    let draught_cm = cursor.read_u16()?;
    let destination_code = cursor.read_u16()?;
    Ok(VesselSnapshot {
        id,
        mmsi,
        name_code,
        callsign_code,
        class,
        status,
        position: Coordinates::new(lat_e7, lon_e7),
        speed_tenths,
        heading_degrees,
        timestamp,
        draught_cm,
        destination_code,
    })
}

pub fn parse_delta_vessels(data: &[u8], base: usize) -> ParseResult<Vec<VesselSnapshot>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 4096 {
        return Err(ParseError::limit(cursor.absolute_position(), "vessel delta count"));
    }
    let mut last_lat = 0i32;
    let mut last_lon = 0i32;
    let mut last_time = 0u64;
    let mut out = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        let id = VesselId(cursor.read_u32()?);
        let mmsi = cursor.read_u32()?;
        let name_code = cursor.read_u16()?;
        let callsign_code = cursor.read_u16()?;
        let class = VesselClass::from_byte(cursor.read_u8()?);
        let status = MovementStatus::from_byte(cursor.read_u8()?);
        last_lat = last_lat.wrapping_add(cursor.read_i32()?);
        last_lon = last_lon.wrapping_add(cursor.read_i32()?);
        let delta_time = cursor.read_varint("vessel delta time")?;
        last_time = last_time.saturating_add(delta_time);
        let speed_tenths = cursor.read_u16()?;
        let heading_degrees = cursor.read_u16()?;
        let draught_cm = cursor.read_u16()?;
        let destination_code = cursor.read_u16()?;
        out.push(VesselSnapshot {
            id,
            mmsi,
            name_code,
            callsign_code,
            class,
            status,
            position: Coordinates::new(last_lat, last_lon),
            speed_tenths,
            heading_degrees,
            timestamp: Timestamp(last_time),
            draught_cm,
            destination_code,
        });
    }
    Ok(out)
}

pub fn summarize_tracks(vessels: &[VesselSnapshot]) -> TrackSummary {
    let mut summary = TrackSummary::default();
    for vessel in vessels {
        summary.count += 1;
        summary.max_speed = summary.max_speed.max(vessel.speed_tenths);
        summary.max_draught = summary.max_draught.max(vessel.draught_cm);
        if !vessel.position.is_plausible() {
            summary.implausible_positions += 1;
        }
        if vessel.is_stationary() {
            summary.stationary += 1;
        }
    }
    summary
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackSummary {
    pub count: usize,
    pub stationary: usize,
    pub implausible_positions: usize,
    pub max_speed: u16,
    pub max_draught: u16,
}

pub fn normalize_heading(value: u16) -> u16 {
    if value >= 360 {
        value % 360
    } else {
        value
    }
}

pub fn track_sort_key(vessel: &VesselSnapshot) -> (u64, u32) {
    (vessel.timestamp.0, vessel.id.0)
}

pub fn deduplicate_tracks(mut vessels: Vec<VesselSnapshot>) -> Vec<VesselSnapshot> {
    vessels.sort_by_key(track_sort_key);
    vessels.dedup_by(|a, b| a.id == b.id && a.timestamp == b.timestamp);
    vessels
}
