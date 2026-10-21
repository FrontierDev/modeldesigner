//! Parametric torso/shoulder/upper-arm construction study.
//!
//! This is deliberately *not* a complete humanoid: it proves that the
//! generic topology engine can build a branching, closed quad surface with
//! anatomical controls and no imported base mesh or implicit field.

use crate::{Joint, Recipe};
use crate::mesh::{MeshBuilder, SurfaceMesh};
use glam::Vec3;
use std::f32::consts::TAU;

const RING_SIDES: usize = 32;
const ARM_HOLE_START_ROW: usize = 6;
const ARM_HOLE_END_ROW: usize = 10;
const ARM_HOLE_COLUMNS: usize = 4;

// y, half-width, half-depth. These are surface *landmarks*, not volume primitives.
// Joint and patch positions are evaluated from the same normalized measurements.
const TORSO_SECTIONS: [(f32, f32, f32); 13] = [
    (0.78, 0.144, 0.105),
    (0.86, 0.173, 0.130),
    (0.95, 0.162, 0.125),
    (1.05, 0.156, 0.118),
    (1.15, 0.167, 0.128),
    (1.24, 0.187, 0.138),
    (1.28, 0.198, 0.144),
    (1.32, 0.207, 0.147),
    (1.36, 0.220, 0.145),
    (1.40, 0.225, 0.137),
    (1.44, 0.208, 0.125),
    (1.48, 0.123, 0.090),
    (1.53, 0.069, 0.066),
];

fn smoothstep(lo: f32, hi: f32, v: f32) -> f32 {
    let t = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy)]
struct Measurements {
    height: f32,
    shoulder_width: f32,
    chest_width: f32,
    chest_depth: f32,
    waist_width: f32,
    ribcage_taper: f32,
    torso_length: f32,
    shoulder_slope: f32,
    arm_length: f32,
    upper_arm_girth: f32,
}
impl From<&Recipe> for Measurements {
    fn from(recipe: &Recipe) -> Self {
        let m = &recipe.morphology;
        Self {
            height: recipe.height / 1.75,
            shoulder_width: recipe.shoulder_width / 0.46,
            chest_width: m.chest_width,
            chest_depth: m.chest_depth,
            waist_width: m.waist_width,
            ribcage_taper: m.ribcage_taper,
            torso_length: m.torso_length,
            shoulder_slope: m.shoulder_slope,
            arm_length: recipe.arm_length,
            upper_arm_girth: m.upper_arm_girth,
        }
    }
}
impl Measurements {
    fn y(self, y: f32) -> f32 {
        (0.78 + (y - 0.78) * self.torso_length) * self.height
    }

    fn side_slope(self, side_weight: f32, y: f32) -> f32 {
        (0.5 - self.shoulder_slope) * 0.12
            * side_weight * smoothstep(1.23, 1.42, y) * self.height
    }

    fn torso_point(self, row: usize, column: usize) -> Vec3 {
        let (y, base_width, base_depth) = TORSO_SECTIONS[row];
        let angle = TAU * column as f32 / RING_SIDES as f32;
        let sin = angle.sin();
        let cos = angle.cos();
        let chest = smoothstep(1.07, 1.31, y);
        let waist = 1.0 - smoothstep(1.00, 1.21, y);
        let width = base_width
            * (1.0 + chest * (self.chest_width - 1.0))
            * (1.0 + chest * (self.shoulder_width - 1.0) * 0.7)
            * (1.0 + waist * (self.waist_width - 1.0))
            * (1.0 + (1.0 - waist) * (1.0 - chest) * (self.ribcage_taper - 1.0));
        let depth = base_depth * (1.0 + chest * (self.chest_depth - 1.0));
        // Very shallow relief from chest planes, tapering gently toward the axilla.
        let upper_chest = smoothstep(1.22, 1.30, y) * (1.0 - smoothstep(1.38, 1.47, y));
        let anterior = cos.max(0.0);
        let relief = 0.007 * upper_chest * anterior * smoothstep(0.02, 0.09, sin.abs());
        Vec3::new(
            sin * width * self.height,
            self.y(y) + self.side_slope(sin.abs().powi(6), y),
            (cos * depth + relief) * self.height,
        )
    }

