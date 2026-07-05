use crate::model::{Coordinates, RouteLeg, RoutePoint, VesselId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone)]
pub struct RouteNode {
    pub id: NodeId,
    pub position: Coordinates,
    pub min_depth_cm: u16,
    pub channel_code: u16,
}

#[derive(Debug, Clone)]
pub struct RouteEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub vessel: VesselId,
    pub sequence: u16,
    pub eta_start: u64,
    pub eta_end: u64,
    pub distance_hint: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RouteGraph {
    pub nodes: BTreeMap<NodeId, RouteNode>,
    pub edges: Vec<RouteEdge>,
}

impl RouteGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_routes(routes: &[RouteLeg]) -> Self {
        let mut graph = RouteGraph::new();
        for route in routes {
            graph.add_route(route);
        }
        graph
    }

    pub fn add_route(&mut self, route: &RouteLeg) {
        let mut previous = self.node_for_point(route.channel_code, &RoutePoint {
            position: route.from,
            eta: route.eta_start,
            speed_limit_tenths: 0,
            depth_cm: u16::MAX,
        });
        for point in &route.points {
            let next = self.node_for_point(route.channel_code, point);
            self.edges.push(RouteEdge {
                from: previous,
                to: next,
                vessel: route.vessel,
                sequence: route.sequence,
                eta_start: route.eta_start.0,
                eta_end: point.eta.0,
                distance_hint: self.distance(previous, next),
            });
            previous = next;
        }
        let terminal = self.node_for_point(route.channel_code, &RoutePoint {
            position: route.to,
            eta: route.eta_end,
            speed_limit_tenths: 0,
            depth_cm: u16::MAX,
        });
        self.edges.push(RouteEdge {
            from: previous,
            to: terminal,
            vessel: route.vessel,
            sequence: route.sequence,
            eta_start: route.eta_start.0,
            eta_end: route.eta_end.0,
            distance_hint: self.distance(previous, terminal),
        });
    }

    fn node_for_point(&mut self, channel_code: u16, point: &RoutePoint) -> NodeId {
        let id = quantized_node(point.position, channel_code);
        self.nodes.entry(id).or_insert(RouteNode {
            id,
            position: point.position,
            min_depth_cm: point.depth_cm,
            channel_code,
        });
        if let Some(node) = self.nodes.get_mut(&id) {
            node.min_depth_cm = node.min_depth_cm.min(point.depth_cm);
        }
        id
    }

    pub fn distance(&self, from: NodeId, to: NodeId) -> u32 {
        match (self.nodes.get(&from), self.nodes.get(&to)) {
            (Some(a), Some(b)) => a.position.manhattan_to(b.position),
            _ => 0,
        }
    }

    pub fn vessels_through(&self, node: NodeId) -> BTreeSet<VesselId> {
        let mut vessels = BTreeSet::new();
        for edge in &self.edges {
            if edge.from == node || edge.to == node {
                vessels.insert(edge.vessel);
            }
        }
        vessels
    }

    pub fn choke_points(&self, min_vessels: usize) -> Vec<NodeId> {
        self.nodes
            .keys()
            .copied()
            .filter(|node| self.vessels_through(*node).len() >= min_vessels)
            .collect()
    }

    pub fn crossing_edges(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for left in 0..self.edges.len() {
            for right in left + 1..self.edges.len() {
                let a = &self.edges[left];
                let b = &self.edges[right];
                if a.vessel == b.vessel {
                    continue;
                }
                if a.from == b.to && a.to == b.from && times_overlap(a, b) {
                    out.push((left, right));
                }
            }
        }
        out
    }

    pub fn low_depth_nodes(&self, draught_cm: u16) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter(|node| node.min_depth_cm < draught_cm.saturating_add(100))
            .map(|node| node.id)
            .collect()
    }
}

fn times_overlap(left: &RouteEdge, right: &RouteEdge) -> bool {
    left.eta_start < right.eta_end && right.eta_start < left.eta_end
}

pub fn quantized_node(position: Coordinates, channel_code: u16) -> NodeId {
    let lat = ((position.lat_e7 / 10_000) as i64) & 0x0fff;
    let lon = ((position.lon_e7 / 10_000) as i64) & 0x0fff;
    NodeId(((channel_code as u32) << 24) | ((lat as u32) << 12) | lon as u32)
}
