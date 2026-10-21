//! Anatomical, topology-preserving pelvis-to-thigh bifurcation.
//!
//! Three boundary curves define one connected "pair of trousers" surface:
//! a 32-vertex waist and two 12-vertex upper-thigh roots. Anatomical roots
//! are constructed first; the remaining pelvic quads are harmonically fitted
//! between those boundaries. There is no horizontal pelvic lid or leg tube
//! attached to a separate skirt.

use crate::mesh::MeshBuilder;
use crate::{Recipe, VertexInfluence};
use glam::Vec3;
use std::f32::consts::{PI, TAU};

const GRID: usize = 8;
const THIGH_RING_COUNT: usize = 12;
const UNUSED: u32 = u32::MAX;
const FAIRING_PASSES: usize = 180;

#[derive(Clone, Copy)]
struct Dimensions {
    scale: f32,
    torso_length: f32,
    hip_width: f32,
    thigh_girth: f32,
    leg_length: f32,
    stance_width: f32,
    pelvis_tilt: f32,
    glute_volume: f32,
    body_mass: f32,
}

impl From<&Recipe> for Dimensions {
    fn from(recipe: &Recipe) -> Self {
        let m = &recipe.morphology;
        Self {
            scale: recipe.height / 1.75,
            torso_length: m.torso_length,
            hip_width: m.hip_width,
            thigh_girth: m.thigh_girth,
            leg_length: recipe.leg_length,
            stance_width: m.stance_width,
            pelvis_tilt: m.pelvis_tilt,
            glute_volume: m.glute_volume,
            body_mass: recipe.body_mass,
        }
    }
}

impl Dimensions {
    fn y(self, y: f32) -> f32 {
        (0.78 + (y - 0.78) * self.torso_length) * self.scale
    }

    fn hip(self, side: f32) -> Vec3 {
        Vec3::new(side * 0.097 * self.hip_width * self.scale, self.y(0.755), 0.0)
    }

    fn knee(self, side: f32) -> Vec3 {
        self.hip(side)
            + Vec3::new(
                side * 0.035 * (self.stance_width - 1.0) * self.scale,
                -0.338 * self.leg_length * self.scale,
                0.0,
            )
    }

    // Proximal femur is inside the pelvic mass. The root circumference is
    // *not* a horizontal ring: lateral greater-trochanter contours climb
    // towards the hips while the medial edges descend into the crotch.
    fn root(self, side: f32, index: usize) -> Vec3 {
        let phi = -3.0 * PI / 4.0 + TAU * index as f32 / THIGH_RING_COUNT as f32;
        let cos = phi.cos();
        let sin = phi.sin();
        let centre_x = side * 0.100 * self.hip_width * self.scale;
        let radius_x = (0.083 * self.thigh_girth
            * (1.0 + 0.10 * (self.body_mass - 0.5)) * self.scale)
            .min(centre_x.abs() * 0.87);
        let radius_z = 0.113 * self.thigh_girth
            * (1.0 + 0.12 * (self.body_mass - 0.5))
            * self.scale;
        let outward = side * cos;
        Vec3::new(
            centre_x + cos * radius_x,
            self.y(0.655) + (0.094 * outward
                + 0.032 * (self.pelvis_tilt - 0.5) * sin) * self.scale,
            sin * radius_z - (0.012
                + 0.035 * (self.glute_volume - 0.5) * (-sin).max(0.0))
                * self.scale,
        )
    }
}

pub fn joint_positions(recipe: &Recipe, side: f32) -> (Vec3, Vec3) {
    let d = Dimensions::from(recipe);
    (d.hip(side), d.knee(side))
}

pub struct LowerBody {
    pub influences: Vec<(u32, VertexInfluence)>,
    pub leg_rings: [Vec<Vec<u32>>; 2],
}

fn weight(pelvis: f32, hip: u16) -> VertexInfluence {
    VertexInfluence {
        joint_indices: [0, hip, 0, 0],
        weights: [pelvis, 1.0 - pelvis, 0.0, 0.0],
    }
}

// The outside of the 9x9 grid is the *existing* lower torso circumference.
// Column 0 faces forward, 8 is right lateral, 16 is posterior, 24 left.
fn outside_index(i: usize, j: usize) -> Option<usize> {
    if j == GRID {
        return Some(if i <= 4 { (28 + i) % 32 } else { i - 4 });
    }
    if i == GRID {
        return Some(12 - j);
    }
    if j == 0 {
        return Some(20 - i);
    }
    if i == 0 {
        return Some(20 + j);
    }
    None
}

