//! Model Designer: topology-first procedural modelling core.
//!
//! Generic quad meshing is separate from anatomy-specific construction.
//! The only active generator is a connected upper-body geometry study.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub mod controls;
pub mod mesh;
pub mod morphology;
mod study;

pub use controls::{BuildControl, BuildParameter, BuildSection, BUILD_CONTROLS};
pub use morphology::Morphology;

pub const SCHEMA_VERSION: u32 = 1;
pub const REFERENCE_HEIGHT: f32 = 1.75;

/// Non-persistent pose controls for testing shoulder topology in the editor.
/// These are not part of the character recipe or exported rig.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviewPose {
    pub arm_raise_deg: f32,
    pub arm_forward_deg: f32,
}
impl PreviewPose {
    pub fn validate(self) -> Result<(), String> {
        if !self.arm_raise_deg.is_finite()
            || !(0.0..=70.0).contains(&self.arm_raise_deg)
            || !self.arm_forward_deg.is_finite()
            || !(0.0..=55.0).contains(&self.arm_forward_deg)
        {
            return Err("Preview arm raise must be 0–70° and forward reach 0–55°".into());
        }
        Ok(())
    }

    pub fn is_neutral(self) -> bool {
        self.arm_raise_deg == 0.0 && self.arm_forward_deg == 0.0
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema_version: u32,
    pub name: String,
    pub height: f32,
    pub shoulder_width: f32,
    pub arm_length: f32,
    pub leg_length: f32,
    pub musculature: f32,
    pub body_mass: f32,
    #[serde(default)]
    pub morphology: Morphology,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: "Upper Body 01".to_owned(),
            height: REFERENCE_HEIGHT,
            shoulder_width: 0.46,
            arm_length: 1.0,
            leg_length: 1.0,
            musculature: 0.5,
            body_mass: 0.5,
            morphology: Morphology::default(),
        }
    }
}

impl Recipe {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("Unsupported recipe schema".to_owned());
        }
        if self.name.trim().is_empty() || self.name.len() > 120 {
            return Err("Name must contain 1–120 characters".to_owned());
        }
        for control in BUILD_CONTROLS {
            let value = control.get(self);
            if !value.is_finite() || !(control.min..=control.max).contains(&value) {
                return Err(format!("{} must be in the range {}–{}", control.label, control.min, control.max));
            }
            if !control.implemented() && (value - control.default).abs() > 1.0e-4 {
                return Err(format!("{} is not implemented in the upper-body study", control.label));
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|err| err.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let recipe: Self = serde_json::from_str(text).map_err(|err| err.to_string())?;
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
    pub normals: Vec<[f32; 3]>,
    pub quads: Vec<[u32; 4]>,
    /// Preview triangles include two triangles per quad plus study end caps.
    pub triangles: Vec<[u32; 3]>,
    pub caps: Vec<[u32; 3]>,
    pub joints: Vec<Joint>,
    pub influences: Vec<VertexInfluence>,
}

#[derive(Serialize)]
struct RigExport<'a> {
    schema_version: u32,
    vertex_count: usize,
    joint_count: usize,
    joints: &'a [Joint],
    influences: &'a [VertexInfluence],
}

impl Model {
    pub fn to_obj(&self) -> String {
        let mut obj = String::from("# Model Designer | metres | Y-up | parametric upper-body study\no upper_body\n");
        for [x, y, z] in &self.vertices {
            writeln!(&mut obj, "v {x:.6} {y:.6} {z:.6}").expect("writing String");
        }
        for [a, b, c, d] in &self.quads {
            writeln!(&mut obj, "f {} {} {} {}", a + 1, b + 1, c + 1, d + 1).expect("writing String");
        }
        for [a, b, c] in &self.caps {
            writeln!(&mut obj, "f {} {} {}", a + 1, b + 1, c + 1).expect("writing String");
        }
        obj
    }

    pub fn to_rig_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&RigExport {
            schema_version: SCHEMA_VERSION,
            vertex_count: self.vertices.len(),
            joint_count: self.joints.len(),
            joints: &self.joints,
            influences: &self.influences,
        })
        .map_err(|err| err.to_string())
    }
}

