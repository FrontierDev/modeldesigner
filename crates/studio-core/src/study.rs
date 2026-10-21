//! Parametric torso/shoulder/upper-arm construction study.
//!
//! This is deliberately *not* a complete humanoid: it proves that the
//! generic topology engine can build a branching, closed quad surface with
//! anatomical controls and no imported base mesh or implicit field.

use crate::{pelvis, ArmPose, Joint, PreviewPose, Recipe, VertexInfluence};
use crate::mesh::{MeshBuilder, SurfaceMesh};
use glam::{Quat, Vec3};
use std::f32::consts::TAU;

const RING_SIDES: usize = 32;
const ARM_HOLE_START_ROW: usize = 6;
const ARM_HOLE_END_ROW: usize = 10;
const ARM_HOLE_COLUMNS: usize = 4;

// y, half-width, half-depth. These are surface *landmarks*, not volume primitives.
// Joint and patch positions are evaluated from the same normalized measurements.
const TORSO_SECTIONS: [(f32, f32, f32); 13] = [
    (0.78, 0.159, 0.122),
    (0.86, 0.151, 0.115),
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
    musculature: f32,
    body_mass: f32,
    neck_length: f32,
    neck_width: f32,
    abdomen_fullness: f32,
    back_curvature: f32,
    ribcage_height: f32,
    waist_depth: f32,
    lat_spread: f32,
    upper_back_fullness: f32,
    pectoral_fullness: f32,
    sternum_definition: f32,
    shoulder_forward: f32,
    deltoid_volume: f32,
    deltoid_roundness: f32,
    biceps_volume: f32,
    triceps_volume: f32,
    elbow_width: f32,
    arm_taper: f32,
    hip_width: f32,
    pelvis_tilt: f32,
    glute_volume: f32,
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
            musculature: recipe.musculature,
            body_mass: recipe.body_mass,
            neck_length: m.neck_length,
            neck_width: m.neck_width,
            abdomen_fullness: m.abdomen_fullness,
            back_curvature: m.back_curvature,
            ribcage_height: m.ribcage_height,
            waist_depth: m.waist_depth,
            lat_spread: m.lat_spread,
            upper_back_fullness: m.upper_back_fullness,
            pectoral_fullness: m.pectoral_fullness,
            sternum_definition: m.sternum_definition,
            shoulder_forward: m.shoulder_forward,
            deltoid_volume: m.deltoid_volume,
            deltoid_roundness: m.deltoid_roundness,
            biceps_volume: m.biceps_volume,
            triceps_volume: m.triceps_volume,
            elbow_width: m.elbow_width,
            arm_taper: m.arm_taper,
            hip_width: m.hip_width,
            pelvis_tilt: m.pelvis_tilt,
            glute_volume: m.glute_volume,
        }
    }
}
impl Measurements {
    fn y(self, y: f32) -> f32 {
        let stretched = 0.78 + (y - 0.78) * self.torso_length;
        let rib_region = smoothstep(0.98, 1.17, y)
            * (1.0 - smoothstep(1.41, 1.52, y));
        let neck_region = smoothstep(1.42, 1.53, y);
        (stretched
            + 0.072 * (self.ribcage_height - 1.0) * rib_region
            + 0.11 * (self.neck_length - 1.0) * neck_region)
            * self.height
    }

