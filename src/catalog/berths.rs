use crate::catalog::BerthInfo;
use crate::model::BerthId;

pub static BERTHS: &[BerthInfo] = &[
    BerthInfo { id: BerthId(0x0101), name: "North Container 1", basin: 1, max_draught_cm: 1480, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0102), name: "North Container 2", basin: 1, max_draught_cm: 1460, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0103), name: "North Container 3", basin: 1, max_draught_cm: 1420, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0201), name: "East Liquid A", basin: 2, max_draught_cm: 1640, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0202), name: "East Liquid B", basin: 2, max_draught_cm: 1600, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0301), name: "RoRo Ramp 1", basin: 3, max_draught_cm: 930, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0302), name: "RoRo Ramp 2", basin: 3, max_draught_cm: 970, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0401), name: "Breakbulk West", basin: 4, max_draught_cm: 1280, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0402), name: "Breakbulk East", basin: 4, max_draught_cm: 1260, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0501), name: "Passenger South", basin: 5, max_draught_cm: 910, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0502), name: "Passenger North", basin: 5, max_draught_cm: 920, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0601), name: "Service Pier A", basin: 6, max_draught_cm: 720, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0602), name: "Service Pier B", basin: 6, max_draught_cm: 700, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0701), name: "Anchorage Alpha", basin: 7, max_draught_cm: 2100, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0702), name: "Anchorage Bravo", basin: 7, max_draught_cm: 2050, hazmat_allowed: true },
    BerthInfo { id: BerthId(0x0801), name: "Repair Quay 1", basin: 8, max_draught_cm: 1150, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0802), name: "Repair Quay 2", basin: 8, max_draught_cm: 1120, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0901), name: "Grain Terminal", basin: 9, max_draught_cm: 1340, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0a01), name: "Cold Storage A", basin: 10, max_draught_cm: 1080, hazmat_allowed: false },
    BerthInfo { id: BerthId(0x0a02), name: "Cold Storage B", basin: 10, max_draught_cm: 1060, hazmat_allowed: false },
];

pub fn lookup_berth(id: BerthId) -> Option<&'static BerthInfo> {
    BERTHS.iter().find(|berth| berth.id == id)
}

pub fn basin_berths(basin: u16) -> impl Iterator<Item = &'static BerthInfo> {
    BERTHS.iter().filter(move |berth| berth.basin == basin)
}

pub fn hazmat_berths() -> impl Iterator<Item = &'static BerthInfo> {
    BERTHS.iter().filter(|berth| berth.hazmat_allowed)
}

pub fn max_draught_for_basin(basin: u16) -> u16 {
    basin_berths(basin)
        .map(|berth| berth.max_draught_cm)
        .max()
        .unwrap_or_default()
}
