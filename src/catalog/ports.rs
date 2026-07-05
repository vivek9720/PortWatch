use crate::catalog::PortInfo;

pub static PORTS: &[PortInfo] = &[
    PortInfo { code: "USLAX", name: "Los Angeles", country: "US", basin: 1, lat_e7: 337366000, lon_e7: -1182630000 },
    PortInfo { code: "USLGB", name: "Long Beach", country: "US", basin: 2, lat_e7: 337520000, lon_e7: -1181890000 },
    PortInfo { code: "USNYC", name: "New York", country: "US", basin: 3, lat_e7: 407120000, lon_e7: -740060000 },
    PortInfo { code: "USSAV", name: "Savannah", country: "US", basin: 4, lat_e7: 320800000, lon_e7: -811000000 },
    PortInfo { code: "USHOU", name: "Houston", country: "US", basin: 5, lat_e7: 297300000, lon_e7: -952600000 },
    PortInfo { code: "NLRTM", name: "Rotterdam", country: "NL", basin: 6, lat_e7: 519220000, lon_e7: 44790000 },
    PortInfo { code: "BEANR", name: "Antwerp", country: "BE", basin: 7, lat_e7: 512200000, lon_e7: 44000000 },
    PortInfo { code: "SGSIN", name: "Singapore", country: "SG", basin: 8, lat_e7: 12650000, lon_e7: 1038200000 },
    PortInfo { code: "CNSHA", name: "Shanghai", country: "CN", basin: 9, lat_e7: 312300000, lon_e7: 1215000000 },
    PortInfo { code: "KRPUS", name: "Busan", country: "KR", basin: 10, lat_e7: 351000000, lon_e7: 1290400000 },
];

pub fn lookup_port(code: &str) -> Option<&'static PortInfo> {
    PORTS.iter().find(|port| port.code == code)
}

pub fn by_country(country: &str) -> impl Iterator<Item = &'static PortInfo> + '_ {
    PORTS.iter().filter(move |port| port.country == country)
}

pub fn nearest(lat_e7: i32, lon_e7: i32) -> Option<&'static PortInfo> {
    PORTS.iter().min_by_key(|port| {
        port.lat_e7
            .abs_diff(lat_e7)
            .saturating_add(port.lon_e7.abs_diff(lon_e7))
    })
}