    fn spinal_shift(self, y: f32) -> f32 {
        let bend = smoothstep(0.90, 1.12, y)
            * (1.0 - smoothstep(1.37, 1.53, y));
        0.050 * (self.back_curvature - 0.5) * bend * self.height
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
        let anterior = cos.max(0.0);
        let posterior = (-cos).max(0.0);
        let lateral = sin.abs();
        let chest = smoothstep(1.07, 1.31, y);
        let waist = 1.0 - smoothstep(1.00, 1.21, y);
        let abdomen = smoothstep(0.87, 1.00, y)
            * (1.0 - smoothstep(1.17, 1.30, y));
        let back = smoothstep(1.10, 1.22, y)
            * (1.0 - smoothstep(1.39, 1.50, y));
        let shoulder_zone = smoothstep(1.25, 1.36, y)
            * (1.0 - smoothstep(1.47, 1.53, y));
        let neck = smoothstep(1.43, 1.53, y);
        let hip_region = 1.0 - smoothstep(0.78, 0.96, y);
        let upper_chest = smoothstep(1.20, 1.29, y)
            * (1.0 - smoothstep(1.38, 1.46, y));
        let medial = 1.0 - smoothstep(0.36, 0.94, lateral);

        let width = base_width
            * (1.0 + chest * (self.chest_width - 1.0))
            * (1.0 + chest * (self.shoulder_width - 1.0) * 0.7)
            * (1.0 + waist * (self.waist_width - 1.0))
            * (1.0 + (1.0 - waist) * (1.0 - chest)
                * (self.ribcage_taper - 1.0))
            * (1.0 + 0.18 * waist * (self.body_mass - 0.5))
            * (1.0 + 0.08 * chest * (self.musculature - 0.5))
            * (1.0 + posterior * lateral * back
                * (self.lat_spread - 1.0) * 0.90)
            * (1.0 + neck * (self.neck_width - 1.0))
            * (1.0 + hip_region * (self.hip_width - 1.0));
        let depth = base_depth
            * (1.0 + chest * (self.chest_depth - 1.0))
            * (1.0 + waist * (self.waist_depth - 1.0))
            * (1.0 + 0.23 * abdomen * (self.body_mass - 0.5))
            * (1.0 + 0.55 * neck * (self.neck_width - 1.0));

        // Anatomical contours are region-local and neutral at their defaults.
        let pectoral = upper_chest * anterior * medial
            * (0.012 + 0.050 * (self.pectoral_fullness - 0.5)
                + 0.018 * (self.musculature - 0.5));
        let sternum = upper_chest * anterior
            * (1.0 - smoothstep(0.0, 0.20, lateral))
            * (0.004 + 0.012 * (self.sternum_definition - 0.5));
        let abdomen_projection = 0.052 * (self.abdomen_fullness - 0.5)
            * abdomen * anterior;
        let back_projection = -0.052 * (self.upper_back_fullness - 0.5)
            * back * posterior;
        let glute_projection = -0.055 * (self.glute_volume - 0.5)
            * posterior * hip_region;
        let pelvic_inclination = 0.028 * (self.pelvis_tilt - 0.5)
            * cos * hip_region;
        let shoulder_translation = 0.090 * (self.shoulder_forward - 0.5)
            * lateral.powi(4) * shoulder_zone;
        Vec3::new(
            sin * width * self.height,
            self.y(y) + self.side_slope(lateral.powi(6), y)
                + pelvic_inclination * self.height,
            ((cos * depth + pectoral - sternum + abdomen_projection
                + back_projection + shoulder_translation + glute_projection) * self.height)
                + self.spinal_shift(y),
        )
    }