/// All three strictly interior lattice points of each 2×4 thigh socket
/// belong exclusively to omitted cells. Allocating any of them produces
/// orphaned vertices (and shifts deterministic output counts).
fn unused_inside_hole(i: usize, j: usize) -> bool {
    (i == 2 || i == 6) && (3..=5).contains(&j)
}

fn is_hole_cell(i: usize, j: usize) -> bool {
    ((1..3).contains(&i) || (5..7).contains(&i)) && (2..6).contains(&j)
}

fn thigh_hole(grid: &[[u32; 9]; 9], start: usize) -> Vec<u32> {
    let end = start + 2;
    let mut rim = Vec::with_capacity(THIGH_RING_COUNT);
    for i in start..end {
        rim.push(grid[2][i]);
    }
    for row in 2..6 {
        rim.push(grid[row][end]);
    }
    for i in (start + 1..=end).rev() {
        rim.push(grid[6][i]);
    }
    for row in (3..=6).rev() {
        rim.push(grid[row][start]);
    }
    rim
}

fn add_edge(neighbours: &mut [Vec<usize>], a: u32, b: u32) {
    neighbours[a as usize].push(b as usize);
    neighbours[b as usize].push(a as usize);
}

/// Solve the free pelvic vertices between the fixed waist and both thigh
/// roots. Iteration uses only authored quad edges (never rendering diagonals).
/// The waist, thigh sockets, and existing shoulder topology remain untouched.
fn fit_pelvis(
    mesh: &mut MeshBuilder,
    grid: &[[u32; 9]; 9],
    pinned: &[bool],
    dimensions: Dimensions,
) -> Result<(), String> {
    let mut neighbours = vec![Vec::<usize>::new(); mesh.vertices.len()];
    for row in 0..GRID {
        for col in 0..GRID {
            if is_hole_cell(col, row) {
                continue;
            }
            let q = [
                grid[row][col],
                grid[row][col + 1],
                grid[row + 1][col + 1],
                grid[row + 1][col],
            ];
            for (a, b) in [(q[0], q[1]), (q[1], q[2]), (q[2], q[3]), (q[3], q[0])] {
                if a == UNUSED || b == UNUSED {
                    return Err("Pelvis uses an uninitialized socket vertex".into());
                }
                add_edge(&mut neighbours, a, b);
            }
        }
    }
    let mut free = Vec::new();
    for row in grid {
        for &id in row {
            if id != UNUSED && !pinned[id as usize] && !free.contains(&(id as usize)) {
                free.push(id as usize);
            }
        }
    }
    for neighbours in &mut neighbours {
        neighbours.sort_unstable();
        neighbours.dedup();
    }
    for _ in 0..FAIRING_PASSES {
        let before = mesh.vertices.clone();
        for &id in &free {
            let adjacent = &neighbours[id];
            if adjacent.is_empty() {
                return Err("Disconnected pelvic patch vertex".into());
            }
            let average = adjacent.iter().map(|&other| before[other]).sum::<Vec3>()
                / adjacent.len() as f32;
            mesh.vertices[id] = before[id].lerp(average, 0.72);
        }
    }
    // Rounded posterior gluteal mass. Apply it only to unconstrained vertices;
    // boundary curves and connected thigh seam positions are exact.
    for &id in &free {
        let p = mesh.vertices[id];
        let posterior = (-p.z / (0.12 * dimensions.scale)).max(0.0);
        let near_hip = (1.0
            - ((p.x.abs() - 0.108 * dimensions.hip_width * dimensions.scale).abs()
                / (0.108 * dimensions.hip_width * dimensions.scale)))
            .clamp(0.0, 1.0);
        mesh.vertices[id].z -= 0.018 * (0.65 + 0.7 * dimensions.glute_volume)
            * posterior * near_hip * dimensions.scale;
    }
    Ok(())
}

