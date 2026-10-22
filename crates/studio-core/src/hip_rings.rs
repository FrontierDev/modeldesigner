//! Anatomical upper-thigh/groin quad patch on the cached neutral cage.
//!
//! The leg side of each annular cut is kept topologically distinct from the
//! pelvis. Two oriented circumferential support rings replace automatic
//! surface-net quads; both the shoulder and hip ownership maps are remapped
//! whenever unused vertices are compacted. No topology changes during fit
//! or rigging previews.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use crate::shoulder_rings::{
    adjacency, band_faces, boundary_loops, bridge, distal_vertices,
    rotate_to_front, sample_ownership, support_ring, valid_new_quads,
    Guide, MAX_RING, MIN_RING,
};
use glam::Vec3;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default)]
pub struct HipRingReport {
    pub left_removed_quads: usize,
    pub right_removed_quads: usize,
    pub left_patch_quads: usize,
    pub right_patch_quads: usize,
    pub left_pelvis_ring: usize,
    pub right_pelvis_ring: usize,
    pub left_thigh_ring: usize,
    pub right_thigh_ring: usize,
}

#[derive(Clone, Copy)]
struct Counts {
    removed: usize,
    created: usize,
    pelvis: usize,
    thigh: usize,
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Fade the trochanter and lateral gluteal surface into the pelvis. The
/// medial groin stays pelvis-owned until reaching the structured thigh band.
fn proximal_ownership(p: Vec3, guide: Guide, left: bool) -> f32 {
    let outward = if left { -1.0 } else { 1.0 };
    let lateral = outward * (p.x - guide.shoulder.x) / guide.scale;
    let radial = guide.radius(p) / guide.scale;
    let axial = guide.t(p);
    smooth(-0.085, 0.025, lateral)
        * (1.0 - smooth(0.15, 0.21, radial))
        * smooth(-0.55, -0.05, axial)
}

/// Indices are stable after this compaction; preserve every ownership field
/// alongside the corresponding vertex when removing orphaned automatic quads.
fn compact(
    mesh: &mut SurfaceMesh,
    shoulders: &mut Vec<[f32; 2]>, hips: &mut Vec<[f32; 2]>,
) {
    assert_eq!(mesh.vertices.len(), shoulders.len());
    assert_eq!(mesh.vertices.len(), hips.len());
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    let mut vertices = Vec::new();
    let mut shoulder_guides = Vec::new();
    let mut hip_guides = Vec::new();
    for quad in &mut mesh.quads {
        for id in quad {
            let old = *id as usize;
            if remap[old] == u32::MAX {
                remap[old] = vertices.len() as u32;
                vertices.push(mesh.vertices[old]);
                shoulder_guides.push(shoulders[old]);
                hip_guides.push(hips[old]);
            }
            *id = remap[old];
        }
    }
    mesh.vertices = vertices;
    *shoulders = shoulder_guides;
    *hips = hip_guides;
    mesh.triangles.clear();
    for &[a, b, c, d] in &mesh.quads {
        mesh.triangles.extend([[a, b, c], [a, c, d]]);
    }
}

fn replace_band(
    source: &SurfaceMesh, selected: &[bool], mut loops: Vec<Vec<u32>>,
    guide: Guide, target: &AnatomyTarget,
    shoulders: &[[f32; 2]], hips: &[[f32; 2]], side: usize,
) -> Option<(SurfaceMesh, Vec<[f32; 2]>, Vec<[f32; 2]>, Counts)> {
    if loops.len() != 2 { return None; }
    let mean_t = |ring: &[u32]| ring.iter()
        .map(|&id| guide.t(source.vertices[id as usize])).sum::<f32>()
        / ring.len() as f32;
    loops.sort_by(|a, b| mean_t(a).total_cmp(&mean_t(b)));
    let mut thigh = loops.pop()?;
    let mut pelvis = loops.pop()?;
    let (m, n) = (pelvis.len(), thigh.len());
    if !(MIN_RING..=MAX_RING).contains(&m)
        || !(MIN_RING..=MAX_RING).contains(&n)
        || m.abs_diff(n) > 20 || (m + n) % 2 != 0
        || m.abs_diff(n) / 2 >= m.min(n)
        || mean_t(&thigh) - mean_t(&pelvis) < 0.20
    { return None; }
    thigh.reverse();
    rotate_to_front(&mut pelvis, source, guide);
    rotate_to_front(&mut thigh, source, guide);

    let distal = distal_vertices(source, selected, &thigh, &pelvis)?;
    let mut base_hips = hips.to_vec();
    for ((&p, label), is_distal) in source.vertices.iter()
        .zip(&mut base_hips).zip(distal)
    {
        label[side] = if is_distal { 1.0 }
            else { proximal_ownership(p, guide, side == 0) };
    }

    let retained = SurfaceMesh {
        vertices: source.vertices.clone(),
        quads: source.quads.iter().enumerate().filter_map(|(i, &quad)|
            (!selected[i]).then_some(quad)).collect(),
        triangles: Vec::new(),
        caps: Vec::new(),
    };
    let start = retained.quads.len();
    let removed = selected.iter().filter(|&&v| v).count();
    let count = m.min(n);
    let phase_offsets = (0..=8).flat_map(|distance| {
        if distance == 0 { vec![0] }
        else { vec![distance, n - distance] }
    });
    for shift in phase_offsets {
        let mut mesh = retained.clone();
        let mut candidate_shoulders = shoulders.to_vec();
        let mut candidate_hips = base_hips.clone();
        let mut phased_thigh = thigh.clone();
        phased_thigh.rotate_left(shift);
        let mut rings = Vec::new();
        for alpha in [0.32_f32, 0.68] {
            let Some(ring) = support_ring(
                &mut mesh, &pelvis, &phased_thigh, count, alpha, target
            ) else { break; };
            candidate_shoulders.extend(vec![[0.0; 2]; ring.len()]);
            candidate_hips.extend((0..ring.len()).map(|i| {
                let u = i as f32 / count as f32;
                let proximal = sample_ownership(&pelvis, &source.vertices,
                    &base_hips, side, u);
                let distal = sample_ownership(&phased_thigh, &source.vertices,
                    &base_hips, side, u);
                let mut label = [0.0; 2];
                label[side] = proximal + (distal - proximal) * alpha;
                label
            }));
            rings.push(ring);
        }
        if rings.len() != 2 { continue; }
        let connected = bridge(&mut mesh, &pelvis, &rings[0], guide.axilla).is_some()
            && bridge(&mut mesh, &rings[0], &rings[1], guide.axilla).is_some()
            && bridge(&mut mesh, &rings[1], &phased_thigh, guide.axilla).is_some();
        if !connected || !valid_new_quads(&mesh, start, target) { continue; }
        let created = mesh.quads.len() - start;
        compact(&mut mesh, &mut candidate_shoulders, &mut candidate_hips);
        if mesh.validate_closed().is_err() { continue; }
        let mut seen = BTreeSet::new();
        if mesh.quads.iter().all(|quad| {
            if quad.iter().copied().collect::<BTreeSet<_>>().len() != 4 {
                return false;
            }
            let mut ids = *quad;
            ids.sort_unstable();
            seen.insert(ids)
        }) {
            return Some((mesh, candidate_shoulders, candidate_hips,
                Counts { removed, created, pelvis: m, thigh: n }));
        }
    }
    None
}

fn one_side(
    mesh: &mut SurfaceMesh, shoulders: &mut Vec<[f32; 2]>,
    hips: &mut Vec<[f32; 2]>, target: &AnatomyTarget, left: bool,
) -> Result<Counts, String> {
    let guide = Guide::hip(target, left)?;
    let (adjacent, uses) = adjacency(mesh)?;
    let mut bands = 0;
    let mut boundaries = 0;
    // The first neutral cut has near-circular 34/32-vertex thigh loops;
    // wider/proximal candidates are retained for future cage resolution changes.
    for start in [0.55, 0.49, 0.43, 0.37, 0.31, 0.25, 0.19, 0.13] {
        for end in [0.87, 0.80, 0.92, 0.72, 0.64] {
            for radius in [0.145, 0.165, 0.185, 0.215] {
                let Some(selected) = band_faces(
                    mesh, guide, &adjacent, start, end, radius
                ) else { continue; };
                bands += 1;
                let Some(loops) = boundary_loops(&selected, &uses) else { continue; };
                if loops.len() == 2 { boundaries += 1; }
                if let Some((candidate, candidate_shoulders, candidate_hips, counts)) =
                    replace_band(mesh, &selected, loops, guide, target,
                        shoulders, hips, if left { 0 } else { 1 })
                {
                    *mesh = candidate;
                    *shoulders = candidate_shoulders;
                    *hips = candidate_hips;
                    return Ok(counts);
                }
            }
        }
    }
    Err(format!(
        "No safe {} hip/groin boundary pair for the structured quad patch \
         ({bands} regions, {boundaries} two-loop boundaries)",
        if left { "left" } else { "right" },
    ))
}

/// Construct and validate the left hip before applying the same procedure
/// on the right. If either side fails, preserve the input cage and guides.
pub(crate) fn apply(
    mesh: &mut SurfaceMesh, shoulders: &mut Vec<[f32; 2]>,
    target: &AnatomyTarget,
) -> Result<(HipRingReport, Vec<[f32; 2]>), String> {
    if shoulders.len() != mesh.vertices.len() {
        return Err("Hip patch requires shoulder guides aligned with the cage".into());
    }
    let mut candidate = mesh.clone();
    let mut shoulder_labels = shoulders.clone();
    let mut hip_labels = vec![[0.0; 2]; candidate.vertices.len()];
    let left = one_side(&mut candidate, &mut shoulder_labels,
        &mut hip_labels, target, true)?;
    candidate.validate_closed()?;
    let right = one_side(&mut candidate, &mut shoulder_labels,
        &mut hip_labels, target, false)?;
    candidate.validate_closed()?;
    if candidate.vertices.len() != shoulder_labels.len()
        || candidate.vertices.len() != hip_labels.len()
    {
        return Err("Hip patch ownership was not welded with the cage".into());
    }
    *mesh = candidate;
    *shoulders = shoulder_labels;
    Ok((HipRingReport {
        left_removed_quads: left.removed,
        right_removed_quads: right.removed,
        left_patch_quads: left.created,
        right_patch_quads: right.created,
        left_pelvis_ring: left.pelvis,
        right_pelvis_ring: right.pelvis,
        left_thigh_ring: left.thigh,
        right_thigh_ring: right.thigh,
    }, hip_labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_template_has_closed_bilateral_hip_loops() {
        let model = crate::quad_template::neutral_template().unwrap();
        let report = crate::quad_template::neutral_hip_ring_report().unwrap();
        assert!(report.left_patch_quads > 0 && report.right_patch_quads > 0);
        assert!(report.left_pelvis_ring >= MIN_RING && report.right_pelvis_ring >= MIN_RING);
        assert!(report.left_thigh_ring >= MIN_RING && report.right_thigh_ring >= MIN_RING);
        assert_eq!(model.vertices.len(), model.hip_guides.len());
        assert_eq!(model.vertices.len(), model.shoulder_guides.len());
        assert_eq!(model.triangles.len(), model.quads.len() * 2);
        for side in 0..2 {
            assert!(model.hip_guides.iter().any(|g| g[side] == 1.0));
            assert!(model.hip_guides.iter().any(|g| (0.05..0.95).contains(&g[side])));
        }
    }
}