    fn shoulder_centre(self, side: f32) -> Vec3 {
        Vec3::new(
            side * 0.240 * self.shoulder_width * self.height,
            self.y(1.356) + self.side_slope(1.0, 1.356),
            self.spinal_shift(1.356)
                + 0.090 * (self.shoulder_forward - 0.5) * self.height,
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

/// Rig-space rotation from neutral A-pose for the selected side.
fn arm_rotation(side: f32, pose: ArmPose) -> Quat {
    let raise = Quat::from_rotation_z(side * pose.raise_deg.to_radians());
    let forward = Quat::from_rotation_y(-side * pose.forward_deg.to_radians());
    forward * raise
}

/// Per-joint local rotations for the clavicle and humerus. World transforms,
/// inverse bind matrices and child propagation are evaluated by skinning.rs.
pub fn pose_rotations(pose: PreviewPose) -> [Quat; 14] {
    let mut rotations = [Quat::IDENTITY; 14];
    for (side, clavicle, shoulder, arm) in [
        (-1.0_f32, 4_usize, 5_usize, pose.left),
        ( 1.0_f32, 7_usize, 8_usize, pose.right),
    ] {
        let rotation = arm_rotation(side, arm);
        // Clavicular elevation originates close to the sternum. The remaining
        // humeral rotation is local to the acromial shoulder joint.
        rotations[clavicle] = Quat::IDENTITY.slerp(rotation, 0.18);
        rotations[shoulder] = Quat::IDENTITY.slerp(rotation, 0.82);
    }
    rotations
}

/// Skin influences are assigned by the topology authoring stage rather than
/// found by nearest-bone distances, particularly at the axillary junction.
/// Shoulder-girdle influence falls smoothly across *both* the torso's
/// angular columns and its vertical anatomical sections. Vertices on the
/// shared arm-hole boundary are moved with the adjoining shoulder patch.
fn torso_shoulder_weights(row: usize, column: usize, base: VertexInfluence) -> VertexInfluence {
    let y = TORSO_SECTIONS[row].0;
    let vertical = smoothstep(1.19, 1.30, y) * (1.0 - smoothstep(1.43, 1.53, y));
    let right_distance = (column as i32 - 8).abs() as f32;
    let left_distance = (column as i32 - 24).abs() as f32;
    let distance = right_distance.min(left_distance);
    let lateral = 1.0 - smoothstep(1.5, 5.5, distance);
    let strength = vertical * lateral;
    if strength <= 1.0e-5 || row <= 5 {
        return base;
    }
    let clavicle = if right_distance < left_distance { 7 } else { 4 };
    let shoulder = if right_distance < left_distance { 8 } else { 5 };
    let clavicle_weight = 0.21 * strength;
    let shoulder_weight = 0.06 * strength;
    influence(
        2, 1.0 - clavicle_weight - shoulder_weight,
        clavicle, clavicle_weight,
        shoulder, shoulder_weight,
    )
}

/// Axillary vertices need more chest attachment, while the superior deltoid
/// travels predominantly with the shoulder. Blend over each 16-vertex ring.
fn anatomical_ring_weights(
    band: usize,
    baseline: &VertexInfluence,
    up_component: f32,
    front_component: f32,
) -> VertexInfluence {
    if band >= 3 {
        return baseline.clone();
    }
    let inferior = (-up_component).clamp(0.0, 1.0);
    let superior = up_component.clamp(0.0, 1.0);
    let anterior = front_component.clamp(0.0, 1.0);
    let axilla = inferior * (0.5 + 0.5 * anterior);
    let attenuation = [0.65, 1.0, 0.35][band];
    let wanted = (0.070 * axilla - 0.095 * superior) * attenuation;
    let chest = baseline.weights[0];
    let arm = baseline.weights[2];
    let delta = wanted.clamp(-chest * 0.7, arm * 0.7);
    influence(
        baseline.joint_indices[0], chest + delta,
        baseline.joint_indices[1], baseline.weights[1],
        baseline.joint_indices[2], arm - delta,
    )
}

fn influence(a: u16, wa: f32, b: u16, wb: f32, c: u16, wc: f32) -> VertexInfluence {
    VertexInfluence {
        joint_indices: [a, b, c, 0],
        weights: [wa, wb, wc, 0.0],
    }
}

pub struct ShoulderRing {
    pub vertices: Vec<u32>,
    pub strength: f32,
    pub side: f32,
    pub axis: Vec3,
}

pub struct BuiltStudy {
    pub surface: SurfaceMesh,
    pub weights: Vec<VertexInfluence>,
    pub corrective_rings: Vec<ShoulderRing>,
}

/// Preserve shoulder loop volume after linear-blend skinning; the sewn torso
/// rim remains fixed. This is a bounded local corrective, not a collision solver.
pub fn preserve_shoulder_volume(
    study: &BuiltStudy,
    posed_vertices: &mut [Vec3],
    matrices: &[glam::Mat4],
    pose: PreviewPose,
) -> Result<(), String> {
    if posed_vertices.len() != study.surface.vertices.len() {
        return Err("Corrective surface length differs from neutral mesh".into());
    }
    for ring in &study.corrective_rings {
        if ring.strength == 0.0 { continue; }
        let arm = if ring.side < 0.0 { pose.left } else { pose.right };
        let intensity = (arm.raise_deg / 70.0)
            .max(arm.forward_deg / 55.0).clamp(0.0, 1.0);
        if intensity == 0.0 { continue; }

        let shoulder = if ring.side < 0.0 { 5 } else { 8 };
        let transform = matrices.get(shoulder)
            .ok_or("Missing shoulder skinning transform")?;
        let posed_axis = transform.transform_vector3(ring.axis).normalize();
        if !posed_axis.is_finite() {
            return Err("Non-finite shoulder axis in corrective".into());
        }

        let count = ring.vertices.len() as f32;
        let neutral_centre = ring.vertices.iter()
            .map(|&id| study.surface.vertices[id as usize]).sum::<Vec3>() / count;
        let posed_centre = ring.vertices.iter()
            .map(|&id| posed_vertices[id as usize]).sum::<Vec3>() / count;
        for &vertex in &ring.vertices {
            let index = vertex as usize;
            let neutral_offset = study.surface.vertices[index] - neutral_centre;
            let neutral_radial = neutral_offset
                - ring.axis * neutral_offset.dot(ring.axis);
            let rest_radius = neutral_radial.length();
            let offset = posed_vertices[index] - posed_centre;
            let radial = offset - posed_axis * offset.dot(posed_axis);
            let radius = radial.length();
            if radius < 1e-6 || !radius.is_finite() {
                return Err("Collapsed shoulder loop during pose deformation".into());
            }
            // Only repair contraction; never enlarge above the neutral radius.
            let deficit = (rest_radius * 0.96 - radius).max(0.0);
            let shift = (deficit * ring.strength * intensity).min(0.010);
            posed_vertices[index] += radial / radius * shift;
        }
    }
    Ok(())
}

/// Round the four logical corners of the torso opening before stitching its
/// existing boundary vertices to the deltoid loops. Keeping the X coordinates
/// on the original torso contour avoids gaps between the chest faces and rim.
fn soften_socket_boundary(builder: &mut MeshBuilder, rim: &[u32]) {
    let points: Vec<Vec3> = rim.iter().map(|&id| builder.vertices[id as usize]).collect();
    let n = points.len();
    for (i, &id) in rim.iter().enumerate() {
        let adjacent = (points[(i + n - 1) % n] + points[(i + 1) % n]) * 0.5;
        let mut position = points[i];
        position.y = position.y * 0.70 + adjacent.y * 0.30;
        position.z = position.z * 0.70 + adjacent.z * 0.30;
        builder.vertices[id as usize] = position;
    }
}

/// Shoulder rings must advance *outward* at every matching vertex. Previously
/// the innermost ellipse points extended as much as 38 mm inside the torso-hole
/// rim, folding the intervening quads back on themselves. This constraint is
/// part of the procedural surface construction, not a post-pose skinning fix.
fn advance_socket_loop(
    builder: &mut MeshBuilder,
    from: &[u32],
    to: &[u32],
    side: f32,
    minimum_step: f32,
) -> Result<(), String> {
    if from.len() != to.len() || from.len() < 3 {
        return Err("Shoulder loops require equal-sized boundaries".into());
    }
    for (&source, &target) in from.iter().zip(to) {
        let old = builder.vertices[source as usize];
        let point = &mut builder.vertices[target as usize];
        point.x = side * (side * point.x).max(side * old.x + minimum_step);
    }
    Ok(())
}

/// Upper socket surface fairing: the previous shoulder socket and loft had
/// valid connectivity but a visible C0 crease across the superior seam.
/// Resolve the angular normal discontinuity in a narrow upper-shoulder band;
/// do not touch the sternum, lower axilla, or the rest of the torso.
/// Stable anatomical upper-socket mask evaluated against reference positions.
fn upper_shoulder_mask(vertices: &[Vec3], m: Measurements) -> Vec<f32> {
    let mut weights = Vec::with_capacity(vertices.len());
    for point in vertices {
        let width_scale = m.shoulder_width * m.chest_width;
        let lateral = point.x.abs() / (m.height * width_scale);
        let y = 0.78 + (point.y / m.height - 0.78) / m.torso_length;
        let outer = ((lateral - 0.105) / 0.040).clamp(0.0, 1.0)
            * ((0.38 - lateral) / 0.040).clamp(0.0, 1.0);
        let upper = ((y - 1.275) / 0.080).clamp(0.0, 1.0)
            * ((1.52 - y) / 0.060).clamp(0.0, 1.0);
        weights.push(outer * upper);
    }
    weights
}

fn fair_upper_shoulder(mesh: &mut MeshBuilder, m: Measurements) -> Result<(), String> {
    let mask = upper_shoulder_mask(&mesh.vertices, m);
    mesh.relax_masked(&mask, 4, 0.20, 0.016 * m.height)
}

/// Resolve shoulder creasing after bone deformation without touching the
/// neutral reference or the lower axillary transition.
pub fn fair_posed_upper_shoulder(
    surface: &mut SurfaceMesh, neutral: &[Vec3], recipe: &Recipe, pose: PreviewPose
) -> Result<(), String> {
    if neutral.len() != surface.vertices.len() {
        return Err("Posed and neutral vertex counts differ".into());
    }
    let m = Measurements::from(recipe);
    let mut mask = upper_shoulder_mask(neutral, m);
    // Never smooth the unposed shoulder: the opposite limb and chest must
    // remain bitwise unchanged during independent preview poses.
    for (index, vertex) in neutral.iter().enumerate() {
        let arm = if vertex.x < 0.0 { pose.left } else { pose.right };
        if arm == ArmPose::default() {
            mask[index] = 0.0;
        }
    }
    surface.relax_masked(&mask, 4, 0.20, 0.010 * m.height)
}

fn arm_ring(
    builder: &mut MeshBuilder,
    hole: &[u32],
    centre: Vec3,
    axis: Vec3,
    radius_y: f32,
    radius_z: f32,
    m: Measurements,
    band: usize,
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
        let upper = angle.cos().max(0.0);
        let anterior = angle.sin().max(0.0);
        let posterior = (-angle.sin()).max(0.0);
        let cap = [0.45, 1.0, 0.60, 0.12, 0.0][band];
        let muscle_region = [0.0, 0.30, 1.0, 1.0, 0.18][band];
        let taper = [0.12, 0.20, 0.45, 0.75, 1.0][band];

        let volume = 1.0
            + cap * (0.46 * (m.deltoid_volume - 0.5)
                + 0.15 * (m.musculature - 0.5))
            + 0.16 * muscle_region * (m.body_mass - 0.5)
            + (m.arm_taper - 1.0) * taper * 0.45;
        let elbow_factor = 1.0
            + (m.elbow_width - 1.0) * [0.0, 0.0, 0.0, 0.25, 1.0][band];
        let deltoid_top = 1.0
            + 0.34 * (m.deltoid_roundness - 0.5) * cap * upper;
        let muscle_front = 1.0 + 0.43
            * (m.biceps_volume - 0.5) * muscle_region * anterior;
        let muscle_back = 1.0 + 0.43
            * (m.triceps_volume - 0.5) * muscle_region * posterior;
        let width_y = radius_y * volume * elbow_factor * deltoid_top;
        let width_z = radius_z * volume * elbow_factor
            * muscle_front * muscle_back;
        centre + up * (angle.cos() * width_y)
            + front * (angle.sin() * width_z)
    }))
}

