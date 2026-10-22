//! Experimental neutral archetype topology, generated from the anatomy field.
//!
//! Surface nets produce one vertex per intersected cubic grid cell and one
//! quad per sign-changing grid edge. This is a new topology, NOT a subdivision
//! of the old 656-quad study or a triangulated marching-tetrahedra output.
//!
//! The template is generated only for the neutral recipe and cached. Other
//! recipes deform the same indexed topology in the cage fitter. This first
//! pass does not yet align edges with anatomical muscle directions, provide
//! skin weights, or guarantee animation-quality topology.

use crate::anatomy::AnatomyTarget;
use crate::mesh::{MeshBuilder, SurfaceMesh};
use crate::quad_flow::{optimize, FlowReport};
use crate::shoulder_patch::{
    apply as refine_shoulders, measured_hotspot_cost, ShoulderPatchReport
};
use crate::shoulder_rings::{apply as build_shoulder_rings, RingPatchReport};
use crate::{Model, Recipe};
use glam::Vec3;
use std::sync::OnceLock;

const STEP: f32 = 0.025;
const MIN_BOUNDS: Vec3 = Vec3::new(-0.75, 0.10, -0.44);
const MAX_BOUNDS: Vec3 = Vec3::new(0.75, 1.80, 0.44);
const CUBE_CORNERS: [(usize, usize, usize); 8] = [
    (0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0),
    (0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1),
];
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
];

struct Grid {
    nodes: [usize; 3],
    origin: Vec3,
    delta: Vec3,
    samples: Vec<f32>,
}

impl Grid {
    fn new(target: &AnatomyTarget) -> Result<Self, String> {
        let span = MAX_BOUNDS - MIN_BOUNDS;
        let cells = [
            (span.x / STEP).ceil() as usize,
            (span.y / STEP).ceil() as usize,
            (span.z / STEP).ceil() as usize,
        ];
        let nodes = [cells[0] + 1, cells[1] + 1, cells[2] + 1];
        let origin = MIN_BOUNDS;
        let delta = span / Vec3::new(cells[0] as f32, cells[1] as f32, cells[2] as f32);
        let mut grid = Self {
            nodes, origin, delta,
            samples: vec![0.0; nodes[0] * nodes[1] * nodes[2]],
        };
        for z in 0..nodes[2] {
            for y in 0..nodes[1] {
                for x in 0..nodes[0] {
                    let point = grid.position(x, y, z);
                    let value = target.field(point);
                    if !value.is_finite() {
                        return Err("Anatomical template encountered a non-finite field".into());
                    }
                    if (x == 0 || y == 0 || z == 0
                        || x == nodes[0] - 1 || y == nodes[1] - 1
                        || z == nodes[2] - 1) && value <= 0.0
                    {
                        return Err("Neutral anatomy touches the automatic quad-grid boundary".into());
                    }
                    let index = grid.index(x, y, z);
                    grid.samples[index] = value;
                }
            }
        }
        Ok(grid)
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.nodes[1] + y) * self.nodes[0] + x
    }

    fn position(&self, x: usize, y: usize, z: usize) -> Vec3 {
        self.origin + self.delta * Vec3::new(x as f32, y as f32, z as f32)
    }

    fn value(&self, x: usize, y: usize, z: usize) -> f32 {
        self.samples[self.index(x, y, z)]
    }

    fn crossed(&self, a: [usize; 3], b: [usize; 3]) -> bool {
        (self.value(a[0], a[1], a[2]) < 0.0)
            != (self.value(b[0], b[1], b[2]) < 0.0)
    }
}

fn active_cell_vertex(grid: &Grid, x: usize, y: usize, z: usize) -> Option<Vec3> {
    let values = CUBE_CORNERS.map(|(dx, dy, dz)| grid.value(x + dx, y + dy, z + dz));
    if values.iter().all(|&v| v >= 0.0) || values.iter().all(|&v| v < 0.0) {
        return None;
    }
    let mut sum = Vec3::ZERO;
    let mut count = 0;
    for (a, b) in CUBE_EDGES {
        let va = values[a];
        let vb = values[b];
        if (va < 0.0) == (vb < 0.0) { continue; }
        let (ax, ay, az) = CUBE_CORNERS[a];
        let (bx, by, bz) = CUBE_CORNERS[b];
        let p0 = grid.position(x + ax, y + ay, z + az);
        let p1 = grid.position(x + bx, y + by, z + bz);
        let t = (va / (va - vb)).clamp(0.0, 1.0);
        sum += p0.lerp(p1, t);
        count += 1;
    }
    if count == 0 { None } else { Some(sum / count as f32) }
}