    fn shoulder_centre(self, side: f32) -> Vec3 {
        Vec3::new(
            side * 0.263 * self.shoulder_width * self.height,
            self.y(1.356) + self.side_slope(1.0, 1.356),
            0.0,
        )
    }

    fn elbow(self, side: f32) -> Vec3 {
        let start = self.shoulder_centre(side);
        start + Vec3::new(side * 0.178, -0.193, 0.0) * (self.arm_length * self.height)
    }
}

fn on_hole(row: usize, column: usize, centre_column: usize) -> bool {
    let first = centre_column - ARM_HOLE_COLUMNS / 2;
    (ARM_HOLE_START_ROW..ARM_HOLE_END_ROW).contains(&row)
        && (first..first + ARM_HOLE_COLUMNS).contains(&column)
}

/// Ordered boundary of a rectangular hole in an indexed torso loft.
/// These are the *existing torso vertices*, not a duplicated mesh seam.
fn boundary(rings: &[Vec<u32>], centre_column: usize) -> Vec<u32> {
    let left = centre_column - ARM_HOLE_COLUMNS / 2;
    let right = left + ARM_HOLE_COLUMNS;
    let mut result = Vec::new();
    for column in left..right { result.push(rings[ARM_HOLE_START_ROW][column]); }
    for row in ARM_HOLE_START_ROW..ARM_HOLE_END_ROW { result.push(rings[row][right]); }
    for column in (left + 1..=right).rev() { result.push(rings[ARM_HOLE_END_ROW][column]); }
    for row in (ARM_HOLE_START_ROW + 1..=ARM_HOLE_END_ROW).rev() { result.push(rings[row][left]); }
    result
}

fn arm_ring(
    builder: &mut MeshBuilder,
    hole: &[u32],
    centre: Vec3,
    axis: Vec3,
    radius_y: f32,
    radius_z: f32,
) -> Vec<u32> {
    // The ellipse plane is perpendicular to the arm. We preserve correspondence
    // between the torso-hole boundary and every subsequent anatomical loop.
    let up = Vec3::new(-axis.y * axis.x.signum(), axis.x.abs(), 0.0).normalize();
    let front = Vec3::Z;
    let hole_centre = hole.iter().map(|&id| builder.vertices[id as usize]).sum::<Vec3>()
        / hole.len() as f32;
    let angles: Vec<f32> = hole.iter().map(|&id| {
        let offset = builder.vertices[id as usize] - hole_centre;
        (offset.dot(front) / 0.065).atan2(offset.dot(up) / 0.085)
    }).collect();
    builder.loop_vertices(angles.into_iter().map(|angle| {
        centre + up * (angle.cos() * radius_y) + front * (angle.sin() * radius_z)
    }))
}

