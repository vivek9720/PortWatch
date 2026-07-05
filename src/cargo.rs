use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{CargoLot, HazardClass, VesselId};

pub fn parse_cargo(data: &[u8], base: usize) -> ParseResult<Vec<CargoLot>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 16_384 {
        return Err(ParseError::limit(cursor.absolute_position(), "cargo lot count"));
    }
    let mut lots = Vec::with_capacity(count.min(256));
    for _ in 0..count {
        lots.push(parse_lot(&mut cursor)?);
    }
    Ok(lots)
}

pub fn parse_lot(cursor: &mut ByteCursor<'_>) -> ParseResult<CargoLot> {
    let vessel = VesselId(cursor.read_u32()?);
    let un_number = cursor.read_u16()?;
    let hazard = HazardClass::from_byte(cursor.read_u8()?);
    let tonnes = cursor.read_u32()?;
    let package_code = cursor.read_u16()?;
    let stowage_zone = cursor.read_u8()?;
    let temperature_tenths = cursor.read_i16()?;
    let flags = cursor.read_u16()?;
    Ok(CargoLot {
        vessel,
        un_number,
        hazard,
        tonnes,
        package_code,
        stowage_zone,
        temperature_tenths,
        flags,
    })
}

pub fn incompatible(left: &CargoLot, right: &CargoLot) -> bool {
    if left.vessel == right.vessel {
        return false;
    }
    if left.stowage_zone != right.stowage_zone {
        return false;
    }
    match (left.hazard, right.hazard) {
        (HazardClass::Explosive, _) | (_, HazardClass::Explosive) => true,
        (HazardClass::Oxidizer, HazardClass::FlammableLiquid)
        | (HazardClass::FlammableLiquid, HazardClass::Oxidizer) => true,
        (HazardClass::Toxic, HazardClass::Miscellaneous)
        | (HazardClass::Miscellaneous, HazardClass::Toxic) => true,
        (HazardClass::Radioactive, _) | (_, HazardClass::Radioactive) => true,
        _ => left.requires_separation() && right.requires_separation(),
    }
}

pub fn conflict_pairs(lots: &[CargoLot]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for left in 0..lots.len() {
        for right in left + 1..lots.len() {
            if incompatible(&lots[left], &lots[right]) {
                pairs.push((left, right));
            }
        }
    }
    pairs
}

pub fn total_tonnage(lots: &[CargoLot]) -> u64 {
    lots.iter().map(|lot| lot.tonnes as u64).sum()
}

pub fn hazard_histogram(lots: &[CargoLot]) -> [u32; 10] {
    let mut out = [0u32; 10];
    for lot in lots {
        let idx = match lot.hazard {
            HazardClass::None => 0,
            HazardClass::Explosive => 1,
            HazardClass::Gas => 2,
            HazardClass::FlammableLiquid => 3,
            HazardClass::FlammableSolid => 4,
            HazardClass::Oxidizer => 5,
            HazardClass::Toxic => 6,
            HazardClass::Radioactive => 7,
            HazardClass::Corrosive => 8,
            HazardClass::Miscellaneous | HazardClass::Unknown(_) => 9,
        };
        out[idx] += 1;
    }
    out
}
