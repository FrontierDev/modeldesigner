//! Constrained fitting of the neutral anatomy-derived quad cage.
//!
//! The quad_template module extracts new indexed all-quad topology ONLY ONCE
//! from the neutral anatomical field. This fitter changes only vertex positions
//! for subsequent morphology recipes; no old study topology is inherited.
//! Resulting geometry has no approved UVs, skinning or production rig.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use crate::quad_flow::FlowReport;
use crate::quad_template::{
    neutral_flow_report, neutral_shoulder_report, neutral_template
};
use crate::shoulder_patch::ShoulderPatchReport;
use crate::{Model, Recipe};
use glam::Vec3;

const ITERATIONS: usize = 28;
const ANCHOR_PASSES: usize = 8;
const PROJECTION_STEP: f32 = 0.035;
const MAX_BIND_OFFSET: f32 = 0.25;
const RELAX_RATE: f32 = 0.045;
const MIN_AREA_SQUARED: f32 = 1.0e-14;
const MIN_FACE_ALIGNMENT: f32 = 0.035;
const MIN_EDGE_RATIO: f32 = 0.38;
const MAX_EDGE_RATIO: f32 = 2.65;

/// Reduce tangential drift in high-curvature anatomical junctions. This is
/// a fitting constraint, not a substitute for skeletal edge-flow alignment.
fn fit_mobility(point: Vec3, scale: f32) -> f32 {
    let p = point / scale;
    let crotch = ((p.y - 0.65).abs() / 0.17).clamp(0.0, 1.0);
    let midline = (p.x.abs() / 0.12).clamp(0.0, 1.0);
    let groin_protection = (1.0 - crotch) * (1.0 - midline);
    let axilla_height = (1.0 - ((p.y - 1.33).abs() / 0.18)).clamp(0.0, 1.0);
    let axilla_lateral = (1.0 - ((p.x.abs() - 0.22).abs() / 0.12)).clamp(0.0, 1.0);
    let axilla_protection = axilla_height * axilla_lateral;
    (1.0 - 0.62 * groin_protection)
        .min(1.0 - 0.42 * axilla_protection)
        .clamp(0.25, 1.0)
}

#[derive(Clone, Copy, Debug)]
pub struct FitReport {
    pub iterations: usize,
    pub initial_mean_error: f32,
    pub mean_error: f32,
    pub maximum_error: f32,
    /// Mean signed-field residual sampled at render-triangle centroids.
    pub surface_mean_error: f32,
    pub surface_maximum_error: f32,
    pub maximum_displacement: f32,
    /// Vertices moved measurably away from the neutral template.
    pub moved_vertices: usize,
    /// Local candidates rejected by geometric safety checks.
    pub rejected_moves: usize,
    pub neutral_flow: FlowReport,
    pub shoulder_patch: ShoulderPatchReport,
}

#[derive(Clone, Debug)]
pub struct FittedCage {
    /// New neutral-anatomy surface-net quads; no rig or UVs.
    pub model: Model,
    pub report: FitReport,
}

fn neighbours(model: &Model) -> Result<Vec<Vec<usize>>, String> {
    let count = model.vertices.len();
    let mut adjacent = vec![Vec::new(); count];
    for &[a, b, c, d] in &model.quads {
        for (u, v) in [(a,b), (b,c), (c,d), (d,a)] {
            let u = u as usize;
            let v = v as usize;
            if u >= count || v >= count {
                return Err("Cage has a quad with invalid indices".into());
            }
            adjacent[u].push(v);
            adjacent[v].push(u);
        }
    }
    for adjacency in &mut adjacent {
        adjacency.sort_unstable();
        adjacency.dedup();
    }
    Ok(adjacent)
}

