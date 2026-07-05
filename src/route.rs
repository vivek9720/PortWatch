use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{Coordinates, RouteLeg, RouteLegKind, RoutePoint, Timestamp, VesselId};

pub fn parse_routes(data: &[u8], base: usize) -> ParseResult<Vec<RouteLeg>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 4096 {
        return Err(ParseError::limit(cursor.absolute_position(), "route count"));
    }
    let mut routes = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        routes.push(parse_route(&mut cursor)?);
    }
    Ok(routes)
}

pub fn parse_route(cursor: &mut ByteCursor<'_>) -> ParseResult<RouteLeg> {
    let vessel = VesselId(cursor.read_u32()?);
    let sequence = cursor.read_u16()?;
    let kind = RouteLegKind::from_byte(cursor.read_u8()?);
    let channel_code = cursor.read_u16()?;
    let from = Coordinates::new(cursor.read_i32()?, cursor.read_i32()?);
    let to = Coordinates::new(cursor.read_i32()?, cursor.read_i32()?);
    let eta_start = Timestamp(cursor.read_u64()?);
    let duration = cursor.read_u32()? as u64;
    let point_count = cursor.read_u16()? as usize;
    if point_count > 2048 {
        return Err(ParseError::limit(cursor.absolute_position(), "route point count"));
    }
    let mut points = Vec::with_capacity(point_count.min(64));
    let mut lat = from.lat_e7;
    let mut lon = from.lon_e7;
    let mut eta = eta_start.0;
    for _ in 0..point_count {
        lat = lat.wrapping_add(cursor.read_zigzag_i64("route lat delta")? as i32);
        lon = lon.wrapping_add(cursor.read_zigzag_i64("route lon delta")? as i32);
        eta = eta.saturating_add(cursor.read_varint("route eta delta")?);
        let speed_limit_tenths = cursor.read_u16()?;
        let depth_cm = cursor.read_u16()?;
        points.push(RoutePoint {
            position: Coordinates::new(lat, lon),
            eta: Timestamp(eta),
            speed_limit_tenths,
            depth_cm,
        });
    }
    Ok(RouteLeg {
        vessel,
        sequence,
        kind,
        from,
        to,
        eta_start,
        eta_end: eta_start.saturating_add(duration),
        channel_code,
        points,
    })
}

pub fn parse_route_bundle(data: &[u8], base: usize) -> ParseResult<RouteBundle> {
    let routes = parse_routes(data, base)?;
    let mut bundle = RouteBundle { routes };
    bundle.routes.sort_by_key(|route| (route.vessel.0, route.sequence));
    Ok(bundle)
}

#[derive(Debug, Clone, Default)]
pub struct RouteBundle {
    pub routes: Vec<RouteLeg>,
}

impl RouteBundle {
    pub fn by_vessel(&self, vessel: VesselId) -> Vec<&RouteLeg> {
        self.routes
            .iter()
            .filter(|route| route.vessel == vessel)
            .collect()
    }

    pub fn total_points(&self) -> usize {
        self.routes.iter().map(|route| route.points.len()).sum()
    }

    pub fn low_water_legs(&self, vessel: VesselId, draught_cm: u16) -> Vec<&RouteLeg> {
        self.routes
            .iter()
            .filter(|route| route.vessel == vessel)
            .filter(|route| route.has_low_water(draught_cm))
            .collect()
    }

    pub fn sequence_gaps(&self, vessel: VesselId) -> Vec<u16> {
        let mut legs: Vec<&RouteLeg> = self
            .routes
            .iter()
            .filter(|route| route.vessel == vessel)
            .collect();
        legs.sort_by_key(|route| route.sequence);
        let mut gaps = Vec::new();
        for pair in legs.windows(2) {
            let expected = pair[0].sequence.saturating_add(1);
            if pair[1].sequence != expected {
                gaps.push(expected);
            }
        }
        gaps
    }
}

pub fn interpolate_leg(route: &RouteLeg, samples: usize) -> Vec<RoutePoint> {
    if samples == 0 {
        return Vec::new();
    }
    if route.points.is_empty() {
        return vec![
            RoutePoint {
                position: route.from,
                eta: route.eta_start,
                speed_limit_tenths: 0,
                depth_cm: 0,
            },
            RoutePoint {
                position: route.to,
                eta: route.eta_end,
                speed_limit_tenths: 0,
                depth_cm: 0,
            },
        ];
    }
    let mut out = Vec::with_capacity(samples);
    for index in 0..samples {
        let scaled = index * route.points.len() / samples;
        let point = route.points[scaled.min(route.points.len() - 1)].clone();
        out.push(point);
    }
    out
}

pub fn route_density(routes: &[RouteLeg]) -> u32 {
    let mut density = 0u32;
    for route in routes {
        density = density.saturating_add(route.points.len() as u32);
        if route.duration() < 900 {
            density = density.saturating_add(5);
        }
        if route.rough_distance() > 5_000_000 {
            density = density.saturating_add(8);
        }
    }
    density
}
