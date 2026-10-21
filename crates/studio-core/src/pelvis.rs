//! Connected procedural pelvis and paired upper-thigh surface patches.
//! The underside of one torso loop becomes a 2-hole quad grid, then each
//! opening is stitched to an upper-leg loft. No overlapping cylinders, caps
//! over hip sockets, mesh unions, or retopology.

use crate::mesh::MeshBuilder;
use crate::{Recipe, VertexInfluence};
use glam::Vec3;
use std::f32::consts::TAU;

const GRID: usize = 8;
const GRID_POSITIONS: [f32; 9] = [0.0, 0.08, 0.24, 0.42, 0.50, 0.58, 0.76, 0.92, 1.0];
const DEPTH_POSITIONS: [f32; 9] = [0.0, 0.07, 0.20, 0.34, 0.50, 0.66, 0.80, 0.93, 1.0];
const THIGH_RING_COUNT: usize = 12;

#[derive(Clone, Copy)]
struct PelvisDimensions {
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
impl From<&Recipe> for PelvisDimensions {
    fn from(recipe: &Recipe) -> Self {
        Self {
            scale: recipe.height / 1.75,
            torso_length: recipe.morphology.torso_length,
            hip_width: recipe.morphology.hip_width,
            thigh_girth: recipe.morphology.thigh_girth,
            leg_length: recipe.leg_length,
            stance_width: recipe.morphology.stance_width,
            pelvis_tilt: recipe.morphology.pelvis_tilt,
            glute_volume: recipe.morphology.glute_volume,
            body_mass: recipe.body_mass,
        }
    }
}
impl PelvisDimensions {
    fn y(self, value: f32) -> f32 {
        (0.78 + (value - 0.78) * self.torso_length) * self.scale
    }

    fn outer_point(self, column: usize) -> Vec3 {
        let angle = TAU * column as f32 / 32.0;
        let sin = angle.sin();
        let cos = angle.cos();
        let posterior = (-cos).max(0.0);
        let width = 0.183 * self.hip_width * self.scale
            * (1.0 + 0.12 * (self.body_mass - 0.5));
        let depth = (0.145
            + 0.056 * (self.glute_volume - 0.5) * posterior)
            * self.scale;
        Vec3::new(
            sin * width,
            self.y(0.665) + 0.032 * (self.pelvis_tilt - 0.5) * cos * self.scale,
            cos * depth,
        )
    }

