//! Model Designer: anatomy-driven procedural mesh and rig diagnostics.
//!
//! The neutral quad template is generated from the anatomical target once;
//! morphology fitting and provisional pose previews preserve connectivity.

use glam::Quat;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub mod anatomy;
pub mod cage;
pub mod quad_template;
pub mod quad_flow;
mod torso_target;
pub mod controls;
pub mod mesh;
pub mod morphology;
pub mod skinning;
pub mod rig_preview;

pub use controls::{BuildControl, BuildParameter, BuildSection, BUILD_CONTROLS};
pub use morphology::Morphology;

pub const SCHEMA_VERSION: u32 = 1;
pub const REFERENCE_HEIGHT: f32 = 1.75;

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
            name: "Humanoid 01".to_owned(),
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
    /// Preview triangles include two triangles per quad and optional end caps.
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
    inverse_bind_matrices: Vec<[f32; 16]>,
}

impl Model {
    pub fn to_obj(&self) -> String {
        let mut obj = String::from("# Model Designer | metres | Y-up | parametric humanoid study\no humanoid_study\n");
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
        let neutral_rotations = vec![Quat::IDENTITY; self.joints.len()];
        let matrices = skinning::evaluate_joints(&self.joints, &neutral_rotations)?;
        serde_json::to_string_pretty(&RigExport {
            schema_version: SCHEMA_VERSION,
            vertex_count: self.vertices.len(),
            joint_count: self.joints.len(),
            joints: &self.joints,
            influences: &self.influences,
            inverse_bind_matrices: matrices.inverse_bind.iter()
                .map(|matrix| matrix.to_cols_array()).collect(),
        })
        .map_err(|err| err.to_string())
    }
}

