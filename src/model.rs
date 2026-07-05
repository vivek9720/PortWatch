use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VesselId(pub u32);

impl VesselId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn is_temporary(self) -> bool {
        self.0 & 0x8000_0000 != 0
    }

    pub fn low_bits(self) -> u16 {
        (self.0 & 0xffff) as u16
    }
}

impl fmt::Display for VesselId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "V{:08x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BerthId(pub u16);

impl BerthId {
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    pub fn zone(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn slot(self) -> u8 {
        (self.0 & 0xff) as u8
    }
}

impl fmt::Display for BerthId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B{:02x}-{:02x}", self.zone(), self.slot())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn seconds(self) -> u64 {
        self.0
    }

    pub fn saturating_add(self, seconds: u64) -> Self {
        Self(self.0.saturating_add(seconds))
    }

    pub fn saturating_sub(self, seconds: u64) -> Self {
        Self(self.0.saturating_sub(seconds))
    }

    pub fn within(self, start: Timestamp, end: Timestamp) -> bool {
        self >= start && self <= end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Coordinates {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

impl Coordinates {
    pub fn new(lat_e7: i32, lon_e7: i32) -> Self {
        Self { lat_e7, lon_e7 }
    }

    pub fn latitude(self) -> f64 {
        self.lat_e7 as f64 / 10_000_000.0
    }

    pub fn longitude(self) -> f64 {
        self.lon_e7 as f64 / 10_000_000.0
    }

    pub fn is_plausible(self) -> bool {
        (-900_000_000..=900_000_000).contains(&self.lat_e7)
            && (-1_800_000_000..=1_800_000_000).contains(&self.lon_e7)
    }

    pub fn manhattan_to(self, other: Coordinates) -> u32 {
        self.lat_e7
            .abs_diff(other.lat_e7)
            .saturating_add(self.lon_e7.abs_diff(other.lon_e7))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VesselClass {
    Cargo,
    Tanker,
    Passenger,
    Tug,
    Pilot,
    Service,
    Fishing,
    Military,
    Unknown(u8),
}

impl VesselClass {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 => VesselClass::Cargo,
            1 => VesselClass::Tanker,
            2 => VesselClass::Passenger,
            3 => VesselClass::Tug,
            4 => VesselClass::Pilot,
            5 => VesselClass::Service,
            6 => VesselClass::Fishing,
            7 => VesselClass::Military,
            other => VesselClass::Unknown(other),
        }
    }

    pub fn risk_weight(self) -> u16 {
        match self {
            VesselClass::Cargo => 20,
            VesselClass::Tanker => 45,
            VesselClass::Passenger => 35,
            VesselClass::Tug => 12,
            VesselClass::Pilot => 6,
            VesselClass::Service => 8,
            VesselClass::Fishing => 14,
            VesselClass::Military => 50,
            VesselClass::Unknown(_) => 25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementStatus {
    Moored,
    Underway,
    Anchored,
    Restricted,
    Aground,
    Unknown(u8),
}

impl MovementStatus {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 => MovementStatus::Moored,
            1 => MovementStatus::Underway,
            2 => MovementStatus::Anchored,
            3 => MovementStatus::Restricted,
            4 => MovementStatus::Aground,
            other => MovementStatus::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardClass {
    None,
    Explosive,
    Gas,
    FlammableLiquid,
    FlammableSolid,
    Oxidizer,
    Toxic,
    Radioactive,
    Corrosive,
    Miscellaneous,
    Unknown(u8),
}

impl HazardClass {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 => HazardClass::None,
            1 => HazardClass::Explosive,
            2 => HazardClass::Gas,
            3 => HazardClass::FlammableLiquid,
            4 => HazardClass::FlammableSolid,
            5 => HazardClass::Oxidizer,
            6 => HazardClass::Toxic,
            7 => HazardClass::Radioactive,
            8 => HazardClass::Corrosive,
            9 => HazardClass::Miscellaneous,
            other => HazardClass::Unknown(other),
        }
    }

    pub fn risk_weight(self) -> u16 {
        match self {
            HazardClass::None => 0,
            HazardClass::Explosive => 80,
            HazardClass::Gas => 55,
            HazardClass::FlammableLiquid => 65,
            HazardClass::FlammableSolid => 50,
            HazardClass::Oxidizer => 60,
            HazardClass::Toxic => 70,
            HazardClass::Radioactive => 90,
            HazardClass::Corrosive => 45,
            HazardClass::Miscellaneous => 20,
            HazardClass::Unknown(_) => 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VesselSnapshot {
    pub id: VesselId,
    pub mmsi: u32,
    pub name_code: u16,
    pub callsign_code: u16,
    pub class: VesselClass,
    pub status: MovementStatus,
    pub position: Coordinates,
    pub speed_tenths: u16,
    pub heading_degrees: u16,
    pub timestamp: Timestamp,
    pub draught_cm: u16,
    pub destination_code: u16,
}

impl VesselSnapshot {
    pub fn risk_hint(&self) -> u16 {
        let mut risk = self.class.risk_weight();
        if self.speed_tenths > 180 {
            risk += 10;
        }
        if self.draught_cm > 1450 {
            risk += 8;
        }
        if !self.position.is_plausible() {
            risk += 60;
        }
        risk
    }

    pub fn is_stationary(&self) -> bool {
        self.speed_tenths <= 3
            || matches!(self.status, MovementStatus::Moored | MovementStatus::Anchored)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BerthPlan {
    pub berth: BerthId,
    pub vessel: VesselId,
    pub start: Timestamp,
    pub end: Timestamp,
    pub priority: u8,
    pub max_draught_cm: u16,
    pub required_tugs: u8,
    pub flags: u16,
}

impl BerthPlan {
    pub fn overlaps(&self, other: &BerthPlan) -> bool {
        self.berth == other.berth && self.start < other.end && other.start < self.end
    }

    pub fn contains(&self, time: Timestamp) -> bool {
        time.within(self.start, self.end)
    }

    pub fn duration_seconds(&self) -> u64 {
        self.end.0.saturating_sub(self.start.0)
    }

    pub fn is_reserved(&self) -> bool {
        self.flags & 0x0001 != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CargoLot {
    pub vessel: VesselId,
    pub un_number: u16,
    pub hazard: HazardClass,
    pub tonnes: u32,
    pub package_code: u16,
    pub stowage_zone: u8,
    pub temperature_tenths: i16,
    pub flags: u16,
}

impl CargoLot {
    pub fn hazard_score(&self) -> u32 {
        self.hazard.risk_weight() as u32
            + self.tonnes / 20
            + if self.flags & 0x04 != 0 { 25 } else { 0 }
    }

    pub fn requires_separation(&self) -> bool {
        matches!(
            self.hazard,
            HazardClass::Explosive
                | HazardClass::FlammableLiquid
                | HazardClass::Oxidizer
                | HazardClass::Toxic
                | HazardClass::Radioactive
        )
    }

    pub fn is_refrigerated(&self) -> bool {
        self.flags & 0x0008 != 0 || self.temperature_tenths < 50
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TideWindow {
    pub basin_code: u16,
    pub start: Timestamp,
    pub end: Timestamp,
    pub min_depth_cm: u16,
    pub max_current_cms: u16,
    pub direction_degrees: u16,
    pub confidence: u8,
}

impl TideWindow {
    pub fn supports_draught(&self, draught_cm: u16) -> bool {
        self.min_depth_cm >= draught_cm.saturating_add(120)
    }

    pub fn overlaps(&self, start: Timestamp, end: Timestamp) -> bool {
        self.start < end && start < self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PilotOrder {
    pub vessel: VesselId,
    pub pilot_code: u16,
    pub boarding_time: Timestamp,
    pub boarding_position: Coordinates,
    pub disembark_position: Coordinates,
    pub language_mask: u16,
    pub flags: u16,
}

impl PilotOrder {
    pub fn is_night_boarding(&self) -> bool {
        let seconds_in_day = self.boarding_time.0 % 86_400;
        seconds_in_day < 21_600 || seconds_in_day > 72_000
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelNotice {
    pub notice_id: u32,
    pub basin_code: u16,
    pub starts_at: Timestamp,
    pub expires_at: Timestamp,
    pub severity: u8,
    pub subject_code: u16,
    pub message_code: u16,
    pub flags: u16,
}

impl ChannelNotice {
    pub fn active_at(&self, time: Timestamp) -> bool {
        time.within(self.starts_at, self.expires_at)
    }

    pub fn is_closure(&self) -> bool {
        self.flags & 0x0001 != 0 || self.severity >= 7
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteLegKind {
    Approach,
    Turn,
    Anchorage,
    BerthEntry,
    Departure,
    Holding,
    Unknown(u8),
}

impl RouteLegKind {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 => RouteLegKind::Approach,
            1 => RouteLegKind::Turn,
            2 => RouteLegKind::Anchorage,
            3 => RouteLegKind::BerthEntry,
            4 => RouteLegKind::Departure,
            5 => RouteLegKind::Holding,
            other => RouteLegKind::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutePoint {
    pub position: Coordinates,
    pub eta: Timestamp,
    pub speed_limit_tenths: u16,
    pub depth_cm: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteLeg {
    pub vessel: VesselId,
    pub sequence: u16,
    pub kind: RouteLegKind,
    pub from: Coordinates,
    pub to: Coordinates,
    pub eta_start: Timestamp,
    pub eta_end: Timestamp,
    pub channel_code: u16,
    pub points: Vec<RoutePoint>,
}

impl RouteLeg {
    pub fn duration(&self) -> u64 {
        self.eta_end.0.saturating_sub(self.eta_start.0)
    }

    pub fn rough_distance(&self) -> u32 {
        let mut distance = self.from.manhattan_to(self.to);
        for pair in self.points.windows(2) {
            distance = distance.saturating_add(pair[0].position.manhattan_to(pair[1].position));
        }
        distance
    }

    pub fn has_low_water(&self, draught_cm: u16) -> bool {
        self.points
            .iter()
            .any(|point| point.depth_cm < draught_cm.saturating_add(100))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HarborEvent {
    Vessel(VesselSnapshot),
    Berth(BerthPlan),
    Cargo(CargoLot),
    Tide(TideWindow),
    Route(RouteLeg),
    Pilot(PilotOrder),
    Notice(ChannelNotice),
    Unknown { tag: u8, len: usize },
}

impl HarborEvent {
    pub fn vessel_id(&self) -> Option<VesselId> {
        match self {
            HarborEvent::Vessel(v) => Some(v.id),
            HarborEvent::Berth(b) => Some(b.vessel),
            HarborEvent::Cargo(c) => Some(c.vessel),
            HarborEvent::Route(r) => Some(r.vessel),
            HarborEvent::Pilot(p) => Some(p.vessel),
            HarborEvent::Tide(_) | HarborEvent::Notice(_) | HarborEvent::Unknown { .. } => None,
        }
    }

    pub fn tag_name(&self) -> &'static str {
        match self {
            HarborEvent::Vessel(_) => "vessel",
            HarborEvent::Berth(_) => "berth",
            HarborEvent::Cargo(_) => "cargo",
            HarborEvent::Tide(_) => "tide",
            HarborEvent::Route(_) => "route",
            HarborEvent::Pilot(_) => "pilot",
            HarborEvent::Notice(_) => "notice",
            HarborEvent::Unknown { .. } => "unknown",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PortSnapshot {
    pub vessels: Vec<VesselSnapshot>,
    pub berths: Vec<BerthPlan>,
    pub cargo: Vec<CargoLot>,
    pub tides: Vec<TideWindow>,
    pub routes: Vec<RouteLeg>,
    pub pilots: Vec<PilotOrder>,
    pub notices: Vec<ChannelNotice>,
}

impl PortSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: HarborEvent) {
        match event {
            HarborEvent::Vessel(v) => self.vessels.push(v),
            HarborEvent::Berth(b) => self.berths.push(b),
            HarborEvent::Cargo(c) => self.cargo.push(c),
            HarborEvent::Tide(t) => self.tides.push(t),
            HarborEvent::Route(r) => self.routes.push(r),
            HarborEvent::Pilot(p) => self.pilots.push(p),
            HarborEvent::Notice(n) => self.notices.push(n),
            HarborEvent::Unknown { .. } => {}
        }
    }

    pub fn event_count(&self) -> usize {
        self.vessels.len()
            + self.berths.len()
            + self.cargo.len()
            + self.tides.len()
            + self.routes.len()
            + self.pilots.len()
            + self.notices.len()
    }

    pub fn vessel(&self, id: VesselId) -> Option<&VesselSnapshot> {
        self.vessels.iter().find(|v| v.id == id)
    }

    pub fn cargo_for(&self, id: VesselId) -> impl Iterator<Item = &CargoLot> {
        self.cargo.iter().filter(move |lot| lot.vessel == id)
    }

    pub fn berth_for(&self, id: VesselId) -> Option<&BerthPlan> {
        self.berths.iter().find(|berth| berth.vessel == id)
    }

    pub fn routes_for(&self, id: VesselId) -> impl Iterator<Item = &RouteLeg> {
        self.routes.iter().filter(move |route| route.vessel == id)
    }

    pub fn sort_by_time(&mut self) {
        self.vessels.sort_by_key(|v| v.timestamp);
        self.berths.sort_by_key(|b| b.start);
        self.tides.sort_by_key(|t| t.start);
        self.routes.sort_by(|a, b| {
            a.eta_start
                .cmp(&b.eta_start)
                .then_with(|| a.sequence.cmp(&b.sequence))
        });
        self.pilots.sort_by_key(|p| p.boarding_time);
        self.notices.sort_by_key(|n| n.starts_at);
    }

    pub fn risk_score(&self) -> u32 {
        let vessel_score: u32 = self.vessels.iter().map(|v| v.risk_hint() as u32).sum();
        let cargo_score: u32 = self.cargo.iter().map(CargoLot::hazard_score).sum();
        let closure_score = self.notices.iter().filter(|n| n.is_closure()).count() as u32 * 20;
        vessel_score
            .saturating_add(cargo_score)
            .saturating_add(closure_score)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl FindingSeverity {
    pub fn weight(&self) -> u32 {
        match self {
            FindingSeverity::Info => 1,
            FindingSeverity::Low => 5,
            FindingSeverity::Medium => 15,
            FindingSeverity::High => 35,
            FindingSeverity::Critical => 80,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: FindingSeverity,
    pub code: &'static str,
    pub vessel: Option<VesselId>,
    pub detail: String,
}

impl Finding {
    pub fn new(
        severity: FindingSeverity,
        code: &'static str,
        vessel: Option<VesselId>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            vessel,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExchangeStats {
    pub frames: usize,
    pub sections: usize,
    pub compressed_sections: usize,
    pub dictionary_entries: usize,
    pub templates: usize,
    pub bytes_in: usize,
    pub bytes_decoded: usize,
}

impl ExchangeStats {
    pub fn merge(&mut self, other: &ExchangeStats) {
        self.frames += other.frames;
        self.sections += other.sections;
        self.compressed_sections += other.compressed_sections;
        self.dictionary_entries += other.dictionary_entries;
        self.templates += other.templates;
        self.bytes_in += other.bytes_in;
        self.bytes_decoded += other.bytes_decoded;
    }

    pub fn compression_ratio(&self) -> f32 {
        if self.bytes_in == 0 {
            1.0
        } else {
            self.bytes_decoded as f32 / self.bytes_in as f32
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl TimeRange {
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, time: Timestamp) -> bool {
        time.within(self.start, self.end)
    }

    pub fn overlaps(&self, other: &TimeRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn duration(&self) -> u64 {
        self.end.0.saturating_sub(self.start.0)
    }
}

pub fn compare_optional_time(a: Option<Timestamp>, b: Option<Timestamp>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
