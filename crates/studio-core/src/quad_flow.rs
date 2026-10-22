//! Neutral-only, anatomy-directed quad-vertex redistribution.
//!
//! Grid-derived surface nets yield closed quads but irregular pole locations
//! and axis misalignment on the limbs. We improve the vertex *layout* once,
//! without altering quad connectivity. This is not a replacement for later
//! landmark-guided connectivity and deformation-loop design.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use glam::Vec3;
use std::collections::BTreeSet;

const PASSES: usize = 14;
const MAX_TANGENT_STEP: f32 = 0.0055;
const MAX_TOTAL_OFFSET: f32 = 0.020;
const MIN_EDGE_RATIO: f32 = 0.58;
const MAX_EDGE_RATIO: f32 = 1.75;
const MIN_NORMAL_DOT: f32 = 0.28;
const MIN_TRIANGLE_AREA2: f32 = 1.0e-13;

#[derive(Clone, Copy, Debug, Default)]
pub struct FlowReport {
    pub accepted_passes: usize,
    /// Lower is better: 0 = axis/circumference aligned, 0.5 = 45 degrees.
    pub directional_error_before: f32,
    pub directional_error_after: f32,
    pub extraordinary_vertices: usize,
    pub three_valence: usize,
    pub five_or_more_valence: usize,
    pub maximum_quad_aspect: f32,
    pub minimum_corner_angle_degrees: f32,
}

/// A smooth preference for axial/circumferential directions on an arm or
/// thigh. Near joint junctions the preference fades out: a single axial
/// frame is ambiguous around an armpit or crotch.
#[derive(Clone, Copy)]
struct Frame {
    region: u8,
    axis: Vec3,
    weight: f32,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn named(target: &AnatomyTarget, label: &str) -> Result<Vec3, String> {
    target.landmarks.iter().find(|item| item.name == label)
        .map(|item| item.position)
        .ok_or_else(|| format!("Missing anatomical flow landmark: {label}"))
}

fn frame_at(
    p: Vec3, scale: f32, paths: &[(Vec3, Vec3, u8)],
) -> Frame {
    let mut result = Frame { region: 0, axis: Vec3::Y, weight: 0.0 };
    for &(start, end, region) in paths {
        let direction = (end - start).normalize();
        let t = (p - start).dot(direction) / start.distance(end);
        if !(0.0..=1.0).contains(&t) { continue; }
        let centre = start.lerp(end, t);
        let distance = centre.distance(p);
        let radius = if region <= 2 { 0.14 } else { 0.19 };
        let attenuation = 1.0 - smoothstep(radius * 0.52 * scale,
            radius * scale, distance);
        let longitudinal = smoothstep(0.08, 0.30, t)
            * (1.0 - smoothstep(0.77, 0.96, t));
        let weight = attenuation * longitudinal;
        if weight > result.weight {
            result = Frame { region, axis: direction, weight };
        }
    }
    result
}

fn longitudinal_paths(target: &AnatomyTarget)
    -> Result<Vec<(Vec3, Vec3, u8)>, String>
{
    Ok(vec![
        (named(target, "left_shoulder")?, named(target, "left_elbow")?, 1),
        (named(target, "right_shoulder")?, named(target, "right_elbow")?, 2),
        (named(target, "left_hip")?, named(target, "left_knee")?, 3),
        (named(target, "right_hip")?, named(target, "right_knee")?, 4),
    ])
}

fn quad_edges(mesh: &SurfaceMesh) -> Vec<(usize, usize)> {
    let mut set = BTreeSet::new();
    for [a,b,c,d] in mesh.quads.iter().copied() {
        for (u,v) in [(a,b), (b,c), (c,d), (d,a)] {
            set.insert((u.min(v) as usize, u.max(v) as usize));
        }
    }
    set.into_iter().collect()
}

fn frame_alignment(
    vertices: &[Vec3], frames: &[Frame], edges: &[(usize,usize)],
    target: &AnatomyTarget,
) -> f32 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for &(i,j) in edges {
        let (a,b) = (frames[i],frames[j]);
        if a.region == 0 || a.region != b.region { continue; }
        let weight = a.weight.min(b.weight);
        if weight < 0.15 { continue; }
        let centre = (vertices[i] + vertices[j]) * 0.5;
        let n = target.outward_normal(centre);
        let axial = (a.axis - n * a.axis.dot(n)).normalize_or_zero();
        let edge = vertices[j] - vertices[i];
        let tangent = (edge - n * edge.dot(n)).normalize_or_zero();
        if axial.length_squared() < 0.5 || tangent.length_squared() < 0.5 {
            continue;
        }
        let squared = axial.dot(tangent).powi(2).clamp(0.0, 1.0);
        weighted_sum += weight * squared.min(1.0 - squared);
        total_weight += weight;
    }
    if total_weight == 0.0 { 0.0 } else { weighted_sum / total_weight }
}

fn valid_update(
    positions: &[Vec3], original: &[Vec3], mesh: &SurfaceMesh,
    edges: &[(usize, usize)], scale: f32,
) -> bool {
    for &(i,j) in edges {
        let before = original[i].distance(original[j]);
        let after = positions[i].distance(positions[j]);
        if before < 1.0e-6 * scale
            || after < before * MIN_EDGE_RATIO
            || after > before * MAX_EDGE_RATIO
        {
            return false;
        }
    }
    for &[a,b,c] in &mesh.triangles {
        let (i,j,k) = (a as usize,b as usize,c as usize);
        let prev = (original[j] - original[i]).cross(original[k] - original[i]);
        let now = (positions[j] - positions[i]).cross(positions[k] - positions[i]);
        if !now.is_finite()
            || now.length_squared() < MIN_TRIANGLE_AREA2 * scale.powi(4)
            || prev.dot(now) < MIN_NORMAL_DOT * prev.length() * now.length()
        {
            return false;
        }
    }
    true
}

