use crate::catalog;
use crate::model::{Finding, FindingSeverity, PortSnapshot};

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub findings: Vec<Finding>,
    pub checked_vessels: usize,
    pub checked_berths: usize,
    pub checked_cargo: usize,
    pub checked_routes: usize,
}

impl ValidationReport {
    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    pub fn score(&self) -> u32 {
        self.findings
            .iter()
            .map(|finding| finding.severity.weight())
            .sum()
    }

    pub fn has_critical(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == FindingSeverity::Critical)
    }
}

pub fn validate_snapshot(snapshot: &PortSnapshot) -> ValidationReport {
    let mut report = ValidationReport::default();
    validate_vessels(snapshot, &mut report);
    validate_berths(snapshot, &mut report);
    validate_cargo(snapshot, &mut report);
    validate_routes(snapshot, &mut report);
    report
}

fn validate_vessels(snapshot: &PortSnapshot, report: &mut ValidationReport) {
    for vessel in &snapshot.vessels {
        report.checked_vessels += 1;
        if vessel.mmsi < 100_000_000 || vessel.mmsi > 999_999_999 {
            report.push(Finding::new(
                FindingSeverity::Low,
                "mmsi-range",
                Some(vessel.id),
                format!("{} has nonstandard MMSI {}", vessel.id, vessel.mmsi),
            ));
        }
        if !vessel.position.is_plausible() {
            report.push(Finding::new(
                FindingSeverity::High,
                "position-range",
                Some(vessel.id),
                format!("{} has implausible coordinates", vessel.id),
            ));
        }
        if vessel.heading_degrees >= 360 && vessel.heading_degrees != 511 {
            report.push(Finding::new(
                FindingSeverity::Info,
                "heading-range",
                Some(vessel.id),
                format!("{} has heading {}", vessel.id, vessel.heading_degrees),
            ));
        }
        if vessel.speed_tenths > 450 {
            report.push(Finding::new(
                FindingSeverity::Medium,
                "speed-range",
                Some(vessel.id),
                format!("{} speed {} tenths", vessel.id, vessel.speed_tenths),
            ));
        }
    }
}

fn validate_berths(snapshot: &PortSnapshot, report: &mut ValidationReport) {
    for berth in &snapshot.berths {
        report.checked_berths += 1;
        if berth.end <= berth.start {
            report.push(Finding::new(
                FindingSeverity::Medium,
                "berth-duration",
                Some(berth.vessel),
                format!("{} has empty berth window", berth.berth),
            ));
        }
        if let Some(info) = catalog::berths::lookup_berth(berth.berth) {
            if berth.max_draught_cm > info.max_draught_cm.saturating_add(200) {
                report.push(Finding::new(
                    FindingSeverity::Low,
                    "berth-draught-override",
                    Some(berth.vessel),
                    format!("{} exceeds catalog draught", berth.berth),
                ));
            }
        }
    }
}

fn validate_cargo(snapshot: &PortSnapshot, report: &mut ValidationReport) {
    for lot in &snapshot.cargo {
        report.checked_cargo += 1;
        if lot.tonnes > 500_000 {
            report.push(Finding::new(
                FindingSeverity::High,
                "cargo-tonnage",
                Some(lot.vessel),
                format!("UN{} has large tonnage {}", lot.un_number, lot.tonnes),
            ));
        }
        if catalog::hazmat::lookup_un_number(lot.un_number).is_none()
            && lot.hazard.risk_weight() >= 50
        {
            report.push(Finding::new(
                FindingSeverity::Medium,
                "hazmat-catalog-miss",
                Some(lot.vessel),
                format!("UN{} missing from catalog", lot.un_number),
            ));
        }
    }
}

fn validate_routes(snapshot: &PortSnapshot, report: &mut ValidationReport) {
    for route in &snapshot.routes {
        report.checked_routes += 1;
        if route.eta_end <= route.eta_start {
            report.push(Finding::new(
                FindingSeverity::Low,
                "route-duration",
                Some(route.vessel),
                format!("route {} has empty ETA span", route.sequence),
            ));
        }
        for point in &route.points {
            if !point.position.is_plausible() {
                report.push(Finding::new(
                    FindingSeverity::Medium,
                    "route-position",
                    Some(route.vessel),
                    format!("route {} has implausible point", route.sequence),
                ));
                break;
            }
        }
    }
}

pub fn find_duplicate_vessels(snapshot: &PortSnapshot) -> Vec<crate::model::VesselId> {
    let mut seen = std::collections::BTreeSet::new();
    let mut duplicates = Vec::new();
    for vessel in &snapshot.vessels {
        if !seen.insert(vessel.id.0) {
            duplicates.push(vessel.id);
        }
    }
    duplicates
}