pub fn build(recipe: &Recipe) -> Result<BuiltStudy, String> {
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
    // Reuse the former lower cap centre as an actual crotch bridge vertex.
    // This preserves all previously authored shoulder patch vertex indices.
    let crotch_centre = mesh.vertex(Vec3::new(0.0, m.y(0.665), 0.0));
    mesh.cap(&rings[rings.len() - 1],
        Vec3::new(0.0, m.y(TORSO_SECTIONS[TORSO_SECTIONS.len() - 1].0), 0.0), false)?;

    // Tag each topology ring with authored joint influences.
    let torso_count = mesh.vertices.len();
    let mut weight_regions: Vec<(u32, VertexInfluence)> = Vec::new();
    let mut correction_regions = Vec::new();
    for (side, centre_column, clavicle, shoulder_joint) in [
        ( 1.0_f32, 8_usize, 7_u16, 8_u16),
        (-1.0_f32, 24_usize, 4_u16, 5_u16),
    ] {
        let hole = boundary(&rings, centre_column);
        soften_socket_boundary(&mut mesh, &hole);
        let hole_centre = hole.iter().map(|&id| mesh.vertices[id as usize]).sum::<Vec3>()
            / hole.len() as f32;
        let shoulder = m.shoulder_centre(side);
        let elbow = m.elbow(side);
        let axis = (elbow - shoulder).normalize();

        let transition_centre = hole_centre.lerp(shoulder, 0.44);
        let transition = arm_ring(&mut mesh, &hole, transition_centre, axis,
            0.090 * m.upper_arm_girth * m.height,
            0.078 * m.upper_arm_girth * m.height, m, 0);
        advance_socket_loop(&mut mesh, &hole, &transition, side, 0.028 * m.height)?;
        mesh.stitch(&hole, &transition)?;

        let deltoid_centre = shoulder;
        let deltoid = arm_ring(&mut mesh, &hole, deltoid_centre, axis,
            0.079 * m.upper_arm_girth * m.height,
            0.072 * m.upper_arm_girth * m.height, m, 1);
        advance_socket_loop(&mut mesh, &transition, &deltoid, side, 0.016 * m.height)?;
        mesh.stitch(&transition, &deltoid)?;

        let insertion_centre = shoulder.lerp(elbow, 0.24);
        let insertion = arm_ring(&mut mesh, &hole, insertion_centre, axis,
            0.070 * m.upper_arm_girth * m.height,
            0.063 * m.upper_arm_girth * m.height, m, 2);
        advance_socket_loop(&mut mesh, &deltoid, &insertion, side, 0.008 * m.height)?;
        mesh.stitch(&deltoid, &insertion)?;

        let mid = arm_ring(&mut mesh, &hole, shoulder.lerp(elbow, 0.55), axis,
            0.062 * m.upper_arm_girth * m.height,
            0.056 * m.upper_arm_girth * m.height, m, 3);
        mesh.stitch(&insertion, &mid)?;

        let elbow_loop = arm_ring(&mut mesh, &hole, elbow, axis,
            0.052 * m.upper_arm_girth * m.height,
            0.050 * m.upper_arm_girth * m.height, m, 4);
        mesh.stitch(&mid, &elbow_loop)?;

        let cap_index = mesh.vertices.len() as u32;
        mesh.cap(&elbow_loop, elbow + axis * 0.004 * m.height, false)?;

        let up = Vec3::new(-axis.y * axis.x.signum(), axis.x.abs(), 0.0).normalize();
        for (band, (indices, centre, base_weights, strength, radius_y, radius_z)) in [
            (transition, transition_centre, influence(2, 0.78, clavicle, 0.17, shoulder_joint, 0.05),
                0.75, 0.090, 0.078),
            (deltoid, deltoid_centre, influence(2, 0.28, clavicle, 0.36, shoulder_joint, 0.36),
                1.0, 0.079, 0.072),
            (insertion, insertion_centre, influence(2, 0.0, clavicle, 0.12, shoulder_joint, 0.88),
                0.5, 0.070, 0.063),
            (mid, shoulder.lerp(elbow, 0.55),
                influence(shoulder_joint, 1.0, 0, 0.0, 0, 0.0), 0.0, 0.062, 0.056),
            (elbow_loop, elbow,
                influence(shoulder_joint, 1.0, 0, 0.0, 0, 0.0), 0.0, 0.052, 0.050),
        ].into_iter().enumerate() {
            for &id in &indices {
                let offset = mesh.vertices[id as usize] - centre;
                let top = (offset.dot(up) / (radius_y * m.upper_arm_girth * m.height))
                    .clamp(-1.0, 1.0);
                let front = (offset.z / (radius_z * m.upper_arm_girth * m.height))
                    .clamp(-1.0, 1.0);
                let weights = anatomical_ring_weights(band, &base_weights, top, front);
                weight_regions.push((id, weights));
            }
            correction_regions.push(ShoulderRing {
                vertices: indices, strength, side, axis,
            });
        }
        weight_regions.push((cap_index, influence(shoulder_joint, 1.0, 0, 0.0, 0, 0.0)));
    }
    let lower_body = pelvis::append(&mut mesh, &rings[0], crotch_centre, recipe)?;
    if lower_body.leg_rings.iter().any(|rings| rings.len() != 4) {
        return Err("Upper-leg construction did not produce four anatomical rings".into());
    }
    fair_upper_shoulder(&mut mesh, m)?;
    let surface = mesh.build()?;
    let mut weights = vec![influence(2, 1.0, 0, 0.0, 0, 0.0); surface.vertices.len()];
    // Torso cross-sections use non-rotating pelvis/spine/chest bind influences;
    // shoulder-hole edges stay fixed on the chest during shoulder motion.
    for (row, ring) in rings.iter().enumerate() {
        let torso_weights = if row <= 2 {
            influence(0, 0.70, 1, 0.30, 2, 0.0)
        } else if row <= 5 {
            influence(0, 0.05, 1, 0.55, 2, 0.40)
        } else {
            influence(2, 1.0, 0, 0.0, 0, 0.0)
        };
        for (column, &id) in ring.iter().enumerate() {
            weights[id as usize] =
                torso_shoulder_weights(row, column, torso_weights.clone());
        }
    }
    // The torso end caps follow their nearest structural joints.
    for index in RING_SIDES * TORSO_SECTIONS.len()..torso_count {
        weights[index] = if index == RING_SIDES * TORSO_SECTIONS.len() {
            influence(0, 1.0, 0, 0.0, 0, 0.0)
        } else {
            influence(3, 1.0, 0, 0.0, 0, 0.0)
        };
    }
    for (id, region_weights) in weight_regions {
        weights[id as usize] = region_weights;
    }
    for (id, region_weights) in lower_body.influences {
        weights[id as usize] = region_weights;
    }
    Ok(BuiltStudy { surface, weights, corrective_rings: correction_regions })

}