/// Candidate safety must be local. Previously a single tight quad anywhere
/// in the 4k-vertex cage vetoed the entire fitting iteration, so broad-chest
/// and narrow-shoulder recipes barely moved even though most vertices were
/// unconstrained. The incident-face and incident-edge checks protect the
/// same geometry while allowing unaffected body regions to continue fitting.
fn incident_triangles(model: &Model) -> Result<Vec<Vec<usize>>, String> {
    let mut incident = vec![Vec::new(); model.vertices.len()];
    for (face, triangle) in model.triangles.iter().enumerate() {
        for &id in triangle {
            let slot = incident.get_mut(id as usize)
                .ok_or_else(|| "Cage triangle has an invalid vertex index".to_owned())?;
            slot.push(face);
        }
    }
    Ok(incident)
}

fn safe_vertex_move(
    index: usize,
    proposed: Vec3,
    positions: &[Vec3],
    reference: &[Vec3],
    model: &Model,
    adjacency: &[Vec<usize>],
    incident: &[Vec<usize>],
    scale: f32,
) -> bool {
    if !proposed.is_finite() { return false; }
    for &other in &adjacency[index] {
        let original = reference[index].distance(reference[other]);
        let distance = proposed.distance(positions[other]);
        if original <= 1.0e-7 * scale
            || distance < original * MIN_EDGE_RATIO
            || distance > original * MAX_EDGE_RATIO
        {
            return false;
        }
    }
    let minimum = MIN_AREA_SQUARED * scale.powi(4);
    for &face in &incident[index] {
        let ids = model.triangles[face];
        let mut now = ids.map(|id| positions[id as usize]);
        for (k, &id) in ids.iter().enumerate() {
            if id as usize == index { now[k] = proposed; }
        }
        let prior = ids.map(|id| reference[id as usize]);
        let current_normal = (now[1] - now[0]).cross(now[2] - now[0]);
        let previous_normal = (prior[1] - prior[0]).cross(prior[2] - prior[0]);
        if !current_normal.is_finite()
            || current_normal.length_squared() < minimum
            || previous_normal.length_squared() < minimum
            || current_normal.dot(previous_normal)
                < MIN_FACE_ALIGNMENT * current_normal.length() * previous_normal.length()
        {
            return false;
        }
    }
    true
}

/// Try to apply one vertex's displacement without changing connectivity or
/// compromising adjacent quads. Other vertices are not held back by a
/// separate region's problematic face.
fn apply_local(
    index: usize,
    destination: Vec3,
    positions: &mut [Vec3],
    reference: &[Vec3],
    model: &Model,
    adjacency: &[Vec<usize>],
    incident: &[Vec<usize>],
    bind_limit: f32,
    scale: f32,
) -> bool {
    let original = positions[index];
    let bounded = reference[index]
        + (destination - reference[index]).clamp_length_max(bind_limit);
    if bounded.distance_squared(original) <= 1.0e-14 * scale * scale {
        return false;
    }
    for factor in [1.0_f32, 0.5, 0.25, 0.125, 0.0625, 0.03125] {
        let candidate = original.lerp(bounded, factor);
        if safe_vertex_move(index, candidate, positions, reference,
            model, adjacency, incident, scale)
        {
            positions[index] = candidate;
            return true;
        }
    }
    false
}

#[derive(Clone, Copy)]
struct MorphAnchor {
    from: Vec3,
    to: Vec3,
    start_offset: Vec3,
    end_offset: Vec3,
    radius: f32,
}

fn landmark(target: &AnatomyTarget, name: &str) -> Result<Vec3, String> {
    target.landmarks.iter()
        .find(|item| item.name == name)
        .map(|item| item.position)
        .ok_or_else(|| format!("Cage fitting requires landmark {name}"))
}