fn quality(mesh: &SurfaceMesh, edges: &[(usize,usize)], accepted: usize,
    before: f32, after: f32) -> FlowReport
{
    let mut valence = vec![0_usize; mesh.vertices.len()];
    for &(i,j) in edges {
        valence[i] += 1;
        valence[j] += 1;
    }
    let mut max_aspect = 1.0_f32;
    let mut min_corner = 180.0_f32;
    for quad in &mesh.quads {
        let [a,b,c,d] = quad.map(|i| mesh.vertices[i as usize]);
        let points = [a,b,c,d];
        let lengths = (0..4).map(|i| {
            points[i].distance(points[(i+1)%4])
        });
        let largest = lengths.clone().fold(0.0_f32, f32::max);
        let smallest = lengths.fold(f32::INFINITY, f32::min);
        if smallest > 1.0e-7 {
            max_aspect = max_aspect.max(largest / smallest);
        }
        for i in 0..4 {
            let origin = points[i];
            let back = (points[(i+3)%4] - origin).normalize_or_zero();
            let forward = (points[(i+1)%4] - origin).normalize_or_zero();
            min_corner = min_corner.min(
                back.dot(forward).clamp(-1.0, 1.0).acos().to_degrees()
            );
        }
    }
    FlowReport {
        accepted_passes: accepted,
        directional_error_before: before,
        directional_error_after: after,
        extraordinary_vertices: valence.iter().filter(|&&v| v != 4).count(),
        three_valence: valence.iter().filter(|&&v| v == 3).count(),
        five_or_more_valence: valence.iter().filter(|&&v| v >= 5).count(),
        maximum_quad_aspect: max_aspect,
        minimum_corner_angle_degrees: min_corner,
    }
}

/// Repositions neutral-template vertices *once*. Connectivity is untouched;
/// fitting the same vertex indices to user morphology occurs separately.
pub(crate) fn optimize(
    mesh: &mut SurfaceMesh, target: &AnatomyTarget
) -> Result<FlowReport, String> {
    let original = mesh.vertices.clone();
    let scale = target.scale;
    let paths = longitudinal_paths(target)?;
    let edges = quad_edges(mesh);
    let mut neighbors = vec![Vec::<usize>::new(); original.len()];
    for &(i,j) in &edges {
        neighbors[i].push(j);
        neighbors[j].push(i);
    }
    let frames: Vec<Frame> = original.iter()
        .map(|&p| frame_at(p, scale, &paths)).collect();
    let mut score = frame_alignment(&original, &frames, &edges, target);
    let initial = score;
    let mut accepted = 0;
    let mut positions = original.clone();

    for _ in 0..PASSES {
        let mut accumulated = vec![Vec3::ZERO; positions.len()];
        let mut weight_sum = vec![0.0_f32; positions.len()];
        for &(i,j) in &edges {
            let (a,b) = (frames[i],frames[j]);
            if a.region == 0 || a.region != b.region { continue; }
            let weight = a.weight.min(b.weight);
            if weight < 0.15 { continue; }
            let centre = (positions[i] + positions[j]) * 0.5;
            let n = target.outward_normal(centre);
            let axis = (a.axis - n * a.axis.dot(n)).normalize_or_zero();
            let edge = positions[j] - positions[i];
            let tangent = edge - n * edge.dot(n);
            if axis.length_squared() < 0.5 || tangent.length_squared() < 1.0e-9 {
                continue;
            }
            let axial = axis * tangent.dot(axis);
            let error = if axial.length_squared() * 2.0 >= tangent.length_squared() {
                tangent - axial
            } else {
                axial
            };
            let error = error.clamp_length_max(0.008 * scale);
            accumulated[i] += 0.5 * error * weight;
            accumulated[j] -= 0.5 * error * weight;
            weight_sum[i] += weight;
            weight_sum[j] += weight;
        }
        let mut desired = positions.clone();
        for i in 0..positions.len() {
            if weight_sum[i] < 0.01 { continue; }
            let p = positions[i];
            let normal = target.outward_normal(p);
            if normal.length_squared() < 0.5 { continue; }
            let center = neighbors[i].iter().map(|&j| positions[j])
                .sum::<Vec3>() / neighbors[i].len() as f32;
            let smooth = (center - p) * 0.055;
            let flow = accumulated[i] / weight_sum[i];
            let update = flow * 0.9 + smooth * frames[i].weight;
            let update = (update - normal * update.dot(normal))
                .clamp_length_max(MAX_TANGENT_STEP * scale);
            let moved = p + update;
            let correction = target.outward_normal(moved) * target.field(moved);
            let placed = moved - correction.clamp_length_max(0.004 * scale);
            desired[i] = original[i] +
                (placed - original[i]).clamp_length_max(MAX_TOTAL_OFFSET * scale);
        }

        let mut adopted = false;
        for fraction in [1.0_f32, 0.5, 0.25, 0.125, 0.0625] {
            let candidate = positions.iter().zip(&desired)
                .map(|(&a,&b)| a.lerp(b, fraction))
                .collect::<Vec<_>>();
            if !valid_update(&candidate, &original, mesh, &edges, scale) {
                continue;
            }
            let next = frame_alignment(&candidate, &frames, &edges, target);
            if next + 1.0e-6 < score {
                score = next;
                positions = candidate;
                adopted = true;
                accepted += 1;
                break;
            }
        }
        if !adopted { break; }
    }
    mesh.vertices = positions;
    mesh.validate_closed()?;
    Ok(quality(mesh, &edges, accepted, initial, score))
}