fn append_joint(joints: &mut Vec<Joint>, name: &'static str, parent: Option<usize>, point: Vec3) {
    joints.push(Joint { name, parent, position: point.to_array() });
}

pub fn skeleton(recipe: &Recipe) -> Vec<Joint> {
    let m = Measurements::from(recipe);
    let mut joints = Vec::<Joint>::new();
    append_joint(&mut joints, "pelvis", None, Vec3::new(0.0, m.y(0.84), 0.0));
    append_joint(&mut joints, "spine", Some(0), Vec3::new(0.0, m.y(1.12), m.spinal_shift(1.12)));
    append_joint(&mut joints, "chest", Some(1), Vec3::new(0.0, m.y(1.37), m.spinal_shift(1.37)));
    append_joint(&mut joints, "neck", Some(2), Vec3::new(0.0, m.y(1.52), m.spinal_shift(1.52)));
    for side in [-1.0_f32, 1.0] {
        let shoulder = m.shoulder_centre(side);
        let clavicle_parent = 2;
        append_joint(
            &mut joints,
            if side < 0.0 { "left_clavicle" } else { "right_clavicle" },
            Some(clavicle_parent),
            // The sternoclavicular end is the pivot; the lateral shoulder
            // is its child, rather than rotating the clavicle at the acromion.
            Vec3::new(
                side * 0.065 * m.height, m.y(1.435),
                0.040 * m.height + m.spinal_shift(1.435)
                    + 0.035 * (m.shoulder_forward - 0.5) * m.height,
            ),
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
    // Upper-leg chains follow the 10 original upper-body joints so previously
    // authored shoulder skin indices and independent arm controls stay stable.
    for side in [-1.0_f32, 1.0] {
        let (hip, knee) = pelvis::joint_positions(recipe, side);
        append_joint(&mut joints,
            if side < 0.0 { "left_hip" } else { "right_hip" }, Some(0), hip);
        let parent = joints.len() - 1;
        append_joint(&mut joints,
            if side < 0.0 { "left_knee" } else { "right_knee" },
            Some(parent), knee);
    }
    joints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_study_is_closed_and_authors_per_ring_skin_weights() {
        let result = build(&Recipe::default()).unwrap();
        assert_eq!(result.surface.vertices.len(), 724);
        assert_eq!(result.surface.quads.len(), 656);
        assert_eq!(result.surface.triangles.len(), 1400);
        assert_eq!(result.weights.len(), result.surface.vertices.len());
        assert_eq!(result.corrective_rings.len(), 10);
        assert_eq!(skeleton(&Recipe::default()).len(), 14);
        for influence in &result.weights {
            let total: f32 = influence.weights.iter().sum();
            assert!((total - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn torso_surface_controls_do_not_shift_the_shoulder_joint_unnecessarily() {
        let neutral = Measurements::from(&Recipe::default());
        let mut recipe = Recipe::default();
        recipe.morphology.abdomen_fullness = 1.0;
        recipe.morphology.upper_back_fullness = 0.8;
        let modified = Measurements::from(&recipe);
        assert_eq!(neutral.shoulder_centre(1.0), modified.shoulder_centre(1.0));
        assert_ne!(neutral.torso_point(3, 0), modified.torso_point(3, 0));
        assert_ne!(neutral.torso_point(8, 16), modified.torso_point(8, 16));
    }

    #[test]
    fn supported_parameters_preserve_topology() {
        let baseline = build(&Recipe::default()).unwrap();
        let mut modified = Recipe::default();
        modified.shoulder_width = 0.60;
        modified.morphology.chest_width = 1.2;
        modified.morphology.upper_arm_girth = 1.3;
        let edited = build(&modified).unwrap();
        assert_eq!(baseline.surface.quads, edited.surface.quads);
        assert_eq!(baseline.surface.triangles, edited.surface.triangles);
        assert_ne!(baseline.surface.vertices, edited.surface.vertices);
    }

    #[test]
    fn shoulder_socket_loops_never_fold_back_into_the_torso() {
        let result = build(&Recipe::default()).unwrap();
        // Per side: 16 vertices on the existing torso boundary followed by
        // five consecutively stitched 16-vertex loops.
        let mut builder = MeshBuilder::default();
        let m = Measurements::from(&Recipe::default());
        let mut torso = Vec::new();
        for row in 0..TORSO_SECTIONS.len() {
            torso.push(builder.loop_vertices(
                (0..RING_SIDES).map(|column| m.torso_point(row, column))
            ));
        }
        for (side, column, first) in [(1.0_f32, 8_usize, 418_usize), (-1.0, 24, 499)] {
            let rim = boundary(&torso, column);
            let mut previous = rim;
            for (stage, gap) in [0.028_f32, 0.016, 0.008].into_iter().enumerate() {
                let next: Vec<u32> = (first + stage * 16..first + (stage + 1) * 16)
                    .map(|id| id as u32).collect();
                for (&from, &to) in previous.iter().zip(&next) {
                    let advance = side * (
                        result.surface.vertices[to as usize].x
                            - result.surface.vertices[from as usize].x
                    );
                    assert!(
                        advance >= gap * 0.75,
                        "socket stage {stage} reversed its lateral direction: {advance}"
                    );
                }
                previous = next;
            }
        }
    }

    #[test]
    fn neutral_shoulder_has_no_back_facing_fold_at_socket_seam() {
        use std::collections::HashMap;
        let result = build(&Recipe::default()).unwrap();
        let mesh = result.surface;
        let mut edges = HashMap::<(u32, u32), Vec<Vec3>>::new();
        for &[a, b, c] in &mesh.triangles {
            let normal = (mesh.vertices[b as usize] - mesh.vertices[a as usize])
                .cross(mesh.vertices[c as usize] - mesh.vertices[a as usize])
                .normalize();
            for (from, to) in [(a, b), (b, c), (c, a)] {
                edges.entry((from.min(to), from.max(to)))
                    .or_default().push(normal);
            }
        }
        let mut inspected = 0;
        for ((a, b), normals) in edges {
            if normals.len() != 2 { continue; }
            let middle = (mesh.vertices[a as usize] + mesh.vertices[b as usize]) * 0.5;
            if middle.x.abs() <= 0.14 || middle.x.abs() >= 0.36
                || middle.y <= 1.20 || middle.y >= 1.50
            {
                continue;
            }
            inspected += 1;
            assert!(
                normals[0].dot(normals[1]) > -0.45,
                "folded shoulder geometry near {middle:?}"
            );
        }
        assert!(inspected > 40, "shoulder seam was not inspected");
    }

    #[test]
    fn humeral_head_is_seated_beneath_lateral_shoulder() {
        let m = Measurements::from(&Recipe::default());
        let half_chest = m.torso_point(9, 8).x.abs();
        let pivot = m.shoulder_centre(1.0);
        assert!(
            (pivot.x.abs() - half_chest) < 0.025,
            "humeral head is excessively lateral relative to chest contour"
        );
        assert!(pivot.x.abs() > half_chest);
    }

    #[test]
    fn upper_shoulder_has_no_abrupt_face_normal_crease() {
        use std::collections::HashMap;
        let mesh = build(&Recipe::default()).unwrap().surface;
        let mut neighbouring = HashMap::<(u32, u32), Vec<Vec3>>::new();
        for &[a, b, c] in &mesh.triangles {
            let n = (mesh.vertices[b as usize] - mesh.vertices[a as usize])
                .cross(mesh.vertices[c as usize] - mesh.vertices[a as usize])
                .normalize();
            for (u, v) in [(a, b), (b, c), (c, a)] {
                neighbouring.entry((u.min(v), u.max(v)))
                    .or_default().push(n);
            }
        }
        let mut checked = 0;
        for ((a, b), normals) in neighbouring {
            if normals.len() != 2 { continue; }
            let mid = (mesh.vertices[a as usize] + mesh.vertices[b as usize]) * 0.5;
            if !(0.15..0.38).contains(&mid.x.abs())
                || !(1.39..1.50).contains(&mid.y)
            {
                continue;
            }
            checked += 1;
            assert!(
                normals[0].dot(normals[1]) > 0.55,
                "visible upper-socket crease around {mid:?}"
            );
        }
        assert!(checked > 40, "did not inspect enough upper-shoulder edges");
    }

    #[test]
    fn clavicle_pivot_is_medial_to_shoulder() {
        let joints = skeleton(&Recipe::default());
        for (clavicle, shoulder) in [(4, 5), (7, 8)] {
            let medial = Vec3::from_array(joints[clavicle].position);
            let lateral = Vec3::from_array(joints[shoulder].position);
            assert!(medial.x.abs() < 0.45 * lateral.x.abs());
            assert!(medial.y > lateral.y);
        }
    }

    #[test]
    fn axillary_and_deltoid_weights_have_opposing_gradients() {
        let baseline = influence(2, 0.28, 4, 0.36, 5, 0.36);
        let upper = anatomical_ring_weights(1, &baseline, 1.0, 0.0);
        let axilla = anatomical_ring_weights(1, &baseline, -1.0, 1.0);
        assert!(upper.weights[2] > baseline.weights[2]);
        assert!(axilla.weights[0] > baseline.weights[0]);
        for weights in [upper, axilla] {
            assert!((weights.weights.iter().sum::<f32>() - 1.0).abs() < 1e-5);
            assert!(weights.weights.iter().all(|weight| *weight >= 0.0));
        }
        let baseline = influence(2, 1.0, 0, 0.0, 0, 0.0);
        let seam = torso_shoulder_weights(8, 8, baseline.clone());
        let centre_chest = torso_shoulder_weights(8, 16, baseline);
        assert!(seam.weights[1] > 0.0);
        assert!(seam.weights[2] > 0.0);
        assert_eq!(centre_chest.weights[0], 1.0);
    }

    #[test]
    fn left_and_right_pose_rotations_are_independent() {
        let pose = PreviewPose {
            left: ArmPose { raise_deg: 60.0, forward_deg: 0.0 },
            right: ArmPose::default(),
        };
        let rotations = pose_rotations(pose);
        assert_ne!(rotations[5], Quat::IDENTITY);
        assert_eq!(rotations[8], Quat::IDENTITY);
        assert_eq!(rotations[7], Quat::IDENTITY);
    }
}
