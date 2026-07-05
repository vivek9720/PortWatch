pub mod berths;
pub mod hazmat;
pub mod ports;
pub mod signals;

#[derive(Debug, Clone, Copy)]
pub struct PortInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub country: &'static str,
    pub basin: u16,
    pub lat_e7: i32,
    pub lon_e7: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct BerthInfo {
    pub id: crate::model::BerthId,
    pub name: &'static str,
    pub basin: u16,
    pub max_draught_cm: u16,
    pub hazmat_allowed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct HazmatInfo {
    pub un_number: u16,
    pub name: &'static str,
    pub class_code: u8,
    pub segregation_group: u8,
    pub water_reactive: bool,
}
