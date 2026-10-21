//! Model Designer: topology-first procedural modelling core.
//!
//! Generic quad meshing is separate from anatomy-specific construction.
//! The only active generator is a connected upper-body geometry study.

use glam::Quat;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub mod anatomy;
pub mod controls;
pub mod mesh;
pub mod morphology;
pub mod skinning;
mod study;
mod pelvis;

pub use controls::{BuildControl, BuildParameter, BuildSection, BUILD_CONTROLS};
pub use morphology::Morphology;

pub const SCHEMA_VERSION: u32 = 1;
pub const REFERENCE_HEIGHT: f32 = 1.75;

/// Independent diagnostic rotation of an upper arm. Not saved in a recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmPose {
    pub raise_deg: f32,
    pub forward_deg: f32,
}
impl ArmPose {
    fn validate(self) -> Result<(), String> {
        if !self.raise_deg.is_finite() || !(0.0..=70.0).contains(&self.raise_deg)
            || !self.forward_deg.is_finite() || !(0.0..=55.0).contains(&self.forward_deg)
        {
            return Err("Arm raise must be 0–70° and forward reach 0–55°".into());
        }
        Ok(())
    }
}

/// Ephemeral skinning preview: two independent arms, one canonical bind pose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviewPose {
    pub left: ArmPose,
    pub right: ArmPose,
}
impl PreviewPose {
    pub fn validate(self) -> Result<(), String> {
        self.left.validate()?;
        self.right.validate()
    }
    pub fn is_neutral(self) -> bool {
        self.left == ArmPose::default() && self.right == ArmPose::default()
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

/// Generate the neutral procedural bind-pose geometry.
pub fn generate(recipe: &Recipe) -> Result<Model, String> {
    generate_posed(recipe, PreviewPose::default())
}

/// Proper hierarchical LBS: neutral geometry and its authored skin weights
/// remain unchanged, while each joint receives pose-world × inverse-bind.
pub fn generate_posed(recipe: &Recipe, pose: PreviewPose) -> Result<Model, String> {
    recipe.validate()?;
    pose.validate()?;
    let mut study = study::build(recipe)?;
    let rest_joints = study::skeleton(recipe);
    let rotations = study::pose_rotations(pose);
    let transforms = skinning::evaluate_joints(&rest_joints, &rotations)?;
    let joints = if pose.is_neutral() {
        rest_joints
    } else {
        skinning::posed_joints(&rest_joints, &transforms)?
    };

    if !pose.is_neutral() {
        let mut positions = skinning::deform(
            &study.surface.vertices, &study.weights, &transforms.skin
        )?;
        study::preserve_shoulder_volume(&study, &mut positions, &transforms.skin, pose)?;
        let neutral_vertices = study.surface.vertices.clone();
        study.surface.vertices = positions;
        study::fair_posed_upper_shoulder(&mut study.surface, &neutral_vertices, recipe, pose)?;
        study.surface.validate_closed()?;
    }
    let normals = study.surface.normals();
    Ok(Model {
        vertices: study.surface.vertices.iter().map(|p| p.to_array()).collect(),
        normals,
        quads: study.surface.quads,
        triangles: study.surface.triangles,
        caps: study.surface.caps,
        joints,
        influences: study.weights,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn exported_mesh_has_quads_and_a_consistent_rig() {
        let m = generate(&Recipe::default()).unwrap();
        assert_eq!(m.joints.len(), 14);
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
    fn all_upper_body_slider_extremes_preserve_topology_and_weights() {
        let reference = generate(&Recipe::default()).unwrap();
        let active: Vec<BuildControl> = BUILD_CONTROLS.iter().copied()
            .filter(|control| control.implemented()).collect();
        assert_eq!(active.len(), 35);
        for control in active {
            assert_ne!(control.description(),
                "Not available until the corresponding anatomy is implemented.");
            for value in [control.min, control.max] {
                let mut recipe = Recipe::default();
                control.set(&mut recipe, value);
                let model = generate(&recipe)
                    .unwrap_or_else(|err| panic!("{}={value}: {err}", control.id));
                assert_eq!(model.quads, reference.quads, "{}", control.id);
                assert_eq!(model.triangles, reference.triangles, "{}", control.id);
                assert_eq!(model.joints.len(), reference.joints.len());
                assert_eq!(model.influences.len(), reference.influences.len());
                assert!(model.vertices.iter().flatten().all(|value| value.is_finite()));
                assert!(model.normals.iter().flatten().all(|value| value.is_finite()));
            }
        }
    }

    #[test]
    fn morphology_recipe_roundtrips_new_parameters() {
        let mut recipe = Recipe::default();
        recipe.morphology.pectoral_fullness = 0.9;
        recipe.morphology.deltoid_volume = 0.8;
        recipe.morphology.biceps_volume = 0.7;
        recipe.morphology.waist_depth = 0.85;
        recipe.morphology.shoulder_forward = 0.25;
        let json = recipe.to_json().unwrap();
        assert_eq!(Recipe::from_json(&json).unwrap(), recipe);
        let schema: serde_json::Value =
            serde_json::from_str(&controls::build_schema_json().unwrap()).unwrap();
        let sliders = schema["controls"].as_array().unwrap();
        assert_eq!(sliders.len(), 35);
        assert!(sliders.iter().all(|control| control["description"].as_str()
            .is_some_and(|description| !description.is_empty())));
    }

    #[test]
    fn chest_and_arm_controls_affect_expected_regions() {
        let baseline = generate(&Recipe::default()).unwrap();
        let mut pec = Recipe::default();
        pec.morphology.pectoral_fullness = 1.0;
        let pec_mesh = generate(&pec).unwrap();
        // Ring row 8, central anterior vertex at column 0.
        let sternum_front = 8 * 32;
        assert!(pec_mesh.vertices[sternum_front][2] >
            baseline.vertices[sternum_front][2]);

        let mut biceps = Recipe::default();
        biceps.morphology.biceps_volume = 1.0;
        let mut triceps = Recipe::default();
        triceps.morphology.triceps_volume = 1.0;
        let front = generate(&biceps).unwrap();
        let rear = generate(&triceps).unwrap();
        // First arm's mid-biceps ring: 16 vertices, indices 466..482.
        let indices = 466..482;
        let max_z = |model: &Model| indices.clone()
            .map(|index| model.vertices[index][2])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_z = |model: &Model| indices.clone()
            .map(|index| model.vertices[index][2])
            .fold(f32::INFINITY, f32::min);
        assert!(max_z(&front) > max_z(&baseline));
        assert!(min_z(&rear) < min_z(&baseline));
    }

    #[test]
    fn proportions_move_relevant_skeletal_landmarks() {
        let original = generate(&Recipe::default()).unwrap();
        let mut shoulders = Recipe::default();
        shoulders.morphology.shoulder_forward = 1.0;
        let moved = generate(&shoulders).unwrap();
        assert!(moved.joints[5].position[2] > original.joints[5].position[2]);
        assert!(moved.joints[8].position[2] > original.joints[8].position[2]);

        let mut neck = Recipe::default();
        neck.morphology.neck_length = 1.15;
        let higher = generate(&neck).unwrap();
        assert!(higher.joints[3].position[1] > original.joints[3].position[1]);
        assert_eq!(higher.quads, original.quads);
    }

    #[test]
    fn pelvis_and_thigh_controls_keep_shared_manifold_topology() {
        let default = generate(&Recipe::default()).unwrap();
        assert_eq!(default.vertices.len(), 720);
        assert_eq!(default.quads.len(), 656);
        assert_eq!(default.triangles.len(), 1400);
        assert_eq!(default.joints.len(), 14);
        for control in BUILD_CONTROLS.iter().copied().filter(|c| c.implemented()
            && matches!(c.section, BuildSection::Pelvis | BuildSection::Legs))
        {
            let mut edited = Recipe::default();
            control.set(&mut edited, control.max);
            let mesh = generate(&edited).unwrap();
            assert_eq!(mesh.quads, default.quads, "{}", control.id);
            assert_eq!(mesh.triangles, default.triangles, "{}", control.id);
            assert_ne!(mesh.vertices, default.vertices, "{}", control.id);
            assert!(mesh.vertices.iter().flatten().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn one_arm_at_a_time_moves_only_its_own_shoulder_and_elbow() {
        let recipe = Recipe::default();
        let neutral = generate(&recipe).unwrap();
        let l = generate_posed(&recipe, PreviewPose {
            left: ArmPose { raise_deg: 60.0, forward_deg: 35.0 },
            right: ArmPose::default(),
        }).unwrap();
        let r = generate_posed(&recipe, PreviewPose {
            left: ArmPose::default(),
            right: ArmPose { raise_deg: 60.0, forward_deg: 35.0 },
        }).unwrap();
        // [4,5,6] = left clavicle/shoulder/elbow, [7,8,9] right.
        assert_ne!(l.joints[6].position, neutral.joints[6].position);
        assert!(
            Vec3::from_array(l.joints[9].position)
                .distance(Vec3::from_array(neutral.joints[9].position)) < 1e-5
        );
        assert!(
            Vec3::from_array(r.joints[6].position)
                .distance(Vec3::from_array(neutral.joints[6].position)) < 1e-5
        );
        // A shoulder may move its own local chest seam, but never the
        // opposite chest half or the central sternum.
        assert_ne!(l.vertices[8 * 32 + 24], neutral.vertices[8 * 32 + 24]);
        assert_ne!(r.vertices[8 * 32 + 8], neutral.vertices[8 * 32 + 8]);
        for row in 0..13 {
            for column in 0..32 {
                let index = row * 32 + column;
                let rest = Vec3::from_array(neutral.vertices[index]);
                if (2..=14).contains(&column) {
                    assert!(Vec3::from_array(l.vertices[index]).distance(rest) < 1e-5);
                }
                if (18..=30).contains(&column) {
                    assert!(Vec3::from_array(r.vertices[index]).distance(rest) < 1e-5);
                }
            }
        }
        for index in 416..418 {
            let rest = Vec3::from_array(neutral.vertices[index]);
            assert!(Vec3::from_array(l.vertices[index]).distance(rest) < 1e-5);
            assert!(Vec3::from_array(r.vertices[index]).distance(rest) < 1e-5);
        }
        for i in 418..499 {
            assert!(Vec3::from_array(r.vertices[i])
                .distance(Vec3::from_array(neutral.vertices[i])) < 1e-5);
        }
        for i in 499..580 {
            assert!(Vec3::from_array(l.vertices[i])
                .distance(Vec3::from_array(neutral.vertices[i])) < 1e-5);
        }
        assert_ne!(r.joints[9].position, neutral.joints[9].position);
        assert_eq!(l.vertices.len(), neutral.vertices.len());
        assert_eq!(r.vertices.len(), neutral.vertices.len());
    }

    #[test]
    fn skinning_preserves_connectivity_and_neutral_weights() {
        let recipe = Recipe::default();
        let neutral = generate(&recipe).unwrap();
        for pose in [
            PreviewPose {
                left: ArmPose { raise_deg: 45.0, forward_deg: 0.0 },
                right: ArmPose::default(),
            },
            PreviewPose {
                left: ArmPose::default(),
                right: ArmPose { raise_deg: 0.0, forward_deg: 40.0 },
            },
            PreviewPose {
                left: ArmPose { raise_deg: 65.0, forward_deg: 45.0 },
                right: ArmPose { raise_deg: 70.0, forward_deg: 55.0 },
            },
        ] {
            let moved = generate_posed(&recipe, pose).unwrap();
            assert_eq!(moved.quads, neutral.quads);
            assert_eq!(moved.triangles, neutral.triangles);
            assert_eq!(moved.joints.len(), neutral.joints.len());
            for (a, b) in moved.influences.iter().zip(neutral.influences.iter()) {
                assert_eq!(a.joint_indices, b.joint_indices);
                assert_eq!(a.weights, b.weights);
            }
            assert_ne!(moved.vertices, neutral.vertices);
            assert!(moved.vertices.iter().flatten().all(|x| x.is_finite()));
        }
    }

    /// Measure the worst normal transition around the superior chest/arm join.
    fn worst_upper_shoulder_normal_dot(model: &Model) -> f32 {
        use std::collections::HashMap;
        let mut neighbours = HashMap::<(u32, u32), Vec<Vec3>>::new();
        for &[a, b, c] in &model.triangles {
            let va = Vec3::from_array(model.vertices[a as usize]);
            let vb = Vec3::from_array(model.vertices[b as usize]);
            let vc = Vec3::from_array(model.vertices[c as usize]);
            let normal = (vb - va).cross(vc - va).normalize();
            for (u, v) in [(a, b), (b, c), (c, a)] {
                neighbours.entry((u.min(v), u.max(v)))
                    .or_default().push(normal);
            }
        }
        let mut worst = 1.0_f32;
        let mut inspected = 0;
        for ((a, b), normals) in neighbours {
            if normals.len() != 2 { continue; }
            let point = (Vec3::from_array(model.vertices[a as usize])
                + Vec3::from_array(model.vertices[b as usize])) * 0.5;
            if !(0.15..0.38).contains(&point.x.abs())
                || !(1.39..1.50).contains(&point.y)
            {
                continue;
            }
            inspected += 1;
            worst = worst.min(normals[0].dot(normals[1]));
        }
        assert!(inspected > 40);
        worst
    }

    #[test]
    fn upper_socket_stays_smooth_in_neutral_and_single_arm_poses() {
        let recipe = Recipe::default();
        let neutral = generate(&recipe).unwrap();
        assert!(worst_upper_shoulder_normal_dot(&neutral) > 0.55);
        for posed in [
            PreviewPose {
                left: ArmPose { raise_deg: 70.0, forward_deg: 0.0 },
                right: ArmPose::default(),
            },
            PreviewPose {
                left: ArmPose::default(),
                right: ArmPose { raise_deg: 0.0, forward_deg: 55.0 },
            },
        ] {
            let model = generate_posed(&recipe, posed).unwrap();
            assert!(worst_upper_shoulder_normal_dot(&model) > 0.40);
            assert_eq!(model.triangles, neutral.triangles);
            assert_eq!(model.quads, neutral.quads);
        }
    }

    #[test]
    fn invalid_independent_pose_is_rejected_without_changing_recipe() {
        let recipe = Recipe::default();
        for invalid in [
            PreviewPose {
                left: ArmPose { raise_deg: 120.0, forward_deg: 0.0 },
                right: ArmPose::default(),
            },
            PreviewPose {
                left: ArmPose::default(),
                right: ArmPose { raise_deg: f32::NAN, forward_deg: 0.0 },
            },
        ] {
            assert!(generate_posed(&recipe, invalid).is_err());
        }
        assert_eq!(recipe, Recipe::default());
    }

    #[test]
    fn neutral_rig_exports_inverse_bind_matrices_for_all_joints() {
        let model = generate(&Recipe::default()).unwrap();
        let exported: serde_json::Value =
            serde_json::from_str(&model.to_rig_json().unwrap()).unwrap();
        assert_eq!(
            exported["inverse_bind_matrices"].as_array().unwrap().len(),
            model.joints.len()
        );
    }

    #[test]
    fn unimplemented_controls_fail_explicitly() {
        let mut recipe = Recipe::default();
        recipe.morphology.forearm_girth = 1.10;
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