fn make_template() -> Result<NeutralTemplate, String> {
    let neutral = AnatomyTarget::from_recipe(&Recipe::default())?;
    let grid = Grid::new(&neutral)?;
    let cells = [grid.nodes[0] - 1, grid.nodes[1] - 1, grid.nodes[2] - 1];
    let cell_count = cells[0] * cells[1] * cells[2];
    let cell_index = |x: usize, y: usize, z: usize| (z * cells[1] + y) * cells[0] + x;
    let mut cell_vertices = vec![u32::MAX; cell_count];
    let mut builder = MeshBuilder::default();
    for z in 0..cells[2] {
        for y in 0..cells[1] {
            for x in 0..cells[0] {
                if let Some(p) = active_cell_vertex(&grid, x, y, z) {
                    cell_vertices[cell_index(x, y, z)] = builder.vertex(p);
                }
            }
        }
    }
    let mut insert_quad = |coords: [[usize; 3]; 4], outward_positive: bool|
        -> Result<(), String>
    {
        let ids = coords.map(|p| cell_vertices[cell_index(p[0], p[1], p[2])]);
        if ids.contains(&u32::MAX) {
            return Err("Surface-net quad encountered a missing intersected cell".into());
        }
        let [a, b, c, d] = ids;
        if outward_positive {
            builder.quad(a, b, c, d);
        } else {
            builder.quad(a, d, c, b);
        }
        Ok(())
    };

    // Every sign-changing primal edge has exactly four incident dual cells.
    // Axis-specific cell order gives +axis normals; flip if inside is on the
    // positive axis side. Only interior edges are visited, so no open caps.
    for z in 1..cells[2] {
        for y in 1..cells[1] {
            for x in 0..cells[0] {
                if grid.crossed([x,y,z], [x+1,y,z]) {
                    insert_quad([
                        [x,y-1,z-1], [x,y,z-1],
                        [x,y,z], [x,y-1,z],
                    ], grid.value(x,y,z) < 0.0)?;
                }
            }
        }
    }
    for z in 1..cells[2] {
        for y in 0..cells[1] {
            for x in 1..cells[0] {
                if grid.crossed([x,y,z], [x,y+1,z]) {
                    insert_quad([
                        [x-1,y,z-1], [x-1,y,z],
                        [x,y,z], [x,y,z-1],
                    ], grid.value(x,y,z) < 0.0)?;
                }
            }
        }
    }
    for z in 0..cells[2] {
        for y in 1..cells[1] {
            for x in 1..cells[0] {
                if grid.crossed([x,y,z], [x,y,z+1]) {
                    insert_quad([
                        [x-1,y-1,z], [x,y-1,z],
                        [x,y,z], [x-1,y,z],
                    ], grid.value(x,y,z) < 0.0)?;
                }
            }
        }
    }

    let mut mesh: SurfaceMesh = builder.build()?;
    if mesh.quads.len() <= 656 {
        return Err(format!(
            "Automatic neutral cage is unexpectedly sparse ({} quads)",
            mesh.quads.len()
        ));
    }
    // A surface net may be closed while containing multiple components.
    // Check that the quad graph represents exactly one connected character.
    let mut neighbours = vec![Vec::<usize>::new(); mesh.vertices.len()];
    for [a, b, c, d] in mesh.quads.iter().copied() {
        for (u,v) in [(a,b), (b,c), (c,d), (d,a)] {
            neighbours[u as usize].push(v as usize);
            neighbours[v as usize].push(u as usize);
        }
    }
    let mut seen = vec![false; neighbours.len()];
    let mut stack = vec![0_usize];
    seen[0] = true;
    while let Some(current) = stack.pop() {
        for &next in &neighbours[current] {
            if !seen[next] {
                seen[next] = true;
                stack.push(next);
            }
        }
    }
    if seen.iter().any(|&visited| !visited) {
        return Err("Automatic neutral cage contains disconnected components".into());
    }
    // Refine quad CONNECTIONS around each shoulder, once at neutral. The
    // closed manifold boundary is preserved by every local two-quad patch.
    // The fitter and rigger later reuse these indices unchanged.
    let mut shoulder_report = refine_shoulders(&mut mesh, &neutral)?;
    // Replace an annular shoulder band with anatomical support loops before
    // vertex flow and well before any runtime morphology or rigging.
    let (ring_report, shoulder_guides) = build_shoulder_rings(&mut mesh, &neutral)?;
    shoulder_report.hotspot_cost_after = measured_hotspot_cost(&mesh, &neutral)?;
    // After rewiring, redistribute neutral vertex positions without further
    // topology changes. Everything is cached for the archetype.
    let flow_report = optimize(&mut mesh, &neutral)?;
    let model = Model {
        vertices: mesh.vertices.iter().map(|p| p.to_array()).collect(),
        normals: mesh.normals(),
        quads: mesh.quads,
        triangles: mesh.triangles,
        caps: mesh.caps,
        shoulder_guides,
        joints: Vec::new(),
        influences: Vec::new(),
    };
    Ok(NeutralTemplate { model, flow_report, shoulder_report, ring_report })
}

struct NeutralTemplate {
    model: Model,
    flow_report: FlowReport,
    shoulder_report: ShoulderPatchReport,
    ring_report: RingPatchReport,
}

static NEUTRAL_TEMPLATE: OnceLock<Result<NeutralTemplate, String>> = OnceLock::new();

/// Constant quad connectivity for this humanoid archetype. Never resample
/// this grid for player slider updates; only fit vertex positions afterward.
pub fn neutral_template() -> Result<&'static Model, String> {
    NEUTRAL_TEMPLATE
        .get_or_init(make_template)
        .as_ref()
        .map(|template| &template.model)
        .map_err(Clone::clone)
}

/// Diagnostic metrics for neutral vertex redistribution and pole distribution.
/// The template is always cached and independent of runtime slider changes.
pub fn neutral_flow_report() -> Result<&'static FlowReport, String> {
    NEUTRAL_TEMPLATE
        .get_or_init(make_template)
        .as_ref()
        .map(|template| &template.flow_report)
        .map_err(Clone::clone)
}

/// Report actual neutral quad connectivity changes near the shoulders.
pub fn neutral_shoulder_report() -> Result<&'static ShoulderPatchReport, String> {
    NEUTRAL_TEMPLATE
        .get_or_init(make_template)
        .as_ref()
        .map(|template| &template.shoulder_report)
        .map_err(Clone::clone)
}

/// Structured neutral shoulder patch counts and welded interface ring sizes.
pub fn neutral_shoulder_ring_report() -> Result<&'static RingPatchReport, String> {
    NEUTRAL_TEMPLATE
        .get_or_init(make_template)
        .as_ref()
        .map(|template| &template.ring_report)
        .map_err(Clone::clone)
}
