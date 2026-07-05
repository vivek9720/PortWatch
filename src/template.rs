use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{CargoLot, Coordinates, HazardClass, HarborEvent, Timestamp, VesselId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    U8,
    U16,
    U32,
    U64,
    I16,
    I32,
    Var,
    StringCode,
    Coordinates,
}

impl FieldKind {
    pub fn from_byte(byte: u8) -> ParseResult<Self> {
        match byte {
            0 => Ok(FieldKind::U8),
            1 => Ok(FieldKind::U16),
            2 => Ok(FieldKind::U32),
            3 => Ok(FieldKind::U64),
            4 => Ok(FieldKind::I16),
            5 => Ok(FieldKind::I32),
            6 => Ok(FieldKind::Var),
            7 => Ok(FieldKind::StringCode),
            8 => Ok(FieldKind::Coordinates),
            _ => Err(ParseError::invalid_value(0, "template field kind")),
        }
    }

    pub fn fixed_width(self) -> Option<usize> {
        match self {
            FieldKind::U8 => Some(1),
            FieldKind::U16 | FieldKind::I16 | FieldKind::StringCode => Some(2),
            FieldKind::U32 | FieldKind::I32 => Some(4),
            FieldKind::U64 => Some(8),
            FieldKind::Coordinates => Some(8),
            FieldKind::Var => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemplateField {
    pub id: u8,
    pub kind: FieldKind,
    pub scale: i16,
    pub flags: u16,
}

#[derive(Debug, Clone)]
pub struct RecordTemplate {
    pub id: u16,
    pub event_kind: u8,
    pub fields: Vec<TemplateField>,
}

impl RecordTemplate {
    pub fn parse(cursor: &mut ByteCursor<'_>) -> ParseResult<Self> {
        let id = cursor.read_u16()?;
        let event_kind = cursor.read_u8()?;
        let count = cursor.read_u8()? as usize;
        if count > 64 {
            return Err(ParseError::limit(cursor.absolute_position(), "template field count"));
        }
        let mut fields = Vec::with_capacity(count);
        for _ in 0..count {
            let id = cursor.read_u8()?;
            let kind = FieldKind::from_byte(cursor.read_u8()?)?;
            let scale = cursor.read_i16()?;
            let flags = cursor.read_u16()?;
            fields.push(TemplateField {
                id,
                kind,
                scale,
                flags,
            });
        }
        Ok(Self {
            id,
            event_kind,
            fields,
        })
    }

    pub fn minimum_width(&self) -> usize {
        self.fields
            .iter()
            .map(|field| field.kind.fixed_width().unwrap_or(1))
            .sum()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    templates: BTreeMap<u16, RecordTemplate>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, template: RecordTemplate) {
        self.templates.insert(template.id, template);
    }

    pub fn get(&self, id: u16) -> Option<&RecordTemplate> {
        self.templates.get(&id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn parse_section(&mut self, data: &[u8], base: usize) -> ParseResult<()> {
        let mut cursor = ByteCursor::with_base(data, base);
        let count = cursor.read_u16()? as usize;
        if count > 1024 {
            return Err(ParseError::limit(cursor.absolute_position(), "template count"));
        }
        for _ in 0..count {
            let template = RecordTemplate::parse(&mut cursor)?;
            self.insert(template);
        }
        Ok(())
    }

    pub fn parse_records(&self, data: &[u8], base: usize) -> ParseResult<Vec<HarborEvent>> {
        let mut cursor = ByteCursor::with_base(data, base);
        let count = cursor.read_u16()? as usize;
        if count > 16_384 {
            return Err(ParseError::limit(cursor.absolute_position(), "template record count"));
        }
        let mut events = Vec::with_capacity(count.min(128));
        for _ in 0..count {
            let template_id = cursor.read_u16()?;
            let template = self
                .get(template_id)
                .ok_or_else(|| ParseError::template(cursor.absolute_position(), "template id"))?;
            if let Some(event) = parse_event_from_template(template, &mut cursor)? {
                events.push(event);
            }
        }
        Ok(events)
    }
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I16(i16),
    I32(i32),
    Var(u64),
    Coordinates(Coordinates),
}

pub fn parse_values(
    template: &RecordTemplate,
    cursor: &mut ByteCursor<'_>,
) -> ParseResult<BTreeMap<u8, FieldValue>> {
    let mut values = BTreeMap::new();
    for field in &template.fields {
        let value = match field.kind {
            FieldKind::U8 => FieldValue::U8(cursor.read_u8()?),
            FieldKind::U16 | FieldKind::StringCode => FieldValue::U16(cursor.read_u16()?),
            FieldKind::U32 => FieldValue::U32(cursor.read_u32()?),
            FieldKind::U64 => FieldValue::U64(cursor.read_u64()?),
            FieldKind::I16 => FieldValue::I16(cursor.read_i16()?),
            FieldKind::I32 => FieldValue::I32(cursor.read_i32()?),
            FieldKind::Var => FieldValue::Var(cursor.read_varint("template var")?),
            FieldKind::Coordinates => {
                FieldValue::Coordinates(Coordinates::new(cursor.read_i32()?, cursor.read_i32()?))
            }
        };
        values.insert(field.id, value);
    }
    Ok(values)
}

fn u16_field(values: &BTreeMap<u8, FieldValue>, id: u8) -> u16 {
    match values.get(&id) {
        Some(FieldValue::U16(value)) => *value,
        Some(FieldValue::U8(value)) => *value as u16,
        Some(FieldValue::U32(value)) => *value as u16,
        _ => 0,
    }
}

fn u32_field(values: &BTreeMap<u8, FieldValue>, id: u8) -> u32 {
    match values.get(&id) {
        Some(FieldValue::U32(value)) => *value,
        Some(FieldValue::U16(value)) => *value as u32,
        Some(FieldValue::U8(value)) => *value as u32,
        Some(FieldValue::Var(value)) => *value as u32,
        _ => 0,
    }
}

fn i16_field(values: &BTreeMap<u8, FieldValue>, id: u8) -> i16 {
    match values.get(&id) {
        Some(FieldValue::I16(value)) => *value,
        Some(FieldValue::U16(value)) => *value as i16,
        _ => 0,
    }
}

pub fn parse_event_from_template(
    template: &RecordTemplate,
    cursor: &mut ByteCursor<'_>,
) -> ParseResult<Option<HarborEvent>> {
    let values = parse_values(template, cursor)?;
    let event = match template.event_kind {
        3 => HarborEvent::Cargo(CargoLot {
            vessel: VesselId(u32_field(&values, 1)),
            un_number: u16_field(&values, 2),
            hazard: HazardClass::from_byte(u16_field(&values, 3) as u8),
            tonnes: u32_field(&values, 4),
            package_code: u16_field(&values, 5),
            stowage_zone: u16_field(&values, 6) as u8,
            temperature_tenths: i16_field(&values, 7),
            flags: u16_field(&values, 8),
        }),
        _ => return Ok(None),
    };
    Ok(Some(event))
}
