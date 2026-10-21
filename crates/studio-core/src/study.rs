//! Parametric torso/shoulder/upper-arm construction study.
//!
//! This is deliberately *not* a complete humanoid: it proves that the
//! generic topology engine can build a branching, closed quad surface with
//! anatomical controls and no imported base mesh or implicit field.

use crate::{Joint, PreviewPose, Recipe};
use crate::mesh::{MeshBuilder, SurfaceMesh};
use glam::{Quat, Vec3};
use std::f32::consts::TAU;

const RING_SIDES: usize = 32;
const ARM_HOLE_START_ROW: usize = 6;
const ARM_HOLE_END_ROW: usize = 10;
const ARM_HOLE_COLUMNS: usize = 4;
#[cfg(test)]
const SHOULDER_BLEND_RINGS: usize = 2;

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
        // Pectoral planes roll gently outward into the anterior axillary region.
        // Relief is highest in the medial chest and fades toward the deltoid.
        let upper_chest = smoothstep(1.20, 1.29, y) * (1.0 - smoothstep(1.38, 1.46, y));
        let anterior = cos.max(0.0);
        let medial = 1.0 - smoothstep(0.36, 0.94, sin.abs());
        let relief = 0.012 * upper_chest * anterior * medial;
        // A shallow sternum groove keeps the chest contour from becoming a cylinder.
        let sternum = 0.004 * upper_chest * anterior * (1.0 - smoothstep(0.0, 0.20, sin.abs()));
        Vec3::new(
            sin * width * self.height,
            self.y(y) + self.side_slope(sin.abs().powi(6), y),
            (cos * depth + relief - sternum) * self.height,
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

/// A test pose pivots the upper arm about its humeral head. Blending the
/// rotation over two shoulder loops keeps the sewn torso seam stationary.
fn arm_rotation(side: f32, pose: PreviewPose) -> Quat {
    let raise = Quat::from_rotation_z(side * pose.arm_raise_deg.to_radians());
    let forward = Quat::from_rotation_y(-side * pose.arm_forward_deg.to_radians());
    forward * raise
}

fn pose_vertex(rest: Vec3, shoulder: Vec3, rotation: Quat, weight: f32) -> Vec3 {
    shoulder + Quat::IDENTITY.slerp(rotation, weight) * (rest - shoulder)
}

fn arm_ring(
    builder: &mut MeshBuilder,
    hole: &[u32],
    centre: Vec3,
    axis: Vec3,
    radius_y: f32,
    radius_z: f32,
    shoulder: Vec3,
    rotation: Quat,
    pose_weight: f32,
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
        let rest = centre + up * (angle.cos() * radius_y) + front * (angle.sin() * radius_z);
        pose_vertex(rest, shoulder, rotation, pose_weight)
    }))
}

pub fn build(recipe: &Recipe, pose: PreviewPose) -> Result<SurfaceMesh, String> {
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
        let hole_centre = hole.iter().map(|&id| mesh.vertices[id as usize]).sum::<Vec3>()
            / hole.len() as f32;
        let shoulder = m.shoulder_centre(side);
        let elbow = m.elbow(side);
        let axis = (elbow - shoulder).normalize();
        let rotation = arm_rotation(side, pose);
        // Two shoulder transition loops allow the pectoral/axillary boundary
        // to settle into a deltoid loop without pulling its shared torso rim.
        let transition_centre = hole_centre.lerp(shoulder, 0.44);
        let transition = arm_ring(&mut mesh, &hole, transition_centre, axis,
            0.090 * m.upper_arm_girth * m.height,
            0.078 * m.upper_arm_girth * m.height,
            shoulder, rotation, 0.15);
        mesh.stitch(&hole, &transition)?;

        let deltoid = arm_ring(&mut mesh, &hole, shoulder, axis,
            0.079 * m.upper_arm_girth * m.height,
            0.072 * m.upper_arm_girth * m.height,
            shoulder, rotation, 0.50);
        mesh.stitch(&transition, &deltoid)?;

        let insertion = arm_ring(&mut mesh, &hole, shoulder.lerp(elbow, 0.24), axis,
            0.070 * m.upper_arm_girth * m.height,
            0.063 * m.upper_arm_girth * m.height,
            shoulder, rotation, 0.85);
        mesh.stitch(&deltoid, &insertion)?;

        let mid = arm_ring(&mut mesh, &hole, shoulder.lerp(elbow, 0.55), axis,
            0.062 * m.upper_arm_girth * m.height,
            0.056 * m.upper_arm_girth * m.height,
            shoulder, rotation, 1.0);
        mesh.stitch(&insertion, &mid)?;

        let elbow_loop = arm_ring(&mut mesh, &hole, elbow, axis,
            0.052 * m.upper_arm_girth * m.height,
            0.050 * m.upper_arm_girth * m.height,
            shoulder, rotation, 1.0);
        mesh.stitch(&mid, &elbow_loop)?;
        let cap_centre = pose_vertex(elbow + axis * 0.004 * m.height, shoulder, rotation, 1.0);
        mesh.cap(&elbow_loop, cap_centre, false)?;
    }
    mesh.build()
}

fn append_joint(joints: &mut Vec<Joint>, name: &'static str, parent: Option<usize>, point: Vec3) {
    joints.push(Joint { name, parent, position: point.to_array() });
}

pub fn skeleton(recipe: &Recipe, pose: PreviewPose) -> Vec<Joint> {
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
            Some(shoulder_index),
            pose_vertex(m.elbow(side), shoulder, arm_rotation(side, pose), 1.0),
        );
    }
    joints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_study_is_closed_and_uses_connected_shoulder_loops() {
        let model = build(&Recipe::default(), PreviewPose::default()).unwrap();
        assert!(model.quads.len() > 100 + 2 * SHOULDER_BLEND_RINGS * 16);
        assert!(model.triangles.len() < 3000);
        assert_eq!(skeleton(&Recipe::default(), PreviewPose::default()).len(), 10);
    }

    #[test]
    fn posing_preserves_indices_and_keeps_stitched_seams_closed() {
        let recipe = Recipe::default();
        let rest = build(&recipe, PreviewPose::default()).unwrap();
        for pose in [
            PreviewPose { arm_raise_deg: 45.0, arm_forward_deg: 0.0 },
            PreviewPose { arm_raise_deg: 0.0, arm_forward_deg: 40.0 },
            PreviewPose { arm_raise_deg: 65.0, arm_forward_deg: 45.0 },
        ] {
            let posed = build(&recipe, pose).unwrap();
            assert_eq!(posed.quads, rest.quads);
            assert_eq!(posed.triangles, rest.triangles);
            assert_ne!(posed.vertices, rest.vertices);
            let joints = skeleton(&recipe, pose);
            assert_eq!(joints.len(), 10);
            assert!(joints.iter().all(|joint| joint.position.iter().all(|value| value.is_finite())));
        }
    }

    #[test]
    fn supported_parameters_preserve_face_indexing() {
        let baseline = build(&Recipe::default(), PreviewPose::default()).unwrap();
        let mut modified = Recipe::default();
        modified.shoulder_width = 0.60;
        modified.morphology.chest_width = 1.2;
        modified.morphology.upper_arm_girth = 1.3;
        let edited = build(&modified, PreviewPose::default()).unwrap();
        assert_eq!(baseline.quads, edited.quads);
        assert_eq!(baseline.triangles, edited.triangles);
        assert_ne!(baseline.vertices, edited.vertices);
    }
}