/// Move an anatomical region with its corresponding shoulder–elbow or
/// hip–knee segment before trying to shrink-wrap the surface. Gradient-only
/// projection cannot reliably find the right part of a displaced arm.
fn morph_anchors(
    neutral: &AnatomyTarget,
    target: &AnatomyTarget,
    scale: f32,
) -> Result<Vec<MorphAnchor>, String> {
    let mut anchors = Vec::with_capacity(4);
    for (start, end, radius) in [
        ("left_shoulder", "left_elbow", 0.165_f32),
        ("right_shoulder", "right_elbow", 0.165),
        ("left_hip", "left_knee", 0.190),
        ("right_hip", "right_knee", 0.190),
    ] {
        let from = landmark(neutral, start)? * scale;
        let to = landmark(neutral, end)? * scale;
        anchors.push(MorphAnchor {
            from,
            to,
            start_offset: landmark(target, start)? - from,
            end_offset: landmark(target, end)? - to,
            radius: radius * scale,
        });
    }
    Ok(anchors)
}

fn landmark_warp(point: Vec3, anchors: &[MorphAnchor]) -> Vec3 {
    let mut result = Vec3::ZERO;
    let mut best = 0.0_f32;
    for anchor in anchors {
        let axis = anchor.to - anchor.from;
        let u = ((point - anchor.from).dot(axis) / axis.length_squared())
            .clamp(0.0, 1.0);
        let closest = anchor.from + axis * u;
        let d = point.distance(closest);
        // Full influence on limb surfaces, fading into neighboring trunk.
        let t = ((d / anchor.radius - 0.52) / 0.48).clamp(0.0, 1.0);
        let weight = 1.0 - t * t * (3.0 - 2.0 * t);
        if weight > best {
            best = weight;
            result = anchor.start_offset.lerp(anchor.end_offset, u) * weight;
        }
    }
    result
}

fn surface_errors(
    points: &[Vec3], triangles: &[[u32; 3]], target: &AnatomyTarget
) -> (f32, f32) {
    let mut sum = 0.0;
    let mut worst = 0.0_f32;
    for &[a, b, c] in triangles {
        let centre = (points[a as usize] + points[b as usize] +
            points[c as usize]) / 3.0;
        let error = target.field(centre).abs();
        sum += error;
        worst = worst.max(error);
    }
    (sum / triangles.len() as f32, worst)
}

fn errors(points: &[Vec3], target: &AnatomyTarget) -> (f32,f32) {
    let mut sum = 0.0;
    let mut max_error: f32 = 0.0;
    for &p in points {
        let error = target.field(p).abs();
        sum += error;
        max_error = max_error.max(error);
    }
    (sum / points.len() as f32, max_error)
}

