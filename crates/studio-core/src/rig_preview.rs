//! Provisional rigging and deformation diagnostics for the *fitted* quad cage.
//!
//! The skeleton uses anatomical landmarks; shoulder skin weights use stable
//! neutral-cage ownership guides rather than proximity to the arm axis.
//! Posing never regenerates topology.
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
    pub stretched_edges: usize,
    pub compressed_edges: usize,
    /// Edges shorter than a fraction of the median neutral quad edge length.
    /// Their raw ratios are unstable and reported separately.
    pub short_reference_edges: usize,
    /// Largest symmetric length-change ratio on a representative quad edge.
    pub worst_edge_stretch: f32,
    /// Largest ratio including unusually short neutral edges.
    pub worst_raw_edge_ratio: f32,
    pub worst_reference_length: f32,
    pub worst_posed_length: f32,
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

/// Lower-limb proximity weighting remains provisional until hip/groin
/// topology carries ownership guides of its own.
fn hip_weight(p: Vec3, start: Vec3, end: Vec3, scale: f32) -> f32 {
    let axis = end - start;
    let length2 = axis.length_squared();
    if length2 <= 1.0e-8 * scale * scale { return 0.0; }
    let u = (p - start).dot(axis) / length2;
    if !(-0.35..=1.40).contains(&u) { return 0.0; }
    let centre = start + axis * u.clamp(0.0, 1.0);
    let distance = p.distance(centre);
    let radial = 1.0 - smooth(0.131 * scale, 0.238 * scale, distance);
    // Most of the limb follows its joint even at u=0; only the tissue
    // entering the torso is shared gradually with the spine/chest/pelvis.
    let proximal = smooth(-0.27, 0.13, u);
    let terminal = 1.0 - smooth(1.18, 1.39, u);
    (radial * proximal * terminal).clamp(0.0, 1.0)
}

/// Normalised pelvis/spine/chest blend for the unposed trunk. The chest
/// follows shoulder rotations through its child bones; assigning every
/// non-limb vertex to pelvis previously caused excessive stretch.
fn trunk_weights(height: f32) -> [f32; 3] {
    let pelvis = 1.0 - smooth(0.77, 1.04, height);
    let upper = smooth(1.04, 1.29, height);
    let spine = (1.0 - pelvis) * (1.0 - upper);
    let chest = (1.0 - pelvis) * upper;
    [pelvis, spine, chest]
}

/// Four highest contributions are retained from up to seven continuous bone
/// fields. The result remains deterministic and exactly normalised; there
/// is no winner-takes-all selection of a single left/right limb.
fn strongest_four(raw: &[f32; 12]) -> VertexInfluence {
    let mut top = [(0_usize, 0.0_f32); 4];
    for (index, &amount) in raw.iter().enumerate() {
        if amount <= 0.0 { continue; }
        for slot in 0..4 {
            if amount > top[slot].1 {
                for shift in (slot + 1..4).rev() {
                    top[shift] = top[shift - 1];
                }
                top[slot] = (index, amount);
                break;
            }
        }
    }
    let total = top.iter().map(|(_, weight)| *weight).sum::<f32>();
    let mut result = VertexInfluence {
        joint_indices: [0; 4],
        weights: [0.0; 4],
    };
    if total <= 1.0e-8 {
        result.weights[0] = 1.0;
        return result;
    }
    for (slot, &(index, amount)) in top.iter().enumerate() {
        result.joint_indices[slot] = index as u16;
        result.weights[slot] = amount / total;
    }
    result
}

