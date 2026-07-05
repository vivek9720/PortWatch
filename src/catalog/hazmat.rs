use crate::catalog::HazmatInfo;

pub static HAZMAT: &[HazmatInfo] = &[
    HazmatInfo { un_number: 1001, name: "Acetylene dissolved", class_code: 2, segregation_group: 3, water_reactive: false },
    HazmatInfo { un_number: 1005, name: "Ammonia anhydrous", class_code: 2, segregation_group: 5, water_reactive: true },
    HazmatInfo { un_number: 1017, name: "Chlorine", class_code: 2, segregation_group: 6, water_reactive: true },
    HazmatInfo { un_number: 1075, name: "Petroleum gases liquefied", class_code: 2, segregation_group: 4, water_reactive: false },
    HazmatInfo { un_number: 1090, name: "Acetone", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1202, name: "Diesel fuel", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1203, name: "Gasoline", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1230, name: "Methanol", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1263, name: "Paint", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1294, name: "Toluene", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1381, name: "Phosphorus white", class_code: 4, segregation_group: 7, water_reactive: true },
    HazmatInfo { un_number: 1402, name: "Calcium carbide", class_code: 4, segregation_group: 7, water_reactive: true },
    HazmatInfo { un_number: 1495, name: "Sodium chlorate", class_code: 5, segregation_group: 8, water_reactive: false },
    HazmatInfo { un_number: 1498, name: "Sodium nitrate", class_code: 5, segregation_group: 8, water_reactive: false },
    HazmatInfo { un_number: 1547, name: "Aniline", class_code: 6, segregation_group: 6, water_reactive: false },
    HazmatInfo { un_number: 1680, name: "Potassium cyanide", class_code: 6, segregation_group: 6, water_reactive: true },
    HazmatInfo { un_number: 1789, name: "Hydrochloric acid", class_code: 8, segregation_group: 1, water_reactive: false },
    HazmatInfo { un_number: 1791, name: "Hypochlorite solution", class_code: 8, segregation_group: 8, water_reactive: false },
    HazmatInfo { un_number: 1824, name: "Sodium hydroxide solution", class_code: 8, segregation_group: 1, water_reactive: false },
    HazmatInfo { un_number: 1830, name: "Sulfuric acid", class_code: 8, segregation_group: 1, water_reactive: true },
    HazmatInfo { un_number: 1863, name: "Fuel aviation turbine engine", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 1942, name: "Ammonium nitrate", class_code: 5, segregation_group: 8, water_reactive: false },
    HazmatInfo { un_number: 1993, name: "Flammable liquid n.o.s.", class_code: 3, segregation_group: 2, water_reactive: false },
    HazmatInfo { un_number: 2014, name: "Hydrogen peroxide aqueous", class_code: 5, segregation_group: 8, water_reactive: false },
    HazmatInfo { un_number: 2031, name: "Nitric acid", class_code: 8, segregation_group: 8, water_reactive: true },
    HazmatInfo { un_number: 2209, name: "Formaldehyde solution", class_code: 8, segregation_group: 6, water_reactive: false },
    HazmatInfo { un_number: 2211, name: "Polymeric beads expandable", class_code: 9, segregation_group: 9, water_reactive: false },
    HazmatInfo { un_number: 2315, name: "Polychlorinated biphenyls", class_code: 9, segregation_group: 6, water_reactive: false },
    HazmatInfo { un_number: 3077, name: "Environmentally hazardous substance solid", class_code: 9, segregation_group: 9, water_reactive: false },
    HazmatInfo { un_number: 3082, name: "Environmentally hazardous substance liquid", class_code: 9, segregation_group: 9, water_reactive: false },
];

pub fn lookup_un_number(un_number: u16) -> Option<&'static HazmatInfo> {
    HAZMAT.iter().find(|entry| entry.un_number == un_number)
}

pub fn by_segregation_group(group: u8) -> impl Iterator<Item = &'static HazmatInfo> {
    HAZMAT
        .iter()
        .filter(move |entry| entry.segregation_group == group)
}

pub fn is_water_reactive(un_number: u16) -> bool {
    lookup_un_number(un_number)
        .map(|entry| entry.water_reactive)
        .unwrap_or(false)
}