fn closest_influences(point: Vec3, joints: &[Joint]) -> Result<VertexInfluence, String> {
    let mut candidates = Vec::with_capacity(joints.len());
    for (i, joint) in joints.iter().enumerate() {
        let Some(parent) = joint.parent else { continue };
        let a = Vec3::from_array(joints[parent].position);
        let b = Vec3::from_array(joint.position);
        let segment = b - a;
        let len2 = segment.length_squared();
        if len2 < 1.0e-10 {
            return Err(format!("Zero-length bone: {}", joint.name));
        }
        let t = ((point - a).dot(segment) / len2).clamp(0.0, 1.0);
        candidates.push((point.distance(a + t * segment), i as u16));
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    if candidates.len() < 4 {
        return Err("Skeleton requires at least four non-root bones".to_owned());
    }
    let mut indices = [0u16; 4];
    let mut weights = [0.0_f32; 4];
    for (slot, (distance, joint)) in candidates.into_iter().take(4).enumerate() {
        indices[slot] = joint;
        weights[slot] = 1.0 / (distance + 0.045).powi(4);
    }
    let total: f32 = weights.iter().sum();
    for weight in &mut weights { *weight /= total; }
    Ok(VertexInfluence { joint_indices: indices, weights })
}

pub fn generate(recipe: &Recipe) -> Result<Model, String> {
    generate_posed(recipe, PreviewPose::default())
}

/// Evaluate a temporary pose for geometry inspection. All skin weights are
/// computed against the neutral reference, never a posed skeleton or surface.
pub fn generate_posed(recipe: &Recipe, pose: PreviewPose) -> Result<Model, String> {
    recipe.validate()?;
    pose.validate()?;
    let neutral_surface = study::build(recipe, PreviewPose::default())?;
    let rest_joints = study::skeleton(recipe, PreviewPose::default());
    let influences = neutral_surface.vertices.iter()
        .map(|&point| closest_influences(point, &rest_joints))
        .collect::<Result<Vec<_>, _>>()?;

    let surface = if pose.is_neutral() {
        neutral_surface
    } else {
        study::build(recipe, pose)?
    };
    let joints = if pose.is_neutral() {
        rest_joints
    } else {
        study::skeleton(recipe, pose)
    };
    let normals = surface.normals();
    Ok(Model {
        vertices: surface.vertices.iter().map(|p| p.to_array()).collect(),
        normals,
        quads: surface.quads,
        triangles: surface.triangles,
        caps: surface.caps,
        joints,
        influences,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_mesh_has_quads_and_a_consistent_rig() {
        let m = generate(&Recipe::default()).unwrap();
        assert_eq!(m.joints.len(), 10);
        assert!(m.quads.len() > 100);
        assert!(m.triangles.len() < 3_000);
        assert_eq!(m.vertices.len(), m.normals.len());
        assert_eq!(m.vertices.len(), m.influences.len());
        assert_eq!(m.to_obj().lines().filter(|s| s.starts_with("f ")).count(),
            m.quads.len() + m.caps.len());
        for influence in &m.influences {
            let sum: f32 = influence.weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4);
            assert!(influence.joint_indices.iter().all(|&j| (j as usize) < m.joints.len()));
        }
    }

    #[test]
    fn supported_controls_change_geometry_without_remeshing() {
        let baseline = generate(&Recipe::default()).unwrap();
        for control in BUILD_CONTROLS.into_iter().filter(|c| c.implemented()) {
            let mut recipe = Recipe::default();
            let candidate = if (control.default - control.min).abs() > 1e-5 {
                control.min
            } else { control.max };
            control.set(&mut recipe, candidate);
            let changed = generate(&recipe).unwrap();
            assert_ne!(changed.vertices, baseline.vertices, "{} is inert", control.id);
            assert_eq!(changed.quads, baseline.quads);
            assert_eq!(changed.triangles, baseline.triangles);
        }
    }

    #[test]
    fn preview_pose_changes_positions_without_changing_topology_or_weights() {
        let recipe = Recipe::default();
        let neutral = generate(&recipe).unwrap();
        for pose in [
            PreviewPose { arm_raise_deg: 45.0, arm_forward_deg: 0.0 },
            PreviewPose { arm_raise_deg: 0.0, arm_forward_deg: 40.0 },
            PreviewPose { arm_raise_deg: 65.0, arm_forward_deg: 45.0 },
        ] {
            let moved = generate_posed(&recipe, pose).unwrap();
            assert_eq!(moved.quads, neutral.quads);
            assert_eq!(moved.triangles, neutral.triangles);
            assert_eq!(moved.influences.len(), neutral.influences.len());
            for (a, b) in moved.influences.iter().zip(neutral.influences.iter()) {
                assert_eq!(a.joint_indices, b.joint_indices);
                assert_eq!(a.weights, b.weights);
            }
            assert_ne!(moved.vertices, neutral.vertices);
            assert_eq!(moved.joints.len(), neutral.joints.len());
            assert!(moved.vertices.iter().flatten().all(|x| x.is_finite()));
        }
    }

    #[test]
    fn invalid_preview_pose_is_rejected_without_changing_recipe() {
        let recipe = Recipe::default();
        assert!(generate_posed(&recipe, PreviewPose {
            arm_raise_deg: 120.0,
            arm_forward_deg: 0.0,
        }).is_err());
        assert!(generate_posed(&recipe, PreviewPose {
            arm_raise_deg: f32::NAN,
            arm_forward_deg: 0.0,
        }).is_err());
        assert_eq!(recipe, Recipe::default());
    }

    #[test]
    fn unimplemented_controls_fail_explicitly() {
        let mut recipe = Recipe::default();
        recipe.musculature = 0.9;
        assert!(recipe.validate().unwrap_err().contains("not implemented"));
    }

    #[test]
    fn recipe_round_trips_and_schema_exports_only_implemented_controls() {
        let recipe = Recipe::default();
        assert_eq!(Recipe::from_json(&recipe.to_json().unwrap()).unwrap(), recipe);
        let schema: serde_json::Value = serde_json::from_str(&controls::build_schema_json().unwrap()).unwrap();
        let active = schema["controls"].as_array().unwrap();
        assert!(active.iter().all(|entry| entry["implemented"].as_bool() == Some(true)));
        assert_eq!(active.len(), BUILD_CONTROLS.iter().filter(|c| c.implemented()).count());
    }
}