/// Morphology-sensitive fit with stable neutral connectivity.
///
/// Stage 1 transports neutral limb regions using source/target anatomical
/// landmarks. Stage 2 fits each vertex to a blend of neutral and requested
/// anatomy fields with *local*, rather than whole-mesh, safety backtracking.
/// Slider strength 0 means neutral; 1 means requested anatomy.
pub fn fit(recipe: &Recipe, strength: f32) -> Result<FittedCage, String> {
    if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
        return Err("Cage fitting strength must be between 0 and 1".into());
    }
    let source = neutral_template()?.clone();
    let target = AnatomyTarget::from_recipe(recipe)?;
    let neutral = AnatomyTarget::from_recipe(&Recipe::default())?;
    let scale = recipe.height / 1.75;
    let reference: Vec<Vec3> = source.vertices.iter()
        .map(|&xyz| Vec3::from_array(xyz) * scale).collect();
    if reference.len() != source.normals.len() || reference.is_empty() {
        return Err("Cage vertex normals do not match the geometry".into());
    }
    let adjacent = neighbours(&source)?;
    let incident = incident_triangles(&source)?;
    let anchors = morph_anchors(&neutral, &target, scale)?;
    let bind_limit = MAX_BIND_OFFSET * scale * strength;
    let (initial_mean_error, _) = errors(&reference, &target);
    let mut positions = reference.clone();
    let mut completed = 0;
    let mut rejected_moves = 0;

    // First transport arm/thigh vertices with their landmarks. This is
    // performed in small increments and every local move is checked against
    // the existing faces; the rest of the mesh can still move independently.
    if strength > f32::EPSILON {
        let offsets: Vec<Vec3> = reference.iter()
            .map(|&p| landmark_warp(p, &anchors) * strength).collect();
        for pass in 1..=ANCHOR_PASSES {
            let fraction = pass as f32 / ANCHOR_PASSES as f32;
            for i in 0..positions.len() {
                let destination = reference[i] + offsets[i] * fraction;
                if destination.distance(positions[i]) <= 0.0001 * scale {
                    continue;
                }
                if !apply_local(i, destination, &mut positions, &reference,
                    &source, &adjacent, &incident, bind_limit, scale)
                {
                    rejected_moves += 1;
                }
            }
        }
    }

    // Project to an interpolated anatomy field. The target signed distance
    // is the right scalar for expansion/contraction, and its gradient follows
    // the modified anatomy. A little tangential fairing evens vertex spacing.
    for _ in 0..ITERATIONS {
        if strength <= f32::EPSILON { break; }
        let before = positions.clone();
        let mut meaningful = 0_usize;
        let mut moved = 0_usize;
        for (i, &point) in before.iter().enumerate() {
            let neutral_point = point / scale;
            let blended_distance =
                (1.0 - strength) * neutral.field(neutral_point) * scale
                + strength * target.field(point);
            if blended_distance.abs() < 0.00025 * scale { continue; }
            meaningful += 1;
            let n = target.outward_normal(point);
            if !n.is_finite() || n.length_squared() < 0.25 { continue; }
            let projection = -n * blended_distance.clamp(
                -PROJECTION_STEP * scale, PROJECTION_STEP * scale
            );
            let average = adjacent[i].iter()
                .map(|&j| before[j]).sum::<Vec3>() / adjacent[i].len() as f32;
            let relaxation = average - point;
            let tangent = relaxation - n * relaxation.dot(n);
            let step = (projection + tangent * RELAX_RATE)
                .clamp_length_max(PROJECTION_STEP * scale)
                * fit_mobility(point, scale);
            let destination = positions[i] + step;
            if apply_local(i, destination, &mut positions, &reference,
                &source, &adjacent, &incident, bind_limit, scale)
            {
                moved += 1;
            } else {
                rejected_moves += 1;
            }
        }
        if meaningful == 0 || moved == 0 { break; }
        completed += 1;
    }

    // Local checks are performed for every accepted change. This final
    // pass verifies that the complete surface remained closed and oriented.
    let final_positions = positions.clone();
    let validator = SurfaceMesh {
        vertices: final_positions,
        quads: source.quads.clone(),
        triangles: source.triangles.clone(),
        caps: source.caps.clone(),
    };
    validator.validate_closed()?;

    let (mean_error,maximum_error) = errors(&positions,&target);
    let (surface_mean_error, surface_maximum_error) =
        surface_errors(&positions, &source.triangles, &target);
    let displacement = positions.iter().zip(&reference)
        .map(|(&a,&b)| a.distance(b)).fold(0.0_f32, f32::max);
    let positions_moved_count = positions.iter().zip(&reference)
        .filter(|(a,b)| a.distance(**b) > 0.0005 * scale).count();
    let surface = SurfaceMesh {
        vertices: positions,
        quads: source.quads,
        triangles: source.triangles,
        caps: source.caps,
    };
    surface.validate_closed()?;
    let normals = surface.normals();
    let model = Model {
        vertices: surface.vertices.into_iter().map(|vertex| vertex.to_array()).collect(),
        normals,
        quads: surface.quads,
        triangles: surface.triangles,
        caps: surface.caps,
        joints: Vec::new(),
        influences: Vec::new(),
    };
    Ok(FittedCage {
        model,
        report: FitReport {
            iterations: completed,
            initial_mean_error,
            mean_error,
            maximum_error,
            surface_mean_error,
            surface_maximum_error,
            maximum_displacement: displacement,
            moved_vertices: positions_moved_count,
            rejected_moves,
            neutral_flow: *neutral_flow_report()?,
            shoulder_patch: *neutral_shoulder_report()?,
        },
    })
}
