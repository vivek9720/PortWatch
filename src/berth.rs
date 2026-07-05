use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{BerthId, BerthPlan, Timestamp, VesselId};

pub fn parse_berths(data: &[u8], base: usize) -> ParseResult<Vec<BerthPlan>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 8192 {
        return Err(ParseError::limit(cursor.absolute_position(), "berth plan count"));
    }
    let mut plans = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        plans.push(parse_berth(&mut cursor)?);
    }
    Ok(plans)
}

pub fn parse_berth(cursor: &mut ByteCursor<'_>) -> ParseResult<BerthPlan> {
    let berth = BerthId(cursor.read_u16()?);
    let vessel = VesselId(cursor.read_u32()?);
    let start = Timestamp(cursor.read_u64()?);
    let duration = cursor.read_u32()? as u64;
    let end = start.saturating_add(duration);
    let priority = cursor.read_u8()?;
    let max_draught_cm = cursor.read_u16()?;
    let required_tugs = cursor.read_u8()?;
    let flags = cursor.read_u16()?;
    Ok(BerthPlan {
        berth,
        vessel,
        start,
        end,
        priority,
        max_draught_cm,
        required_tugs,
        flags,
    })
}

pub fn find_conflicts(plans: &[BerthPlan]) -> Vec<(usize, usize)> {
    let mut conflicts = Vec::new();
    for left in 0..plans.len() {
        for right in left + 1..plans.len() {
            if plans[left].overlaps(&plans[right]) {
                conflicts.push((left, right));
            }
        }
    }
    conflicts
}

pub fn compact_schedule(plans: &[BerthPlan]) -> Vec<BerthPlan> {
    let mut out = plans.to_vec();
    out.sort_by_key(|plan| (plan.berth.0, plan.start.0, plan.priority));
    out.dedup_by(|a, b| {
        a.berth == b.berth
            && a.vessel == b.vessel
            && a.start == b.start
            && a.end == b.end
            && a.flags == b.flags
    });
    out
}

#[derive(Debug, Clone, Default)]
pub struct BerthOccupancy {
    pub berth: BerthId,
    pub windows: Vec<(Timestamp, Timestamp, VesselId)>,
}

impl BerthOccupancy {
    pub fn from_plans(berth: BerthId, plans: &[BerthPlan]) -> Self {
        let mut windows = Vec::new();
        for plan in plans.iter().filter(|plan| plan.berth == berth) {
            windows.push((plan.start, plan.end, plan.vessel));
        }
        windows.sort_by_key(|window| window.0);
        Self { berth, windows }
    }

    pub fn occupied_at(&self, time: Timestamp) -> Option<VesselId> {
        self.windows
            .iter()
            .find(|(start, end, _)| time >= *start && time <= *end)
            .map(|(_, _, vessel)| *vessel)
    }

    pub fn utilization_seconds(&self) -> u64 {
        self.windows
            .iter()
            .map(|(start, end, _)| end.0.saturating_sub(start.0))
            .sum()
    }
}
