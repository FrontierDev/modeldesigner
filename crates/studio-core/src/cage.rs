//! Constrained fitting of the neutral anatomy-derived quad cage.
//!
//! The quad_template module extracts new indexed all-quad topology ONLY ONCE
//! from the neutral anatomical field. This fitter changes only vertex positions
//! for subsequent morphology recipes; no old study topology is inherited.
//! Resulting geometry has no approved UVs, skinning or production rig.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use crate::quad_template::neutral_template;
use crate::{Model, Recipe};
use glam::Vec3;

const ITERATIONS: usize = 18;
const PROJECTION_STEP: f32 = 0.018;
const MAX_BIND_OFFSET: f32 = 0.22;
const RELAX_RATE: f32 = 0.10;
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

/// Guarding the *rendered triangles* (including end caps) is important:
/// keeping quad indices unchanged is not enough to prevent face inversions.
fn safe_geometry(
    positions: &[Vec3],
    reference: &[Vec3],
    triangles: &[[u32; 3]],
    quads: &[[u32; 4]],
    scale: f32,
) -> bool {
    let minimum = MIN_AREA_SQUARED * scale.powi(4);
    for &[a,b,c] in triangles {
        let (a,b,c) = (a as usize,b as usize,c as usize);
        let current = (positions[b]-positions[a]).cross(positions[c]-positions[a]);
        let original = (reference[b]-reference[a]).cross(reference[c]-reference[a]);
        if !current.is_finite() || current.length_squared() < minimum
            || original.length_squared() < minimum
        {
            return false;
        }
        if current.dot(original)
            < MIN_FACE_ALIGNMENT * current.length() * original.length()
        {
            return false;
        }
    }
    // Projected vertices may keep faces outward-facing while stretching a
    // quad edge into a poor skinning element. Keep edge lengths within a
    // conservative envelope relative to the stable cage template.
    for &[a, b, c, d] in quads {
        for (u, v) in [(a, b), (b, c), (c, d), (d, a)] {
            let before = reference[u as usize].distance(reference[v as usize]);
            let after = positions[u as usize].distance(positions[v as usize]);
            if before <= 1.0e-7 * scale ||
                after < before * MIN_EDGE_RATIO ||
                after > before * MAX_EDGE_RATIO
            {
                return false;
            }
        }
    }
    true
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

/// Constrained fit. It projects iteratively along the implicit field's
/// normalized gradient, then applies modest *tangential* graph fairing.
/// Backtracking guards every iteration against inverted/collapsed triangles.
///
/// The neutral template is surface-derived, substantially denser than the
/// legacy cage and cached between recipes. Grid-aligned surface nets do not
/// automatically yield joint-optimized skeletal loops: fitting is still a
/// diagnostic preview, not production-quality retopology.
pub fn fit(recipe: &Recipe, strength: f32) -> Result<FittedCage, String> {
    if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
        return Err("Cage fitting strength must be between 0 and 1".into());
    }
    // Never use the old 656-quad generator here. The cached neutral quad
    // template has its own topology and is reused for every slider value.
    let source = neutral_template()?.clone();
    let target = AnatomyTarget::from_recipe(recipe)?;
    let adjacent = neighbours(&source)?;
    let scale = recipe.height / 1.75;
    let reference: Vec<Vec3> = source.vertices.iter()
        .map(|&xyz| Vec3::from_array(xyz) * scale).collect();
    if reference.len() != source.normals.len() || reference.is_empty() {
        return Err("Cage vertex normals do not match the geometry".into());
    }
    let bind_limit = MAX_BIND_OFFSET * scale * strength;
    let (initial_mean_error,_) = errors(&reference, &target);
    let mut positions = reference.clone();
    let mut completed = 0;

    // Leave the fan-cap centre points fixed. Their tiny triangles otherwise
    // collapse when a centre vertex chases a rounded implicit termination.
    let mut fixed = vec![false; reference.len()];
    for &[a,b,c] in &source.caps {
        // Fan centre is shared by many cap triangles, while rim vertices
        // are also attached to quadrilateral faces.
        for id in [a,b,c] {
            let idx = id as usize;
            if adjacent[idx].is_empty() {
                fixed[idx] = true;
            }
        }
    }

    for iteration in 0..ITERATIONS {
        if strength <= f32::EPSILON {
            break;
        }
        let before = positions.clone();
        let mut desired = before.clone();
        let mut largest_step: f32 = 0.0;
        for (i, &point) in before.iter().enumerate() {
            if fixed[i] { continue; }
            let n = target.outward_normal(point);
            if n.length_squared() < 0.25 || !n.is_finite() {
                continue;
            }
            let distance = target.field(point);
            let projection = -n * distance.clamp(
                -PROJECTION_STEP * scale, PROJECTION_STEP * scale
            );
            let tangent = if adjacent[i].is_empty() {
                Vec3::ZERO
            } else {
                let average = adjacent[i].iter().map(|&j| before[j]).sum::<Vec3>()
                    / adjacent[i].len() as f32;
                let difference = average - point;
                difference - n * difference.dot(n)
            };
            let mobility = fit_mobility(point, scale);
            let change = (projection + tangent * RELAX_RATE)
                .clamp_length_max(PROJECTION_STEP * scale) * mobility;
            let proposed = point + change;
            let offset = (proposed - reference[i]).clamp_length_max(bind_limit);
            desired[i] = reference[i] + offset;
            largest_step = largest_step.max((desired[i]-point).length());
        }
        if largest_step < 1.0e-6 * scale { break; }

        let mut accepted = false;
        for factor in [1.0_f32, 0.5, 0.25, 0.125, 0.0625, 0.03125] {
            let candidate: Vec<Vec3> = before.iter().zip(&desired)
                .map(|(&old,&new)| old.lerp(new, factor)).collect();
            if safe_geometry(
                &candidate, &reference, &source.triangles, &source.quads, scale
            ) {
                positions = candidate;
                accepted = true;
                break;
            }
        }
        if !accepted {
            // Do not show a mesh with inverted faces; report a meaningful
            // status and preserve only the already validated iterations.
            if iteration == 0 {
                return Err("Cage fit blocked by inverted or collapsed faces at the first step; revise the seed topology".into());
            }
            break;
        }
        completed += 1;
    }

    let (mean_error,maximum_error) = errors(&positions,&target);
    let (surface_mean_error, surface_maximum_error) =
        surface_errors(&positions, &source.triangles, &target);
    let displacement = positions.iter().zip(&reference)
        .map(|(&a,&b)| a.distance(b)).fold(0.0_f32, f32::max);
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
        },
    })
}