    fn hip(self, side: f32) -> Vec3 {
        Vec3::new(
            side * 0.097 * self.hip_width * self.scale,
            self.y(0.755),
            0.0,
        )
    }
    fn knee(self, side: f32) -> Vec3 {
        self.hip(side) + Vec3::new(
            side * 0.035 * (self.stance_width - 1.0) * self.scale,
            -0.338 * self.leg_length * self.scale,
            0.0,
        )
    }
}

pub fn joint_positions(recipe: &Recipe, side: f32) -> (Vec3, Vec3) {
    let d = PelvisDimensions::from(recipe);
    (d.hip(side), d.knee(side))
}

pub struct LowerBody {
    pub influences: Vec<(u32, VertexInfluence)>,
    pub leg_rings: [Vec<Vec<u32>>; 2],
}

fn weight(pelvis: f32, hip_joint: u16) -> VertexInfluence {
    VertexInfluence {
        joint_indices: [0, hip_joint, 0, 0],
        weights: [pelvis, 1.0 - pelvis, 0.0, 0.0],
    }
}

// Number the 32 outside grid vertices to match the existing Y-up torso
// circle: 0 at anterior midline, 8 right, 16 posterior, 24 left.
fn outside_index(i: usize, j: usize) -> Option<usize> {
    if j == GRID {
        return Some(if i <= 4 { (28 + i) % 32 } else { i - 4 });
    }
    if i == GRID { return Some(12 - j); }
    if j == 0 { return Some(20 - i); }
    if i == 0 { return Some(20 + j); }
    None
}

fn coons_position(i: usize, j: usize, outside: &[Vec3; 32]) -> Vec3 {
    let u = GRID_POSITIONS[i];
    let v = DEPTH_POSITIONS[j];
    let boundary = |i, j| outside[outside_index(i, j).expect("outer grid point")];
    let left = boundary(0, j);
    let right = boundary(GRID, j);
    let back = boundary(i, 0);
    let front = boundary(i, GRID);
    let corners = boundary(0, 0) * ((1.0 - u) * (1.0 - v))
        + boundary(GRID, 0) * (u * (1.0 - v))
        + boundary(0, GRID) * ((1.0 - u) * v)
        + boundary(GRID, GRID) * (u * v);
    left * (1.0 - u) + right * u
        + back * (1.0 - v) + front * v - corners
}

fn thigh_hole(grid: &[[u32; 9]; 9], start: usize) -> Vec<u32> {
    let end = start + 2;
    let mut rim = Vec::with_capacity(THIGH_RING_COUNT);
    for i in start..end { rim.push(grid[2][i]); }
    for row in 2..6 { rim.push(grid[row][end]); }
    for i in (start + 1..=end).rev() { rim.push(grid[6][i]); }
    for row in (3..=6).rev() { rim.push(grid[row][start]); }
    assert_eq!(rim.len(), THIGH_RING_COUNT);
    rim
}

fn is_hole_cell(i: usize, j: usize) -> bool {
    (1..3).contains(&i) && (2..6).contains(&j)
        || (5..7).contains(&i) && (2..6).contains(&j)
}

/// Extend an unclosed torso bottom ring into one manifold surface with two
/// upper legs. `crotch_id` is an already reserved original torso vertex, so
/// the established arm vertex indices do not change.
pub fn append(
    mesh: &mut MeshBuilder,
    torso_bottom: &[u32],
    crotch_id: u32,
    recipe: &Recipe,
) -> Result<LowerBody, String> {
    if torso_bottom.len() != 32 || crotch_id as usize >= mesh.vertices.len() {
        return Err("Pelvis requires 32 torso vertices and a crotch centre".into());
    }
    let dims = PelvisDimensions::from(recipe);
    let skirt = mesh.loop_vertices((0..32).map(|column| dims.outer_point(column)));
    mesh.stitch(&skirt, torso_bottom)?;

    let mut grid = [[0_u32; 9]; 9];
    let outside: [Vec3; 32] = std::array::from_fn(|column| mesh.vertices[skirt[column] as usize]);
    let mut influences = Vec::new();
    for j in 0..=GRID {
        for i in 0..=GRID {
            grid[j][i] = if let Some(column) = outside_index(i, j) {
                skirt[column]
            } else {
                let position = coons_position(i, j, &outside);
                if i == 4 && j == 4 {
                    mesh.vertices[crotch_id as usize] = position;
                    crotch_id
                } else {
                    mesh.vertex(position)
                }
            };
            influences.push((grid[j][i], weight(1.0, 0)));
        }
    }

    // Rectangular 2×4 quad holes create two disjoint leg-boundary loops.
    // Grid faces run downward; the holes run in reverse boundary orientation
    // and are stitched directly to thigh rings below the pelvic plane.
    for row in 0..GRID {
        for column in 0..GRID {
            if is_hole_cell(column, row) { continue; }
            mesh.quad(
                grid[row][column], grid[row][column + 1],
                grid[row + 1][column + 1], grid[row + 1][column],
            );
        }
    }

    let mut leg_rings = [Vec::new(), Vec::new()];
    for (index, (side, start, joint)) in [
        (-1.0_f32, 1_usize, 10_u16),
        (1.0_f32, 5_usize, 12_u16),
    ].into_iter().enumerate() {
        let hole = thigh_hole(&grid, start);
        let hole_centre = hole.iter()
            .map(|&id| mesh.vertices[id as usize]).sum::<Vec3>()
            / hole.len() as f32;
        let (hip, knee) = joint_positions(recipe, side);
        let direction = (knee - hip).normalize();
        // Retain the angular correspondence of the pelvis grid's hole rim.
        let angles: Vec<f32> = hole.iter().map(|&id| {
            let offset = mesh.vertices[id as usize] - hole_centre;
            offset.z.atan2(offset.x)
        }).collect();
        let radius_limit = hip.x.abs() * 0.78;
        let mut previous = hole;
        for (band, (fraction, width, depth, pelvis_weight)) in [
            (0.20_f32, 0.062_f32, 0.082_f32, 0.42_f32),
            (0.44, 0.061, 0.077, 0.17),
            (0.72, 0.054, 0.065, 0.0),
            (1.0, 0.043, 0.054, 0.0),
        ].into_iter().enumerate() {
            let centre = hole_centre.lerp(knee, fraction);
            let radius_x = (width * dims.thigh_girth * dims.scale)
                .min(radius_limit);
            let radius_z = depth * dims.thigh_girth * dims.scale
                * (1.0 + 0.08 * (dims.body_mass - 0.5));
            let next = mesh.loop_vertices(angles.iter().map(|angle| {
                // The upper-most ring blends towards the pelvic opening;
                // knee rings gradually taper. Later pose correctives can
                // reuse exactly these stable ring indices.
                let roundness = if band == 0 { 1.0 } else { 0.96 };
                centre + Vec3::new(
                    angle.cos() * radius_x * roundness,
                    direction.y.abs() * (fraction * 0.001) * dims.scale,
                    angle.sin() * radius_z * roundness,
                )
            }));
            mesh.stitch(&previous, &next)?;
            for &vertex in &next {
                influences.push((vertex, weight(pelvis_weight, joint)));
            }
            leg_rings[index].push(next.clone());
            previous = next;
        }
        let cap_id = mesh.vertices.len() as u32;
        mesh.cap(&previous, knee - Vec3::Y * (0.008 * dims.scale), false)?;
        influences.push((cap_id, weight(0.0, joint)));
    }
    Ok(LowerBody { influences, leg_rings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hole_boundaries_each_have_twelve_distinct_vertices() {
        let grid: [[u32; 9]; 9] =
            std::array::from_fn(|j| std::array::from_fn(|i| (j * 9 + i) as u32));
        for x in [1, 5] {
            let boundary = thigh_hole(&grid, x);
            assert_eq!(boundary.len(), 12);
            let unique: std::collections::HashSet<_> = boundary.iter().collect();
            assert_eq!(unique.len(), 12);
        }
    }

    #[test]
    fn pelvis_grid_has_two_holes_and_an_unchanged_32_vertex_perimeter() {
        assert_eq!((0..GRID).flat_map(|j| (0..GRID).map(move |i| (i, j)))
            .filter(|&(i, j)| !is_hole_cell(i, j)).count(), 48);
        let ids: Vec<_> = (0..=GRID).flat_map(|j|
            (0..=GRID).filter_map(move |i| outside_index(i, j))).collect();
        assert_eq!(ids.len(), 32);
        let distinct: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(distinct.len(), 32);
    }

    #[test]
    fn hip_width_and_stance_affect_hip_knee_landmarks() {
        let neutral = Recipe::default();
        let mut broad = neutral.clone();
        broad.morphology.hip_width = 1.2;
        broad.morphology.stance_width = 1.2;
        let (hip, knee) = joint_positions(&neutral, 1.0);
        let (wide_hip, wide_knee) = joint_positions(&broad, 1.0);
        assert!(wide_hip.x > hip.x);
        assert!(wide_knee.x > knee.x);
    }
}