/// One shared surface with three anatomical boundaries: a torso ring and two
/// curved thigh roots. A fixed quad-patch graph connects them, and its
/// positions are *solved from the roots* rather than generated as a flat lid.
/// The reserved original crotch vertex keeps shoulder indices unchanged.
pub fn append(
    mesh: &mut MeshBuilder,
    torso_bottom: &[u32],
    crotch_id: u32,
    recipe: &Recipe,
) -> Result<LowerBody, String> {
    if torso_bottom.len() != 32 || crotch_id as usize >= mesh.vertices.len() {
        return Err("Pelvis requires the existing 32-vertex waist and crotch anchor".into());
    }
    let d = Dimensions::from(recipe);
    let mut grid = [[UNUSED; 9]; 9];

    // Allocate *only* vertices used by faces. Each 2×4 socket omits three
    // grid vertices along its centreline, not just the central one.
    for j in 0..=GRID {
        for i in 0..=GRID {
            grid[j][i] = if let Some(column) = outside_index(i, j) {
                torso_bottom[column]
            } else if unused_inside_hole(i, j) {
                UNUSED
            } else if i == 4 && j == 4 {
                crotch_id
            } else {
                mesh.vertex(Vec3::new(
                    (i as f32 / GRID as f32 - 0.5) * 0.30 * d.scale,
                    d.y(0.70),
                    (j as f32 / GRID as f32 - 0.5) * 0.25 * d.scale,
                ))
            };
        }
    }

    let mut pinned = vec![false; mesh.vertices.len()];
    for &id in torso_bottom {
        pinned[id as usize] = true;
    }

    let mut influences = Vec::new();
    let mut root_loops = [Vec::<u32>::new(), Vec::<u32>::new()];
    for (which, (side, start, hip_joint)) in [
        (-1.0_f32, 1_usize, 10_u16),
        (1.0_f32, 5_usize, 12_u16),
    ].into_iter().enumerate() {
        let rim = thigh_hole(&grid, start);
        if rim.len() != THIGH_RING_COUNT || rim.contains(&UNUSED) {
            return Err("Invalid anatomical thigh socket boundary".into());
        }
        for (i, &id) in rim.iter().enumerate() {
            mesh.vertices[id as usize] = d.root(side, i);
            pinned[id as usize] = true;
            influences.push((id, weight(0.75, hip_joint)));
        }
        root_loops[which] = rim;
    }

    // The patch is a connected 3-boundary surface, not a closing pelvis cap.
    for j in 0..GRID {
        for i in 0..GRID {
            if is_hole_cell(i, j) {
                continue;
            }
            mesh.quad(
                grid[j][i],
                grid[j][i + 1],
                grid[j + 1][i + 1],
                grid[j + 1][i],
            );
        }
    }
    fit_pelvis(mesh, &grid, &pinned, d)?;

    for row in &grid {
        for &id in row {
            if id != UNUSED && !pinned[id as usize] {
                influences.push((id, weight(1.0, 0)));
            }
        }
    }

    let mut leg_rings = [Vec::new(), Vec::new()];
    for (side_index, (side, joint)) in [(-1.0_f32, 10_u16), (1.0, 12_u16)]
        .into_iter()
        .enumerate()
    {
        let root = &root_loops[side_index];
        let knee = d.knee(side);
        let mut previous = root.clone();
        // Anatomical thigh surfaces grow continuously *from* the socket ring.
        // The curved socket is high laterally and low at the inner crotch.
        for (band, fraction) in [0.18_f32, 0.42, 0.70, 1.0].into_iter().enumerate() {
            let blend = fraction * fraction * (3.0 - 2.0 * fraction);
            let roots: Vec<Vec3> = root.iter()
                .map(|&id| mesh.vertices[id as usize]).collect();
            let next = mesh.loop_vertices(roots.into_iter().enumerate().map(|(i, origin)| {
                let phi = -3.0 * PI / 4.0 + TAU * i as f32 / THIGH_RING_COUNT as f32;
                let target = Vec3::new(
                    knee.x + phi.cos() * 0.052 * d.thigh_girth * d.scale,
                    knee.y,
                    phi.sin() * 0.064 * d.thigh_girth * d.scale - 0.010 * d.scale,
                );
                let mut point = origin.lerp(target, blend);
                point.y = origin.y * (1.0 - fraction) + knee.y * fraction;
                point
            }));
            if previous.iter().zip(&next).any(|(&a, &b)| {
                mesh.vertices[b as usize].y >= mesh.vertices[a as usize].y - 1.0e-5
            }) {
                return Err(format!("Thigh ring {band} folds upward toward the pelvis"));
            }
            mesh.stitch(&previous, &next)?;
            for &id in &next {
                influences.push((id, weight([0.42, 0.17, 0.0, 0.0][band], joint)));
            }
            leg_rings[side_index].push(next.clone());
            previous = next;
        }
        let centre_id = mesh.vertices.len() as u32;
        mesh.cap(&previous, knee - Vec3::Y * 0.006 * d.scale, false)?;
        influences.push((centre_id, weight(0.0, joint)));
    }
    Ok(LowerBody { influences, leg_rings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn anatomical_roots_slope_toward_the_crotch() {
        let d = Dimensions::from(&Recipe::default());
        for side in [-1.0, 1.0] {
            let root = (0..12).map(|index| d.root(side, index)).collect::<Vec<_>>();
            let outside = root.iter().filter(|v| v.x * side > 0.14).map(|v| v.y)
                .fold(f32::NEG_INFINITY, f32::max);
            let inside = root.iter().filter(|v| v.x * side < 0.06).map(|v| v.y)
                .fold(f32::INFINITY, f32::min);
            assert!(outside > inside + 0.12);
        }
    }

    #[test]
    fn every_socket_interior_vertex_is_skipped() {
        let hole_points: Vec<(usize, usize)> = (1..8)
            .flat_map(|i| (1..8).map(move |j| (i, j)))
            .filter(|&(i,j)| unused_inside_hole(i,j))
            .collect();
        assert_eq!(hole_points, vec![
            (2,3), (2,4), (2,5), (6,3), (6,4), (6,5)
        ]);
        for start in [1, 5] {
            for j in 3..=5 {
                assert!(unused_inside_hole(start + 1, j));
            }
        }
    }

    #[test]
    fn three_boundary_patch_has_no_unused_socket_vertices() {
        let mut mesh = MeshBuilder::default();
        let waist = mesh.loop_vertices((0..32).map(|i| {
            let theta = TAU * i as f32 / 32.0;
            Vec3::new(theta.sin() * 0.159, 0.78, theta.cos() * 0.122)
        }));
        let upper = mesh.loop_vertices((0..32).map(|i| {
            let theta = TAU * i as f32 / 32.0;
            Vec3::new(theta.sin() * 0.150, 0.86, theta.cos() * 0.116)
        }));
        mesh.stitch(&waist, &upper).unwrap();
        let crotch = mesh.vertex(Vec3::ZERO);
        mesh.cap(&upper, Vec3::new(0.0, 0.86, 0.0), false).unwrap();
        let before = mesh.vertices.len();
        let lower = append(&mut mesh, &waist, crotch, &Recipe::default()).unwrap();
        assert_eq!(mesh.vertices.len() - before, 140);
        assert_eq!(lower.leg_rings[0].len(), 4);
        assert_eq!(lower.leg_rings[1].len(), 4);
        let surface = mesh.build().unwrap();
        let referenced: HashSet<u32> = surface.triangles.iter().flatten().copied().collect();
        assert_eq!(referenced.len(), surface.vertices.len(), "Unused / detached vertices");
        assert!(surface.vertices.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn hip_and_thigh_ranges_preserve_anatomical_geometry() {
        for hip in [0.8, 1.0, 1.25] {
            for girth in [0.7, 1.0, 1.4] {
                for tilt in [0.0, 1.0] {
                    let mut recipe = Recipe::default();
                    recipe.morphology.hip_width = hip;
                    recipe.morphology.thigh_girth = girth;
                    recipe.morphology.pelvis_tilt = tilt;
                    let d = Dimensions::from(&recipe);
                    let left = (0..12).map(|i| d.root(-1.0, i)).collect::<Vec<_>>();
                    let right = (0..12).map(|i| d.root(1.0, i)).collect::<Vec<_>>();
                    let left_inner = left.iter().map(|v| v.x).fold(f32::NEG_INFINITY, f32::max);
                    let right_inner = right.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
                    assert!(right_inner - left_inner > 0.020, "Thigh root overlap");
                    assert!(d.knee(-1.0).y < left.iter().map(|v| v.y)
                        .fold(f32::INFINITY, f32::min));
                }
            }
        }
    }

    #[test]
    fn hip_width_and_stance_move_correct_bones() {
        let mut edited = Recipe::default();
        edited.morphology.hip_width = 1.25;
        edited.morphology.stance_width = 1.20;
        let (hip, knee) = joint_positions(&Recipe::default(), 1.0);
        let (wide_hip, wide_knee) = joint_positions(&edited, 1.0);
        assert!(wide_hip.x > hip.x);
        assert!(wide_knee.x > knee.x);
    }
}