pub fn build(recipe: &Recipe) -> Result<SurfaceMesh, String> {
    let m = Measurements::from(recipe);
    let mut mesh = MeshBuilder::default();
    let mut rings = Vec::new();
    for row in 0..TORSO_SECTIONS.len() {
        rings.push(mesh.loop_vertices((0..RING_SIDES).map(|column| m.torso_point(row, column))));
    }
    for row in 0..rings.len() - 1 {
        for column in 0..RING_SIDES {
            if on_hole(row, column, 8) || on_hole(row, column, 24) {
                continue;
            }
            let next = (column + 1) % RING_SIDES;
            mesh.quad(
                rings[row][column], rings[row][next],
                rings[row + 1][next], rings[row + 1][column],
            );
        }
    }
    mesh.cap(&rings[0], Vec3::new(0.0, m.y(TORSO_SECTIONS[0].0), 0.0), true)?;
    mesh.cap(&rings[rings.len() - 1],
        Vec3::new(0.0, m.y(TORSO_SECTIONS[TORSO_SECTIONS.len() - 1].0), 0.0), false)?;

    for (side, centre_column) in [(1.0_f32, 8_usize), (-1.0_f32, 24_usize)] {
        let hole = boundary(&rings, centre_column);
        let centre = m.shoulder_centre(side);
        let elbow = m.elbow(side);
        let axis = (elbow - centre).normalize();
        let outer = arm_ring(&mut mesh, &hole, centre, axis,
            0.080 * m.upper_arm_girth * m.height,
            0.074 * m.upper_arm_girth * m.height);
        mesh.stitch(&hole, &outer)?;
        let mid_centre = centre.lerp(elbow, 0.48);
        let mid = arm_ring(&mut mesh, &hole, mid_centre, axis,
            0.066 * m.upper_arm_girth * m.height,
            0.058 * m.upper_arm_girth * m.height);
        mesh.stitch(&outer, &mid)?;
        let elbow_loop = arm_ring(&mut mesh, &hole, elbow, axis,
            0.052 * m.upper_arm_girth * m.height,
            0.050 * m.upper_arm_girth * m.height);
        mesh.stitch(&mid, &elbow_loop)?;
        mesh.cap(&elbow_loop, elbow + axis * 0.004 * m.height, false)?;
    }
    mesh.build()
}

fn append_joint(joints: &mut Vec<Joint>, name: &'static str, parent: Option<usize>, point: Vec3) {
    joints.push(Joint { name, parent, position: point.to_array() });
}

pub fn skeleton(recipe: &Recipe) -> Vec<Joint> {
    let m = Measurements::from(recipe);
    let mut joints = Vec::<Joint>::new();
    append_joint(&mut joints, "pelvis", None, Vec3::new(0.0, m.y(0.84), 0.0));
    append_joint(&mut joints, "spine", Some(0), Vec3::new(0.0, m.y(1.12), 0.0));
    append_joint(&mut joints, "chest", Some(1), Vec3::new(0.0, m.y(1.37), 0.0));
    append_joint(&mut joints, "neck", Some(2), Vec3::new(0.0, m.y(1.52), 0.0));
    for side in [-1.0_f32, 1.0] {
        let shoulder = m.shoulder_centre(side);
        let clavicle_parent = 2;
        append_joint(
            &mut joints,
            if side < 0.0 { "left_clavicle" } else { "right_clavicle" },
            Some(clavicle_parent),
            shoulder.lerp(Vec3::new(0.0, m.y(1.43), 0.03 * m.height), 0.15),
        );
        let clavicle_index = joints.len() - 1;
        append_joint(
            &mut joints,
            if side < 0.0 { "left_shoulder" } else { "right_shoulder" },
            Some(clavicle_index), shoulder,
        );
        let shoulder_index = joints.len() - 1;
        append_joint(
            &mut joints,
            if side < 0.0 { "left_elbow" } else { "right_elbow" },
            Some(shoulder_index), m.elbow(side),
        );
    }
    joints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_study_is_closed_and_uses_connected_shoulder_loops() {
        let model = build(&Recipe::default()).unwrap();
        assert!(model.quads.len() > 100);
        assert!(model.triangles.len() < 3000);
        assert_eq!(skeleton(&Recipe::default()).len(), 10);
    }

    #[test]
    fn supported_parameters_preserve_face_indexing() {
        let baseline = build(&Recipe::default()).unwrap();
        let mut modified = Recipe::default();
        modified.shoulder_width = 0.60;
        modified.morphology.chest_width = 1.2;
        modified.morphology.upper_arm_girth = 1.3;
        let edited = build(&modified).unwrap();
        assert_eq!(baseline.quads, edited.quads);
        assert_eq!(baseline.triangles, edited.triangles);
        assert_ne!(baseline.vertices, edited.vertices);
    }
}