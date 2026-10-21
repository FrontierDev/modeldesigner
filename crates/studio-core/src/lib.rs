//! Reproducible, topology-preserving humanoid authoring prototype.
//! The canonical triangulated surface is a reference cage, not final retopology.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

pub const SCHEMA_VERSION: u32 = 1;
pub const GENERATOR_VERSION: u32 = 1;
pub const REFERENCE_HEIGHT: f32 = 1.75;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema_version: u32,
    pub generator_version: u32,
    pub name: String,
    /// Total standing height, in metres.
    pub height: f32,
    /// Distance between shoulder joint centres, in metres at the reference height.
    pub shoulder_width: f32,
    /// Relative to the reference arm length (1.0 = neutral).
    pub arm_length: f32,
    /// Relative to the reference leg length (1.0 = neutral).
    pub leg_length: f32,
    /// 0.0–1.0 stylistic muscle fullness (0.5 = neutral).
    pub musculature: f32,
    /// 0.0–1.0 stylistic body fullness (0.5 = neutral).
    pub body_mass: f32,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            generator_version: GENERATOR_VERSION,
            name: "Humanoid 01".into(),
            height: REFERENCE_HEIGHT,
            shoulder_width: 0.46,
            arm_length: 1.0,
            leg_length: 1.0,
            musculature: 0.5,
            body_mass: 0.5,
        }
    }
}


