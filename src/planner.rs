use crate::model::{BerthId, BerthPlan, PilotOrder, Timestamp, VesselId, VesselSnapshot};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TugClass {
    Harbor,
    Escort,
    Firefighting,
    Salvage,
}

#[derive(Debug, Clone)]
pub struct TugAsset {
    pub code: u16,
    pub class: TugClass,
    pub bollard_pull_tonnes: u16,
    pub home_berth: BerthId,
    pub available_from: Timestamp,
}

#[derive(Debug, Clone)]
pub struct TugAssignment {
    pub vessel: VesselId,
    pub tug_code: u16,
    pub starts_at: Timestamp,
    pub ends_at: Timestamp,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct TugPlanner {
    assets: Vec<TugAsset>,
    assignments: Vec<TugAssignment>,
}

impl TugPlanner {
    pub fn new(assets: Vec<TugAsset>) -> Self {
        Self {
            assets,
            assignments: Vec::new(),
        }
    }

    pub fn assignments(&self) -> &[TugAssignment] {
        &self.assignments
    }

    pub fn plan_for(
        &mut self,
        vessel: &VesselSnapshot,
        berth: Option<&BerthPlan>,
        pilot: Option<&PilotOrder>,
    ) -> usize {
        let needed = required_tugs(vessel, berth);
        if needed == 0 {
            return 0;
        }
        let start = pilot
            .map(|order| order.boarding_time)
            .or_else(|| berth.map(|plan| plan.start.saturating_sub(1800)))
            .unwrap_or(vessel.timestamp);
        let end = berth
            .map(|plan| plan.end)
            .unwrap_or_else(|| start.saturating_add(7200));
        let mut assigned = 0usize;
        for asset in self.assets.clone() {
            if assigned >= needed {
                break;
            }
            if self.is_available(asset.code, start, end) && asset.available_from <= start {
                self.assignments.push(TugAssignment {
                    vessel: vessel.id,
                    tug_code: asset.code,
                    starts_at: start,
                    ends_at: end,
                    reason: assignment_reason(vessel, berth),
                });
                assigned += 1;
            }
        }
        assigned
    }

    pub fn is_available(&self, tug_code: u16, start: Timestamp, end: Timestamp) -> bool {
        self.assignments.iter().all(|assignment| {
            assignment.tug_code != tug_code
                || assignment.ends_at <= start
                || assignment.starts_at >= end
        })
    }

    pub fn utilization_by_tug(&self) -> BTreeMap<u16, u64> {
        let mut out = BTreeMap::new();
        for assignment in &self.assignments {
            *out.entry(assignment.tug_code).or_insert(0) +=
                assignment.ends_at.0.saturating_sub(assignment.starts_at.0);
        }
        out
    }
}

pub fn required_tugs(vessel: &VesselSnapshot, berth: Option<&BerthPlan>) -> usize {
    let mut needed = berth.map(|plan| plan.required_tugs as usize).unwrap_or(0);
    if vessel.draught_cm > 1350 {
        needed = needed.max(2);
    }
    if vessel.speed_tenths > 120 {
        needed = needed.max(1);
    }
    if vessel.risk_hint() > 70 {
        needed = needed.max(3);
    }
    needed.min(6)
}

pub fn assignment_reason(vessel: &VesselSnapshot, berth: Option<&BerthPlan>) -> &'static str {
    if vessel.risk_hint() > 70 {
        "risk"
    } else if berth.map(|plan| plan.required_tugs > 0).unwrap_or(false) {
        "berth"
    } else if vessel.draught_cm > 1350 {
        "draught"
    } else {
        "movement"
    }
}

pub fn default_assets() -> Vec<TugAsset> {
    vec![
        TugAsset { code: 101, class: TugClass::Harbor, bollard_pull_tonnes: 42, home_berth: BerthId(0x0601), available_from: Timestamp(0) },
        TugAsset { code: 102, class: TugClass::Harbor, bollard_pull_tonnes: 44, home_berth: BerthId(0x0601), available_from: Timestamp(0) },
        TugAsset { code: 201, class: TugClass::Escort, bollard_pull_tonnes: 78, home_berth: BerthId(0x0602), available_from: Timestamp(0) },
        TugAsset { code: 202, class: TugClass::Escort, bollard_pull_tonnes: 82, home_berth: BerthId(0x0602), available_from: Timestamp(0) },
        TugAsset { code: 301, class: TugClass::Firefighting, bollard_pull_tonnes: 64, home_berth: BerthId(0x0801), available_from: Timestamp(0) },
        TugAsset { code: 401, class: TugClass::Salvage, bollard_pull_tonnes: 120, home_berth: BerthId(0x0802), available_from: Timestamp(0) },
    ]
}
