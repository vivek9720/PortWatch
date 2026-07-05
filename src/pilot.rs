use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{Coordinates, PilotOrder, Timestamp, VesselId};

pub fn parse_pilots(data: &[u8], base: usize) -> ParseResult<Vec<PilotOrder>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 4096 {
        return Err(ParseError::limit(cursor.absolute_position(), "pilot order count"));
    }
    let mut out = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        out.push(parse_pilot(&mut cursor)?);
    }
    Ok(out)
}

pub fn parse_pilot(cursor: &mut ByteCursor<'_>) -> ParseResult<PilotOrder> {
    let vessel = VesselId(cursor.read_u32()?);
    let pilot_code = cursor.read_u16()?;
    let boarding_time = Timestamp(cursor.read_u64()?);
    let boarding_position = Coordinates::new(cursor.read_i32()?, cursor.read_i32()?);
    let disembark_position = Coordinates::new(cursor.read_i32()?, cursor.read_i32()?);
    let language_mask = cursor.read_u16()?;
    let flags = cursor.read_u16()?;
    Ok(PilotOrder {
        vessel,
        pilot_code,
        boarding_time,
        boarding_position,
        disembark_position,
        language_mask,
        flags,
    })
}

#[derive(Debug, Clone, Default)]
pub struct PilotRoster {
    pub orders: Vec<PilotOrder>,
}

impl PilotRoster {
    pub fn new(orders: Vec<PilotOrder>) -> Self {
        Self { orders }
    }

    pub fn by_vessel(&self, vessel: VesselId) -> Option<&PilotOrder> {
        self.orders.iter().find(|order| order.vessel == vessel)
    }

    pub fn night_boardings(&self) -> usize {
        self.orders
            .iter()
            .filter(|order| order.is_night_boarding())
            .count()
    }

    pub fn language_gaps(&self, required_mask: u16) -> Vec<VesselId> {
        self.orders
            .iter()
            .filter(|order| order.language_mask & required_mask == 0)
            .map(|order| order.vessel)
            .collect()
    }
}
