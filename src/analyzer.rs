use crate::berth;
use crate::cargo;
use crate::catalog;
use crate::model::{
    Finding, FindingSeverity, HarborEvent, PortSnapshot, RouteLegKind, VesselId,
};
use crate::route;
use crate::session::StreamSummary;
use crate::tide;
use crate::{error::ParseResult, model::ExchangeStats};

#[derive(Debug, Clone)]
pub struct HarborReport {
    pub stream_id: u16,
    pub stats: ExchangeStats,
    pub event_count: usize,
    pub risk_score: u32,
    pub findings: Vec<Finding>,
    pub materialized_names: usize,
}

pub fn analyze_exchange(summary: StreamSummary) -> ParseResult<HarborReport> {
    let mut findings = Vec::new();
    analyze_berth_conflicts(&summary.snapshot, &mut findings);
    analyze_cargo_conflicts(&summary.snapshot, &mut findings);
    analyze_tide_windows(&summary.snapshot, &mut findings);
    analyze_routes(&summary.snapshot, &mut findings);
    analyze_pilotage(&summary.snapshot, &mut findings);
    analyze_catalog_consistency(&summary.snapshot, &mut findings);
    let risk_score = summary
        .snapshot
        .risk_score()
        .saturating_add(findings.iter().map(|f| f.severity.weight()).sum::<u32>());
    Ok(HarborReport {
        stream_id: summary.stream_id,
        stats: summary.stats,
        event_count: summary.snapshot.event_count(),
        risk_score,
        findings,
        materialized_names: summary.ledger_materialized.len(),
    })
}

fn analyze_berth_conflicts(snapshot: &PortSnapshot, findings: &mut Vec<Finding>) {
    let compact = berth::compact_schedule(&snapshot.berths);
    for (left, right) in berth::find_conflicts(&compact) {
        let a = &compact[left];
        let b = &compact[right];
        findings.push(Finding::new(
            FindingSeverity::High,
            "berth-overlap",
            Some(a.vessel),
            format!("{} overlaps {} at {}", a.vessel, b.vessel, a.berth),
        ));
    }
}

fn analyze_cargo_conflicts(snapshot: &PortSnapshot, findings: &mut Vec<Finding>) {
    for (left, right) in cargo::conflict_pairs(&snapshot.cargo) {
        let a = &snapshot.cargo[left];
        let b = &snapshot.cargo[right];
        findings.push(Finding::new(
            FindingSeverity::Medium,
            "cargo-separation",
            Some(a.vessel),
            format!("UN{} conflicts with UN{}", a.un_number, b.un_number),
        ));
    }
}

fn analyze_tide_windows(snapshot: &PortSnapshot, findings: &mut Vec<Finding>) {
    for vessel in &snapshot.vessels {
        for berth in snapshot.berths.iter().filter(|plan| plan.vessel == vessel.id) {
            let basin = berth.berth.zone() as u16;
            if tide::best_window(
                &snapshot.tides,
                basin,
                berth.start,
                berth.end,
                vessel.draught_cm,
            )
            .is_none()
            {
                findings.push(Finding::new(
                    FindingSeverity::Medium,
                    "missing-tide-window",
                    Some(vessel.id),
                    format!("{} lacks tide support for berth {}", vessel.id, berth.berth),
                ));
            }
        }
    }
}

fn analyze_routes(snapshot: &PortSnapshot, findings: &mut Vec<Finding>) {
    let density = route::route_density(&snapshot.routes);
    if density > 20_000 {
        findings.push(Finding::new(
            FindingSeverity::Low,
            "route-density",
            None,
            format!("route density {}", density),
        ));
    }
    for vessel in &snapshot.vessels {
        for leg in snapshot.routes_for(vessel.id) {
            if leg.has_low_water(vessel.draught_cm) {
                findings.push(Finding::new(
                    FindingSeverity::High,
                    "route-low-water",
                    Some(vessel.id),
                    format!("route {} has low-water point", leg.sequence),
                ));
            }
            if matches!(leg.kind, RouteLegKind::BerthEntry) && snapshot.berth_for(vessel.id).is_none()
            {
                findings.push(Finding::new(
                    FindingSeverity::Low,
                    "entry-without-berth",
                    Some(vessel.id),
                    "route enters berth without berth plan",
                ));
            }
        }
    }
}

fn analyze_pilotage(snapshot: &PortSnapshot, findings: &mut Vec<Finding>) {
    for vessel in &snapshot.vessels {
        let high_risk = vessel.risk_hint() > 40;
        let has_order = snapshot.pilots.iter().any(|pilot| pilot.vessel == vessel.id);
        if high_risk && !has_order {
            findings.push(Finding::new(
                FindingSeverity::Medium,
                "pilotage-missing",
                Some(vessel.id),
                "high-risk vessel has no pilot order",
            ));
        }
    }
}

fn analyze_catalog_consistency(snapshot: &PortSnapshot, findings: &mut Vec<Finding>) {
    for lot in &snapshot.cargo {
        if catalog::hazmat::lookup_un_number(lot.un_number).is_none()
            && lot.hazard.risk_weight() > 0
        {
            findings.push(Finding::new(
                FindingSeverity::Info,
                "unknown-un-number",
                Some(lot.vessel),
                format!("UN{} not in local catalog", lot.un_number),
            ));
        }
    }
    for berth in &snapshot.berths {
        if catalog::berths::lookup_berth(berth.berth).is_none() {
            findings.push(Finding::new(
                FindingSeverity::Info,
                "unknown-berth",
                Some(berth.vessel),
                format!("{} not in local berth catalog", berth.berth),
            ));
        }
    }
}

pub fn summarize_events(events: &[HarborEvent]) -> (usize, usize) {
    let mut vessel_events = 0;
    let mut port_events = 0;
    for event in events {
        if event.vessel_id().is_some() {
            vessel_events += 1;
        } else {
            port_events += 1;
        }
    }
    (vessel_events, port_events)
}

pub fn vessel_risk_bucket(snapshot: &PortSnapshot, vessel: VesselId) -> u8 {
    let vessel_score = snapshot
        .vessel(vessel)
        .map(|v| v.risk_hint() as u32)
        .unwrap_or_default();
    let cargo_score: u32 = snapshot.cargo_for(vessel).map(|lot| lot.hazard_score()).sum();
    match vessel_score.saturating_add(cargo_score) {
        0..=49 => 0,
        50..=149 => 1,
        150..=299 => 2,
        _ => 3,
    }
}
