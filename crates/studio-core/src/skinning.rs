//! Reusable linear-blend skinning for topologically fixed procedural meshes.
//!
//! The bind pose is the procedural rest skeleton. Each local joint rotation is
//! accumulated through the hierarchy and multiplied by its inverse bind matrix.

use crate::{Joint, VertexInfluence};
use glam::{Mat4, Quat, Vec3};

pub struct SkinMatrices {
    pub skin: Vec<Mat4>,
    pub inverse_bind: Vec<Mat4>,
    pub world: Vec<Mat4>,
}

pub fn evaluate_joints(rest: &[Joint], local_rotations: &[Quat]) -> Result<SkinMatrices, String> {
    if rest.len() != local_rotations.len() || rest.is_empty() {
        return Err("The rotation array must match a nonempty rest skeleton".into());
    }
    let mut world = Vec::<Mat4>::with_capacity(rest.len());
    let mut inverse_bind = Vec::with_capacity(rest.len());
    let mut skin = Vec::with_capacity(rest.len());
    for (index, joint) in rest.iter().enumerate() {
        let position = Vec3::from_array(joint.position);
        let rotation = local_rotations[index];
        if !position.is_finite() || !rotation.is_finite() {
            return Err(format!("Non-finite transform for joint {}", joint.name));
        }
        let local_offset = if let Some(parent) = joint.parent {
            if parent >= index {
                return Err(format!("Joint {} must follow its parent in the hierarchy", joint.name));
            }
            position - Vec3::from_array(rest[parent].position)
        } else {
            position
        };
        let local = Mat4::from_translation(local_offset) * Mat4::from_quat(rotation);
        let joint_world = joint.parent.map_or(local, |parent| world[parent] * local);
        let inverse = Mat4::from_translation(-position);
        let matrix = joint_world * inverse;
        if !matrix.to_cols_array().iter().all(|v| v.is_finite()) {
            return Err(format!("Invalid skin transform for joint {}", joint.name));
        }
        world.push(joint_world);
        inverse_bind.push(inverse);
        skin.push(matrix);
    }
    Ok(SkinMatrices { skin, inverse_bind, world })
}

pub fn skin_point(
    position: Vec3,
    influence: &VertexInfluence,
    matrices: &[Mat4],
) -> Result<Vec3, String> {
    let mut result = Vec3::ZERO;
    let mut total = 0.0;
    for (&index, &weight) in influence.joint_indices.iter().zip(&influence.weights) {
        if !weight.is_finite() || weight < 0.0 {
            return Err("Skin weights must be finite and nonnegative".into());
        }
        if weight == 0.0 {
            continue;
        }
        let transform = matrices.get(index as usize)
            .ok_or_else(|| format!("Invalid skin joint index {index}"))?;
        result += transform.transform_point3(position) * weight;
        total += weight;
    }
    if (total - 1.0).abs() > 1e-4 || !result.is_finite() {
        return Err("Skin weights must sum to one and produce finite positions".into());
    }
    Ok(result)
}

pub fn deform(
    rest_vertices: &[Vec3],
    weights: &[VertexInfluence],
    transforms: &[Mat4],
) -> Result<Vec<Vec3>, String> {
    if rest_vertices.len() != weights.len() {
        return Err("Vertex positions and skin weights have different lengths".into());
    }
    rest_vertices.iter().zip(weights.iter())
        .map(|(&position, weight)| skin_point(position, weight, transforms))
        .collect()
}

pub fn posed_joints(rest: &[Joint], matrices: &SkinMatrices) -> Result<Vec<Joint>, String> {
    if rest.len() != matrices.world.len() {
        return Err("Rest skeleton and skin matrices differ in size".into());
    }
    Ok(rest.iter().enumerate().map(|(index, joint)| {
        Joint {
            name: joint.name,
            parent: joint.parent,
            position: matrices.world[index].transform_point3(Vec3::ZERO).to_array(),
        }
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_pose_is_identity_for_every_bone() {
        let skeleton = vec![
            Joint { name: "root", parent: None, position: [0.0, 0.0, 0.0] },
            Joint { name: "arm", parent: Some(0), position: [1.0, 1.0, 0.0] },
        ];
        let bind = evaluate_joints(&skeleton, &[Quat::IDENTITY; 2]).unwrap();
        for matrix in bind.skin {
            let values = matrix.to_cols_array();
            let identity = Mat4::IDENTITY.to_cols_array();
            for (a, b) in values.iter().zip(identity.iter()) {
                assert!((a - b).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn child_joint_inherits_parent_rotation_and_rotates_about_bind_pivot() {
        let skeleton = vec![
            Joint { name: "root", parent: None, position: [0.0, 0.0, 0.0] },
            Joint { name: "shoulder", parent: Some(0), position: [1.0, 0.0, 0.0] },
            Joint { name: "elbow", parent: Some(1), position: [2.0, 0.0, 0.0] },
        ];
        let matrices = evaluate_joints(
            &skeleton,
            &[Quat::IDENTITY, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Quat::IDENTITY]
        ).unwrap();
        let joints = posed_joints(&skeleton, &matrices).unwrap();
        let shoulder = Vec3::from_array(joints[1].position);
        let elbow = Vec3::from_array(joints[2].position);
        assert!(shoulder.distance(Vec3::new(1.0, 0.0, 0.0)) < 1e-5);
        assert!(elbow.distance(Vec3::new(1.0, 1.0, 0.0)) < 1e-5);
    }
}
