//! Provisional rigging and deformation diagnostics for the *fitted* quad cage.
//!
//! The skeleton and continuous skin weights are rebuilt from the anatomical
//! landmarks whenever morphology changes. Posing never regenerates topology.
//! This is a diagnostic preview, NOT validated production rigging or export.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use crate::skinning;
use crate::{Joint, Model, Recipe, VertexInfluence};
use glam::{Mat4, Quat, Vec3};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JointAngles {
    /// Shoulder/hip abduction, away from the body midline (degrees).
    pub outward: f32,
    /// Forward flexion relative to neutral (degrees).
    pub forward: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DiagnosticPose {
    pub left_arm: JointAngles,
    pub right_arm: JointAngles,
    pub left_leg: JointAngles,
    pub right_leg: JointAngles,
}

impl DiagnosticPose {
    pub fn validate(self) -> Result<(), String> {
        for (label, item, outward_limit, forward_limit) in [
            ("left arm", self.left_arm, 100.0, 90.0),
            ("right arm", self.right_arm, 100.0, 90.0),
            ("left leg", self.left_leg, 55.0, 85.0),
            ("right leg", self.right_leg, 55.0, 85.0),
        ] {
            if !item.outward.is_finite() || !(0.0..=outward_limit).contains(&item.outward)
                || !item.forward.is_finite() || !(-20.0..=forward_limit).contains(&item.forward)
            {
                return Err(format!("Invalid {label} diagnostic pose"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DeformationReport {
    pub inverted_faces: usize,
    pub collapsed_faces: usize,
    pub strained_edges: usize,
    pub worst_edge_stretch: f32,
    pub largest_vertex_displacement: f32,
}

pub struct RiggedPreview {
    pub model: Model,
    pub report: DeformationReport,
    /// Indices of vertices involved in collapsed/inverted or distorted faces.
    pub defect_vertices: Vec<u32>,
}

fn landmark(target: &AnatomyTarget, name: &str) -> Result<Vec3, String> {
    target.landmarks.iter().find(|item| item.name == name)
        .map(|item| item.position)
        .ok_or_else(|| format!("Missing diagnostic skeleton landmark: {name}"))
}

fn joint(name: &'static str, parent: Option<usize>, position: Vec3) -> Joint {
    Joint { name, parent, position: position.to_array() }
}

/// Parent-before-child hierarchy compatible with the existing shared
/// inverse-bind and linear-blend skinning implementation.
fn make_skeleton(target: &AnatomyTarget) -> Result<Vec<Joint>, String> {
    let s = target.scale;
    let waist = landmark(target, "waist")?;
    let spine = Vec3::new(0.0, 1.02 * s, waist.z);
    let chest = landmark(target, "sternum")? - Vec3::Z * 0.08 * s;
    Ok(vec![
        joint("pelvis", None, landmark(target, "sacrum")? + Vec3::Y * 0.05 * s),
        joint("spine", Some(0), spine),
        joint("chest", Some(1), chest),
        joint("neck", Some(2), landmark(target, "neck_top")?),
        joint("left_shoulder", Some(2), landmark(target, "left_shoulder")?),
        joint("left_elbow", Some(4), landmark(target, "left_elbow")?),
        joint("right_shoulder", Some(2), landmark(target, "right_shoulder")?),
        joint("right_elbow", Some(6), landmark(target, "right_elbow")?),
        joint("left_hip", Some(0), landmark(target, "left_hip")?),
        joint("left_knee", Some(8), landmark(target, "left_knee")?),
        joint("right_hip", Some(0), landmark(target, "right_hip")?),
        joint("right_knee", Some(10), landmark(target, "right_knee")?),
    ])
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Approximate anatomical influence of one upper-limb segment. Weight fades
/// along its root and radially into the trunk, avoiding a hard skinning seam.
fn segment_weight(
    p: Vec3, start: Vec3, end: Vec3, scale: f32, arm: bool,
) -> f32 {
    let axis = end - start;
    let u = (p - start).dot(axis) / axis.length_squared();
    if !(-0.25..=1.35).contains(&u) {
        return 0.0;
    }
    let distance = (p - (start + axis * u.clamp(0.0, 1.0))).length();
    let radius = if arm { 0.135 } else { 0.165 };
    let radial = 1.0 - smooth(radius * scale, (radius + 0.065) * scale, distance);
    let root = smooth(-0.18, if arm { 0.28 } else { 0.34 }, u);
    let end_fade = 1.0 - smooth(1.14, 1.36, u);
    (radial * root * end_fade).clamp(0.0, 1.0)
}

fn weights(points: &[Vec3], joints: &[Joint], scale: f32) -> Vec<VertexInfluence> {
    // The highest-scoring limb dominates, preventing four overlapping limb
    // influences across the medial groin. Pelvis retains the residual weight.
    const SEGMENTS: [(usize, usize, bool); 4] = [
        (4, 5, true), (6, 7, true), (8, 9, false), (10, 11, false)
    ];
    points.iter().map(|&p| {
        let mut strongest = 0.0_f32;
        let mut selected = 0_usize;
        for (start, end, arm) in SEGMENTS {
            let influence = segment_weight(p,
                Vec3::from_array(joints[start].position),
                Vec3::from_array(joints[end].position), scale, arm);
            if influence > strongest {
                strongest = influence;
                selected = start;
            }
        }
        let limb_weight = strongest.clamp(0.0, 1.0);
        VertexInfluence {
            joint_indices: [0, selected as u16, 0, 0],
            weights: [1.0 - limb_weight, limb_weight, 0.0, 0.0],
        }
    }).collect()
}

fn local_rotations(pose: DiagnosticPose) -> Vec<Quat> {
    let mut angles = vec![Quat::IDENTITY; 12];
    for (index, side, p) in [
        (4, -1.0_f32, pose.left_arm), (6, 1.0, pose.right_arm),
    ] {
        let raise = Quat::from_rotation_z(side * p.outward.to_radians());
        let reach = Quat::from_rotation_y(-side * p.forward.to_radians());
        angles[index] = reach * raise;
    }
    for (index, side, p) in [
        (8, -1.0_f32, pose.left_leg), (10, 1.0, pose.right_leg),
    ] {
        let abduct = Quat::from_rotation_z(side * p.outward.to_radians());
        let flex = Quat::from_rotation_x(-p.forward.to_radians());
        angles[index] = flex * abduct;
    }
    angles
}

/// Measures reference-to-pose face inversion and gross stretch, without
/// suppressing defects: displaying deformation failures is the goal.
fn analyze(reference: &[Vec3], posed: &[Vec3], triangles: &[[u32; 3]],
    quads: &[[u32; 4]], influences: &[VertexInfluence], skin: &[Mat4])
    -> (DeformationReport, Vec<u32>)
{
    let mut report = DeformationReport::default();
    let mut defects = BTreeSet::<u32>::new();
    for &[a,b,c] in triangles {
        let indices = [a as usize, b as usize, c as usize];
        let original = (reference[indices[1]] - reference[indices[0]])
            .cross(reference[indices[2]] - reference[indices[0]]);
        let current = (posed[indices[1]] - posed[indices[0]])
            .cross(posed[indices[2]] - posed[indices[0]]);
        // Compare against the *skinned* expected normal, not the unposed
        // world-space normal. A correctly rotated arm can turn well beyond
        // 90 degrees without its faces flipping in their local coordinates.
        let mut expected = Vec3::ZERO;
        let rest_normal = original.normalize_or_zero();
        for &id in &indices {
            let weight = &influences[id];
            for (&joint, &amount) in weight.joint_indices.iter().zip(&weight.weights) {
                if amount > 0.0 {
                    expected += skin[joint as usize].transform_vector3(rest_normal)
                        * (amount / 3.0);
                }
            }
        }
        if current.length_squared() < 0.0025 * original.length_squared() {
            report.collapsed_faces += 1;
            defects.extend([a,b,c]);
        } else if expected.length_squared() > 1.0e-8
            && current.dot(expected) <= 0.0
        {
            report.inverted_faces += 1;
            defects.extend([a,b,c]);
        }
    }
    let mut edges = BTreeSet::<(u32,u32)>::new();
    for &[a,b,c,d] in quads {
        for (u,v) in [(a,b),(b,c),(c,d),(d,a)] {
            edges.insert((u.min(v),u.max(v)));
        }
    }
    for (a,b) in edges {
        let (i,j) = (a as usize,b as usize);
        let before = reference[i].distance(reference[j]);
        let after = posed[i].distance(posed[j]);
        if before < 1.0e-7 { continue; }
        let ratio = after / before;
        report.worst_edge_stretch = report.worst_edge_stretch.max(ratio.max(1.0 / ratio.max(1.0e-7)));
        if ratio < 0.55 || ratio > 1.8 {
            report.strained_edges += 1;
            defects.extend([a,b]);
        }
    }
    report.largest_vertex_displacement = reference.iter().zip(posed.iter())
        .map(|(a,b)| a.distance(*b)).fold(0.0_f32,f32::max);
    (report, defects.into_iter().collect())
}

/// Neutral fitted-cage positions must be supplied on *every* pose change.
/// The vertex/index correspondence is unchanged, including in extreme poses.
pub fn pose_fitted(
    fitted: &Model, recipe: &Recipe, pose: DiagnosticPose
) -> Result<RiggedPreview, String> {
    recipe.validate()?;
    pose.validate()?;
    if fitted.vertices.is_empty() || fitted.quads.is_empty() {
        return Err("Rigging preview requires a fitted quad cage".into());
    }
    let target = AnatomyTarget::from_recipe(recipe)?;
    let rest_joints = make_skeleton(&target)?;
    let neutral_vertices: Vec<Vec3> = fitted.vertices.iter()
        .map(|p| Vec3::from_array(*p)).collect();
    let skin_weights = weights(&neutral_vertices, &rest_joints, target.scale);
    let matrices = skinning::evaluate_joints(&rest_joints, &local_rotations(pose))?;
    let positions = skinning::deform(&neutral_vertices, &skin_weights, &matrices.skin)?;
    let posed_joints = skinning::posed_joints(&rest_joints, &matrices)?;
    let (report, defect_vertices) = analyze(
        &neutral_vertices, &positions, &fitted.triangles, &fitted.quads,
        &skin_weights, &matrices.skin
    );
    let surface = SurfaceMesh {
        vertices: positions,
        quads: fitted.quads.clone(),
        triangles: fitted.triangles.clone(),
        caps: fitted.caps.clone(),
    };
    let normals = surface.normals();
    Ok(RiggedPreview {
        model: Model {
            vertices: surface.vertices.iter().map(|p| p.to_array()).collect(),
            normals,
            quads: surface.quads,
            triangles: surface.triangles,
            caps: surface.caps,
            joints: posed_joints,
            influences: skin_weights,
        },
        report,
        defect_vertices,
    })
}