/** A stable identifier for a character-build slider. */
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildParameter {
    Height,
    ShoulderWidth,
    ArmLength,
    LegLength,
    Musculature,
    BodyMass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildSection {
    Proportions,
    Physique,
}

/// One source of truth for authoring and future in-game build sliders.
/// Values stored in recipes remain in metres or unitless ratios; only UI display is scaled.
#[derive(Clone, Copy, Debug)]
pub struct BuildControl {
    pub parameter: BuildParameter,
    pub section: BuildSection,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub display_scale: f32,
    pub suffix: &'static str,
}

impl BuildControl {
    pub fn get(self, recipe: &Recipe) -> f32 {
        match self.parameter {
            BuildParameter::Height => recipe.height,
            BuildParameter::ShoulderWidth => recipe.shoulder_width,
            BuildParameter::ArmLength => recipe.arm_length,
            BuildParameter::LegLength => recipe.leg_length,
            BuildParameter::Musculature => recipe.musculature,
            BuildParameter::BodyMass => recipe.body_mass,
        }
    }

    pub fn set(self, recipe: &mut Recipe, value: f32) {
        match self.parameter {
            BuildParameter::Height => recipe.height = value,
            BuildParameter::ShoulderWidth => recipe.shoulder_width = value,
            BuildParameter::ArmLength => recipe.arm_length = value,
            BuildParameter::LegLength => recipe.leg_length = value,
            BuildParameter::Musculature => recipe.musculature = value,
            BuildParameter::BodyMass => recipe.body_mass = value,
        }
    }
}

pub const BUILD_CONTROLS: [BuildControl; 6] = [
    BuildControl { parameter: BuildParameter::Height, section: BuildSection::Proportions,
        label: "Height", min: 1.50, max: 2.10, display_scale: 100.0, suffix: " cm" },
    BuildControl { parameter: BuildParameter::ShoulderWidth, section: BuildSection::Proportions,
        label: "Shoulder width", min: 0.34, max: 0.62, display_scale: 100.0, suffix: " cm" },
    BuildControl { parameter: BuildParameter::ArmLength, section: BuildSection::Proportions,
        label: "Arm length", min: 0.82, max: 1.18, display_scale: 100.0, suffix: "%" },
    BuildControl { parameter: BuildParameter::LegLength, section: BuildSection::Proportions,
        label: "Leg length", min: 0.82, max: 1.18, display_scale: 100.0, suffix: "%" },
    BuildControl { parameter: BuildParameter::Musculature, section: BuildSection::Physique,
        label: "Musculature", min: 0.0, max: 1.0, display_scale: 100.0, suffix: "%" },
    BuildControl { parameter: BuildParameter::BodyMass, section: BuildSection::Physique,
        label: "Body mass", min: 0.0, max: 1.0, display_scale: 100.0, suffix: "%" },
];

impl Recipe {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION || self.generator_version != GENERATOR_VERSION {
            return Err("Unsupported recipe schema or generator version".into());
        }
        if self.name.trim().is_empty() || self.name.len() > 120 {
            return Err("Name must contain 1–120 characters".into());
        }
        for control in BUILD_CONTROLS {
            let value = control.get(self);
            if !value.is_finite() || !(control.min..=control.max).contains(&value) {
                return Err(format!(
                    "{} must be in the range {}–{}",
                    control.label, control.min, control.max
                ));
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let recipe: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        recipe.validate()?;
        Ok(recipe)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Joint {
    pub name: &'static str,
    pub parent: Option<usize>,
    pub position: [f32; 3],
}

#[derive(Clone, Debug, Serialize)]
pub struct VertexInfluence {
    pub joint_indices: [u16; 4],
    pub weights: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct Model {
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub joints: Vec<Joint>,
    pub influences: Vec<VertexInfluence>,
}

#[derive(Serialize)]
struct RigExport<'a> {
    schema_version: u32,
    generator_version: u32,
    vertex_count: usize,
    joint_count: usize,
    joints: &'a [Joint],
    influences: &'a [VertexInfluence],
}

impl Model {
    pub fn to_obj(&self) -> String {
        let mut obj = String::from("# Model Designer | metres | Y-up | canonical vertex order\no humanoid\n");
        for [x, y, z] in &self.vertices {
            writeln!(&mut obj, "v {x:.6} {y:.6} {z:.6}").expect("writing String");
        }
        for [a, b, c] in &self.triangles {
            writeln!(&mut obj, "f {} {} {}", a + 1, b + 1, c + 1).expect("writing String");
        }
        obj
    }

    /// Companion metadata: OBJ itself cannot encode skeletal skinning.
    pub fn to_rig_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&RigExport {
            schema_version: SCHEMA_VERSION,
            generator_version: GENERATOR_VERSION,
            vertex_count: self.vertices.len(),
            joint_count: self.joints.len(),
            joints: &self.joints,
            influences: &self.influences,
        })
        .map_err(|e| e.to_string())
    }
}

struct Canonical {
    vertices: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    influences: Vec<VertexInfluence>,
}

static CANONICAL: OnceLock<Canonical> = OnceLock::new();

/// Every recipe uses the identical canonical mesh and vertex indexing.
pub fn generate(recipe: &Recipe) -> Result<Model, String> {
    recipe.validate()?;
    let reference = CANONICAL.get_or_init(build_reference);
    let joints = skeleton(recipe);
    let vertices = reference
        .vertices
        .iter()
        .map(|p| deform(*p, recipe).to_array())
        .collect();
    Ok(Model {
        vertices,
        triangles: reference.triangles.clone(),
        joints,
        influences: reference.influences.clone(),
    })
}

fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn joint_positions(height: f32, shoulder_width: f32, arm: f32, leg: f32) -> [Vec3; 19] {
    let scale = height / REFERENCE_HEIGHT;
    let hip_y = scale * (0.09 + 0.80 * leg);
    let shoulder_y = scale * 1.42;
    let chest_y = hip_y + (shoulder_y - hip_y) * 0.77;
    let half_shoulder = shoulder_width * scale * 0.5;
    let side = |s: f32, x: f32, y: f32| Vec3::new(s * x, y, 0.0);
    [
        Vec3::new(0.0, hip_y, 0.0), // hips
        Vec3::new(0.0, hip_y + (shoulder_y - hip_y) * 0.38, 0.0),
        Vec3::new(0.0, chest_y, 0.0),
        Vec3::new(0.0, scale * 1.53, 0.0),
        Vec3::new(0.0, scale * 1.65, 0.0),
        side(-1.0, half_shoulder, shoulder_y),
        side(-1.0, half_shoulder + 0.18 * scale * arm, shoulder_y - 0.23 * scale * arm),
        side(-1.0, half_shoulder + 0.35 * scale * arm, shoulder_y - 0.38 * scale * arm),
        side(-1.0, half_shoulder + 0.42 * scale * arm, shoulder_y - 0.44 * scale * arm),
        side(1.0, half_shoulder, shoulder_y),
        side(1.0, half_shoulder + 0.18 * scale * arm, shoulder_y - 0.23 * scale * arm),
        side(1.0, half_shoulder + 0.35 * scale * arm, shoulder_y - 0.38 * scale * arm),
        side(1.0, half_shoulder + 0.42 * scale * arm, shoulder_y - 0.44 * scale * arm),
        side(-1.0, 0.12 * scale, hip_y),
        side(-1.0, 0.13 * scale, hip_y - 0.42 * scale * leg),
        side(-1.0, 0.13 * scale, 0.09 * scale),
        side(1.0, 0.12 * scale, hip_y),
        side(1.0, 0.13 * scale, hip_y - 0.42 * scale * leg),
        side(1.0, 0.13 * scale, 0.09 * scale),
    ]
}

pub fn skeleton(recipe: &Recipe) -> Vec<Joint> {
    const NAMES: [&str; 19] = [
        "hips", "spine", "chest", "neck", "head",
        "left_shoulder", "left_elbow", "left_wrist", "left_hand",
        "right_shoulder", "right_elbow", "right_wrist", "right_hand",
        "left_hip", "left_knee", "left_ankle", "right_hip", "right_knee", "right_ankle",
    ];
    const PARENTS: [Option<usize>; 19] = [
        None, Some(0), Some(1), Some(2), Some(3),
        Some(2), Some(5), Some(6), Some(7),
        Some(2), Some(9), Some(10), Some(11),
        Some(0), Some(13), Some(14), Some(0), Some(16), Some(17),
    ];
    let coords = joint_positions(recipe.height, recipe.shoulder_width, recipe.arm_length, recipe.leg_length);
    NAMES.iter().enumerate().map(|(i, name)| Joint {
        name: *name,
        parent: PARENTS[i],
        position: coords[i].to_array(),
    }).collect()
}

/// Piecewise vertical landmark remapping preserves overall height while changing leg length.
/// Limb and volume warps are smooth so the *indices* and the connected reference surface remain stable.
fn deform(p: Vec3, r: &Recipe) -> Vec3 {
    let scale = r.height / REFERENCE_HEIGHT;
    let target_hip = scale * (0.09 + 0.80 * r.leg_length);
    let target_shoulder = scale * 1.42;
    let y = if p.y <= 0.89 {
        p.y / 0.89 * target_hip
    } else if p.y <= 1.42 {
        target_hip + (p.y - 0.89) / (1.42 - 0.89) * (target_shoulder - target_hip)
    } else {
        target_shoulder + (p.y - 1.42) / (REFERENCE_HEIGHT - 1.42) * (r.height - target_shoulder)
    };
    let arm_region = smoothstep(0.19, 0.37, p.x.abs()) * (1.0 - smoothstep(1.48, 1.60, p.y));
    let shoulders = smoothstep(1.04, 1.35, p.y) * (1.0 - smoothstep(1.55, 1.70, p.y));
    let shoulder_scale = 1.0 + (r.shoulder_width / 0.46 - 1.0) * shoulders;
    let lateral = p.x * shoulder_scale;
    let arm_adjust = p.x.signum() * (p.x.abs() - 0.21).max(0.0) * (r.arm_length - 1.0) * arm_region;
    let mass_region = smoothstep(0.78, 1.00, p.y) * (1.0 - smoothstep(1.37, 1.57, p.y));
    let mass_scale = 1.0 + (r.body_mass - 0.5) * 0.37 * mass_region;
    let muscle_region = (smoothstep(0.23, 0.38, p.x.abs()) * (1.0 - smoothstep(1.38, 1.56, p.y)))
        .max(smoothstep(0.08, 0.18, p.x.abs()) * (1.0 - smoothstep(0.70, 0.96, p.y)));
    let muscle_scale = 1.0 + (r.musculature - 0.5) * 0.16 * muscle_region;
    let arm_drop = (target_shoulder - y) * (r.arm_length - 1.0) * arm_region * 0.75;
    Vec3::new((lateral + arm_adjust) * mass_scale * muscle_scale * scale,
              y - arm_drop,
              p.z * (1.0 + (r.body_mass - 0.5) * 0.48 * mass_region) * muscle_scale * scale)
}

fn ellipsoid(p: Vec3, centre: Vec3, radii: Vec3) -> f32 {
    (((p - centre) / radii).length() - 1.0) * radii.min_element()
}

fn tapered_capsule(p: Vec3, start: Vec3, end: Vec3, r0: f32, r1: f32) -> f32 {
    let delta = end - start;
    let t = ((p - start).dot(delta) / delta.length_squared()).clamp(0.0, 1.0);
    (p - (start + t * delta)).length() - r0 - (r1 - r0) * t
}

fn union_smooth(a: f32, b: f32, k: f32) -> f32 {
    let t = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * t - k * t * (1.0 - t)
}

fn reference_field(p: Vec3) -> f32 {
    let mut field = ellipsoid(p, Vec3::new(0.0, 1.14, 0.0), Vec3::new(0.206, 0.335, 0.136));
    field = union_smooth(field, ellipsoid(p, Vec3::new(0.0, 1.30, 0.0), Vec3::new(0.243, 0.183, 0.159)), 0.075);
    field = union_smooth(field, ellipsoid(p, Vec3::new(0.0, 0.94, 0.0), Vec3::new(0.209, 0.131, 0.151)), 0.065);
    field = union_smooth(field, tapered_capsule(p, Vec3::new(0.0, 1.40, 0.0), Vec3::new(0.0, 1.56, 0.0), 0.075, 0.066), 0.06);
    field = union_smooth(field, ellipsoid(p, Vec3::new(0.0, 1.635, 0.0), Vec3::new(0.105, 0.133, 0.105)), 0.033);
    for sign in [-1.0_f32, 1.0] {
        let shoulder = Vec3::new(sign * 0.23, 1.42, 0.0);
        let elbow = Vec3::new(sign * 0.41, 1.19, 0.0);
        let wrist = Vec3::new(sign * 0.58, 1.04, 0.0);
        let hand = Vec3::new(sign * 0.65, 0.98, 0.0);
        field = union_smooth(field, tapered_capsule(p, shoulder, elbow, 0.106, 0.074), 0.073);
        field = union_smooth(field, tapered_capsule(p, elbow, wrist, 0.074, 0.047), 0.030);
        field = union_smooth(field, tapered_capsule(p, wrist, hand, 0.050, 0.039), 0.017);
        let hip = Vec3::new(sign * 0.12, 0.90, 0.0);
        let knee = Vec3::new(sign * 0.13, 0.47, 0.0);
        let ankle = Vec3::new(sign * 0.13, 0.09, 0.0);
        field = union_smooth(field, tapered_capsule(p, hip, knee, 0.111, 0.072), 0.06);
        field = union_smooth(field, tapered_capsule(p, knee, ankle, 0.074, 0.047), 0.025);
        field = union_smooth(field, ellipsoid(p, Vec3::new(sign * 0.13, 0.075, 0.075), Vec3::new(0.073, 0.064, 0.155)), 0.034);
    }
    field
}

fn closest_influences(point: Vec3, joints: &[Joint]) -> VertexInfluence {
    let mut candidates = Vec::with_capacity(joints.len());
    for (i, joint) in joints.iter().enumerate() {
        let Some(parent) = joint.parent else { continue; };
        let a = Vec3::from_array(joints[parent].position);
        let b = Vec3::from_array(joint.position);
        let segment = b - a;
        let t = ((point - a).dot(segment) / segment.length_squared()).clamp(0.0, 1.0);
        let d = point.distance(a + t * segment);
        candidates.push((d, i as u16));
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut indices = [0; 4];
    let mut weights = [0.0; 4];
    for (slot, (distance, index)) in candidates.into_iter().take(4).enumerate() {
        indices[slot] = index;
        weights[slot] = 1.0 / (distance + 0.045).powi(4);
    }
    let total: f32 = weights.iter().sum();
    for weight in &mut weights { *weight /= total; }
    VertexInfluence { joint_indices: indices, weights }
}

fn build_reference() -> Canonical {
    const NX: usize = 44;
    const NY: usize = 82;
    const NZ: usize = 30;
    let start = Vec3::new(-0.78, -0.06, -0.27);
    let size = Vec3::new(1.56, 1.90, 0.55);
    let index = |x: usize, y: usize, z: usize| (z * NY + y) * NX + x;
    let mut coords = Vec::with_capacity(NX * NY * NZ);
    let mut scalar = Vec::with_capacity(NX * NY * NZ);
    for z in 0..NZ { for y in 0..NY { for x in 0..NX {
        let p = start + Vec3::new(size.x * x as f32 / (NX - 1) as f32,
                                  size.y * y as f32 / (NY - 1) as f32,
                                  size.z * z as f32 / (NZ - 1) as f32);
        coords.push(p);
        scalar.push(reference_field(p));
    } } }
    let mut vertices = Vec::<Vec3>::new();
    let mut triangles = Vec::<[u32; 3]>::new();
    let mut cache = HashMap::<(usize, usize), u32>::new();
    const TETS: [[usize; 4]; 6] = [
        [0, 5, 1, 6], [0, 1, 2, 6], [0, 2, 3, 6],
        [0, 3, 7, 6], [0, 7, 4, 6], [0, 4, 5, 6],
    ];
    for z in 0..NZ - 1 { for y in 0..NY - 1 { for x in 0..NX - 1 {
        let cube = [index(x,y,z), index(x+1,y,z), index(x+1,y+1,z), index(x,y+1,z),
                    index(x,y,z+1), index(x+1,y,z+1), index(x+1,y+1,z+1), index(x,y+1,z+1)];
        for tet in TETS {
            let corners = tet.map(|n| cube[n]);
            let inside: Vec<_> = corners.into_iter().filter(|&id| scalar[id] < 0.0).collect();
            if inside.is_empty() || inside.len() == 4 { continue; }
            let outside: Vec<_> = corners.into_iter().filter(|&id| scalar[id] >= 0.0).collect();
            let mut edge = |a: usize, b: usize| -> u32 {
                let key = (a.min(b), a.max(b));
                *cache.entry(key).or_insert_with(|| {
                    let t = scalar[a] / (scalar[a] - scalar[b]);
                    let p = coords[a].lerp(coords[b], t);
                    let id = vertices.len() as u32;
                    vertices.push(p);
                    id
                })
            };
            let faces: Vec<[u32; 3]> = match inside.len() {
                1 => vec![[edge(inside[0], outside[0]), edge(inside[0], outside[1]), edge(inside[0], outside[2])]],
                3 => vec![[edge(outside[0], inside[0]), edge(outside[0], inside[1]), edge(outside[0], inside[2])]],
                2 => {
                    let a = edge(inside[0], outside[0]);
                    let b = edge(inside[0], outside[1]);
                    let c = edge(inside[1], outside[0]);
                    let d = edge(inside[1], outside[1]);
                    vec![[a, b, c], [b, d, c]]
                }
                _ => unreachable!(),
            };
            let outward = outside.iter().map(|&id| coords[id]).sum::<Vec3>() / outside.len() as f32
                - inside.iter().map(|&id| coords[id]).sum::<Vec3>() / inside.len() as f32;
            for mut face in faces {
                let a = vertices[face[0] as usize];
                let b = vertices[face[1] as usize];
                let c = vertices[face[2] as usize];
                if (b - a).cross(c - a).dot(outward) < 0.0 { face.swap(1, 2); }
                triangles.push(face);
            }
        }
    } } }
    let reference_joints = skeleton(&Recipe::default());
    let influences = vertices.iter().map(|p| closest_influences(*p, &reference_joints)).collect();
    Canonical { vertices, triangles, influences }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn build_controls_match_recipe_validation_and_are_reversible() {
        let default = Recipe::default();
        assert_eq!(BUILD_CONTROLS.len(), 6);
        for (index, control) in BUILD_CONTROLS.iter().copied().enumerate() {
            assert!(control.min < control.max);
            assert!(control.display_scale > 0.0);
            assert!((control.min..=control.max).contains(&control.get(&default)));
            assert!(!control.label.is_empty());
            assert!(!control.suffix.is_empty());
            assert!(BUILD_CONTROLS[..index].iter().all(|other| other.parameter != control.parameter));

            let mut edited = default.clone();
            control.set(&mut edited, control.max);
            edited.validate().unwrap();
            assert_eq!(control.get(&edited), control.max);

            control.set(&mut edited, control.max + 1.0);
            assert!(edited.validate().is_err());
        }
    }

    #[test]
    fn generator_is_deterministic() {
        let recipe = Recipe::default();
        let a = generate(&recipe).unwrap();
        let b = generate(&recipe).unwrap();
        assert_eq!(a.vertices, b.vertices);
        assert_eq!(a.triangles, b.triangles);
        assert!(a.vertices.len() > 1000);
    }

    #[test]
    fn topology_unchanged_by_all_parameters() {
        let base = generate(&Recipe::default()).unwrap();
        let changed = generate(&Recipe { height: 2.07, shoulder_width: 0.58,
            arm_length: 1.16, leg_length: 0.87, musculature: 1.0,
            body_mass: 0.93, ..Recipe::default() }).unwrap();
        assert_eq!(base.triangles, changed.triangles);
        assert_eq!(base.vertices.len(), changed.vertices.len());
        assert_ne!(base.vertices, changed.vertices);
        assert_eq!(base.influences.len(), changed.influences.len());
    }

    #[test]
    fn valid_indices_and_normalized_weights() {
        let model = generate(&Recipe::default()).unwrap();
        assert_eq!(model.vertices.len(), model.influences.len());
        assert_eq!(model.joints.len(), 19);
        for triangle in &model.triangles {
            for i in triangle { assert!((*i as usize) < model.vertices.len()); }
        }
        for joint in &model.joints {
            if let Some(parent) = joint.parent {
                assert!(parent < model.joints.iter().position(|x| x.name == joint.name).unwrap());
            }
        }
        for influence in &model.influences {
            let sum: f32 = influence.weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5);
            for i in influence.joint_indices { assert!((i as usize) < model.joints.len()); }
        }
    }

    #[test]
    fn reference_surface_has_two_faces_per_edge() {
        let model = generate(&Recipe::default()).unwrap();
        let mut uses = std::collections::HashMap::<(u32, u32), usize>::new();
        for face in &model.triangles {
            for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                *uses.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        assert!(!uses.is_empty());
        assert!(uses.values().all(|count| *count == 2), "Expected a closed, manifold canonical surface");
    }

    #[test]
    fn validate_and_round_trip_recipe() {
        let recipe = Recipe::default();
        assert_eq!(Recipe::from_json(&recipe.to_json().unwrap()).unwrap(), recipe);
        let mut invalid = recipe.clone();
        invalid.arm_length = f32::NAN;
        assert!(invalid.validate().is_err());
        invalid = recipe.clone();
        invalid.generator_version = 2;
        assert!(invalid.validate().is_err());
        invalid = recipe.clone();
        invalid.body_mass = 1.5;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn exports_contain_geometry_and_rig() {
        let model = generate(&Recipe::default()).unwrap();
        let obj = model.to_obj();
        assert_eq!(obj.lines().filter(|x| x.starts_with("v ")).count(), model.vertices.len());
        let rig: serde_json::Value = serde_json::from_str(&model.to_rig_json().unwrap()).unwrap();
        assert_eq!(rig["joint_count"].as_u64(), Some(19));
        assert_eq!(rig["vertex_count"].as_u64(), Some(model.vertices.len() as u64));
    }
}