fn weights(
    points: &[Vec3], joints: &[Joint], scale: f32,
    shoulder_guides: &[[f32; 2]],
) -> Result<Vec<VertexInfluence>, String> {
    if shoulder_guides.len() != points.len()
        || shoulder_guides.iter().flatten()
            .any(|&value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err("Fitted cage has invalid anatomical shoulder ownership".into());
    }
    const HIPS: [(usize, usize); 2] = [(8, 9), (10, 11)];
    let result = points.iter().zip(shoulder_guides).map(|(&p, &shoulder)| {
        let mut limb = [0.0_f32; 4];
        // The neutral shoulder patch explicitly partitions upper arms from
        // the torso. Never infer shoulder ownership again by spatial radius:
        // this previously pulled lateral chest vertices with a raised arm.
        limb[..2].copy_from_slice(&shoulder);
        for (slot, (root, end)) in HIPS.iter().copied().enumerate() {
            limb[slot + 2] = hip_weight(
                p,
                Vec3::from_array(joints[root].position),
                Vec3::from_array(joints[end].position),
                scale
            );
        }
        // Opposing hip influences are allowed to blend, but the central
        // crotch must retain enough pelvis influence to avoid the two
        // thighs pulling its vertices directly apart.
        let groin = (1.0 - smooth(0.015, 0.085, p.x.abs() / scale))
            * (1.0 - smooth(0.68, 0.84, p.y / scale));
        limb[2] *= 1.0 - 0.60 * groin;
        limb[3] *= 1.0 - 0.60 * groin;

        let total = limb.iter().sum::<f32>();
        // Leave a small, continuous amount of torso anchoring even where
        // the limb's shoulder/hip influence is otherwise fully dominant.
        let limb_total = total.min(0.98);
        let trunk_total = 1.0 - limb_total;
        let trunk = trunk_weights(p.y / scale);
        let mut raw = [0.0_f32; 12];
        raw[0] = trunk[0] * trunk_total;
        raw[1] = trunk[1] * trunk_total;
        raw[2] = trunk[2] * trunk_total;
        if total > 1.0e-8 {
            for (slot, root) in [4_usize, 6, 8, 10].into_iter().enumerate() {
                raw[root] = limb_total * limb[slot] / total;
            }
        }
        strongest_four(&raw)
    }).collect();
    Ok(result)
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

    // A few surface-net quads have extremely short edges. Their raw
    // length-change ratios can explode even with millimetric motion.
    // Use the median reference edge to identify these separately rather
    // than allowing them to dominate the deformation-quality summary.
    let edge_lengths: Vec<(u32,u32,f32)> = edges.iter().map(|&(a,b)| {
        (a,b,reference[a as usize].distance(reference[b as usize]))
    }).collect();
    let mut lengths: Vec<f32> = edge_lengths.iter().map(|&(_,_,length)| length)
        .filter(|length| length.is_finite() && *length > 1.0e-8).collect();
    lengths.sort_by(|a,b| a.total_cmp(b));
    let median = lengths.get(lengths.len() / 2).copied().unwrap_or(0.0);
    let short_threshold = (0.15 * median).max(0.001);
    report.worst_edge_stretch = 1.0;
    report.worst_raw_edge_ratio = 1.0;
    for (a,b,before) in edge_lengths {
        let (i,j) = (a as usize,b as usize);
        let after = posed[i].distance(posed[j]);
        if before <= 1.0e-8 || !before.is_finite() || !after.is_finite() {
            report.collapsed_faces += 1;
            defects.extend([a,b]);
            continue;
        }
        let ratio = after / before;
        let symmetric = ratio.max(1.0 / ratio.max(1.0e-8));
        report.worst_raw_edge_ratio = report.worst_raw_edge_ratio.max(symmetric);

        if before < short_threshold {
            report.short_reference_edges += 1;
            // Still expose grossly distorted short edges, but do not flag
            // sub-millimetre differences solely because of a huge ratio.
            if (after - before).abs() > (0.45 * median).max(0.005) {
                report.strained_edges += 1;
                if after > before { report.stretched_edges += 1; }
                else { report.compressed_edges += 1; }
                defects.extend([a,b]);
            }
            continue;
        }

        if symmetric > report.worst_edge_stretch {
            report.worst_edge_stretch = symmetric;
            report.worst_reference_length = before;
            report.worst_posed_length = after;
        }
        if ratio > 1.80 {
            report.stretched_edges += 1;
            report.strained_edges += 1;
            defects.extend([a,b]);
        } else if ratio < 0.55 {
            report.compressed_edges += 1;
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
    let skin_weights = weights(
        &neutral_vertices, &rest_joints, target.scale, &fitted.shoulder_guides
    )?;
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
            shoulder_guides: fitted.shoulder_guides.clone(),
            joints: posed_joints,
            influences: skin_weights,
        },
        report,
        defect_vertices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoulder_guides_anchor_chest_and_move_the_arm() {
        let recipe = Recipe::default();
        let fitted = crate::cage::fit(&recipe, 1.0).unwrap().model;
        let count = fitted.vertices.len();
        assert_eq!(fitted.shoulder_guides.len(), count);
        assert_eq!(fitted.shoulder_guides,
            crate::quad_template::neutral_template().unwrap().shoulder_guides);
        let posed = pose_fitted(&fitted, &recipe, DiagnosticPose {
            left_arm: JointAngles { outward: 60.0, forward: 0.0 },
            ..DiagnosticPose::default()
        }).unwrap();
        assert_eq!(posed.model.vertices.len(), count);
        assert_eq!(posed.model.quads, fitted.quads);
        let mut moved_arm = 0;
        let mut anchored_torso = 0;
        for (i, (&guide, weight)) in fitted.shoulder_guides.iter()
            .zip(&posed.model.influences).enumerate()
        {
            let contribution = |bone: u16| weight.joint_indices.iter()
                .zip(&weight.weights).filter(|(joint, _)| **joint == bone)
                .map(|(_, weight)| *weight).sum::<f32>();
            assert!((weight.weights.iter().sum::<f32>() - 1.0).abs() < 1.0e-5);
            let distance = Vec3::from_array(fitted.vertices[i])
                .distance(Vec3::from_array(posed.model.vertices[i]));
            if guide[0] == 0.0 && guide[1] == 0.0 {
                assert_eq!(contribution(4), 0.0);
                assert_eq!(contribution(6), 0.0);
                assert!(distance < 1.0e-5,
                    "Torso vertex {i} followed the raised shoulder by {distance}");
                anchored_torso += 1;
            }
            if guide[0] == 1.0 {
                assert!(contribution(4) >= 0.97);
                if distance > 0.08 { moved_arm += 1; }
            }
            if guide[1] == 1.0 {
                assert!(contribution(6) >= 0.97);
                assert!(distance < 1.0e-5,
                    "The unposed right arm moved with the left shoulder");
            }
        }
        assert!(anchored_torso > 500);
        assert!(moved_arm > 20);
        let mut unlabelled = fitted.clone();
        unlabelled.shoulder_guides.pop();
        assert!(pose_fitted(&unlabelled, &recipe, DiagnosticPose::default()).is_err());
    }

    #[test]
    fn deltoid_rotates_with_shoulder_at_30_60_and_90_degrees() {
        let recipe = Recipe::default();
        let target = AnatomyTarget::from_recipe(&recipe).unwrap();
        let shoulder = landmark(&target, "left_shoulder").unwrap();
        let fitted = crate::cage::fit(&recipe, 1.0).unwrap().model;

        // Deltoid-cap vertices close to, but distinctly above/outside, the
        // shoulder pivot must follow the joint. Checking only distal upper
        // arm vertices missed the pinned-cap regression.
        let cap = fitted.vertices.iter().enumerate()
            .filter_map(|(i, point)| {
                let p = Vec3::from_array(*point);
                let r = p.distance(shoulder) / target.scale;
                (r >= 0.045 && r <= 0.145
                    && p.x <= shoulder.x - 0.02 * target.scale
                    && p.y >= shoulder.y - 0.035 * target.scale)
                    .then_some(i)
            }).collect::<Vec<_>>();
        assert!(cap.len() >= 8, "Expected a sampled deltoid cap");

        let joints = make_skeleton(&target).unwrap();
        let mut previous_displacement = 0.0;
        for angle in [30.0, 60.0, 90.0] {
            let pose = DiagnosticPose {
                left_arm: JointAngles { outward: angle, forward: 0.0 },
                ..DiagnosticPose::default()
            };
            let preview = pose_fitted(&fitted, &recipe, pose).unwrap();
            assert_eq!(preview.report.collapsed_faces, 0);
            if angle <= 60.0 {
                assert!(preview.report.inverted_faces <= 12
                    && preview.report.strained_edges <= 32,
                    "Excessive shoulder defects at {angle}°: {} inverted, {} strained",
                    preview.report.inverted_faces, preview.report.strained_edges);
            }
            let matrices = skinning::evaluate_joints(&joints, &local_rotations(pose)).unwrap();
            let (mut motion, mut rigid_motion, mut influence) = (0.0, 0.0, 0.0);
            for &i in &cap {
                let p = Vec3::from_array(fitted.vertices[i]);
                let moved = Vec3::from_array(preview.model.vertices[i]);
                motion += p.distance(moved);
                rigid_motion += p.distance(matrices.skin[4].transform_point3(p));
                influence += preview.model.influences[i].joint_indices.iter()
                    .zip(&preview.model.influences[i].weights)
                    .filter(|(bone, _)| **bone == 4)
                    .map(|(_, weight)| *weight).sum::<f32>();
            }
            let mean_motion = motion / cap.len() as f32;
            let ratio = motion / rigid_motion;
            let mean_influence = influence / cap.len() as f32;
            assert!(ratio > 0.60 && mean_influence > 0.60,
                "Deltoid stayed pinned at {angle}°: motion ratio {ratio:.2}, shoulder weight {mean_influence:.2}");
            assert!(mean_motion > previous_displacement,
                "Deltoid did not advance with the raised shoulder at {angle}°");
            previous_displacement = mean_motion;
        }
    }

    #[test]
    fn shoulder_flexion_rotates_deltoid_without_moving_other_arm() {
        let recipe = Recipe::default();
        let target = AnatomyTarget::from_recipe(&recipe).unwrap();
        let fitted = crate::cage::fit(&recipe, 1.0).unwrap().model;
        let left = landmark(&target, "left_shoulder").unwrap();
        let right = landmark(&target, "right_shoulder").unwrap();
        let caps = [left, right].map(|pivot| fitted.vertices.iter()
            .enumerate().filter_map(|(i, point)| {
                let p = Vec3::from_array(*point);
                let r = p.distance(pivot) / target.scale;
                (r >= 0.045 && r <= 0.145
                    && p.x.abs() >= pivot.x.abs() + 0.02 * target.scale
                    && p.y >= pivot.y - 0.035 * target.scale)
                    .then_some(i)
            }).collect::<Vec<_>>());
        assert!(caps.iter().all(|cap| cap.len() >= 8));
        for angle in [30.0, 60.0] {
            let posed = pose_fitted(&fitted, &recipe, DiagnosticPose {
                left_arm: JointAngles { outward: 0.0, forward: angle },
                ..DiagnosticPose::default()
            }).unwrap();
            let mean_motion = |side: usize| caps[side].iter().map(|&i| {
                Vec3::from_array(fitted.vertices[i])
                    .distance(Vec3::from_array(posed.model.vertices[i]))
            }).sum::<f32>() / caps[side].len() as f32;
            assert!(mean_motion(0) > 0.015,
                "Left deltoid stayed pinned during {angle}° forward flexion");
            assert!(mean_motion(1) < 1.0e-5,
                "Right shoulder moved when only the left arm flexed");
        }
    }
}
