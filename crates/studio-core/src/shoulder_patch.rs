//! Landmark-guided neutral shoulder topology surgery.
//!
//! Each accepted operation re-quadrangulates the six-sided boundary of TWO
//! adjacent quads, changing their shared edge. No vertex movement or dynamic
//! remeshing is performed here. All decisions apply only to the neutral
//! archetype, before fitting and skinning. This is an initial connectivity
//! refinement, NOT a complete concentric deltoid/axilla patch.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

const MAX_CHANGES_PER_SHOULDER: usize = 72;
const MAX_ROUNDS: usize = 5;
const MIN_TRIANGLE_AREA_SQUARED: f32 = 1.0e-12;
const MIN_OUTWARD_ALIGNMENT: f32 = 0.16;
const MAX_EDGE_ASPECT: f32 = 3.75;
const MIN_GAIN: f32 = 0.015;

#[derive(Clone, Copy, Debug, Default)]
pub struct ShoulderPatchReport {
    /// Number of genuinely replaced shared edges (quad connectivity changes).
    pub left_rewires: usize,
    pub right_rewires: usize,
    /// Weighted extraordinary-valence cost near each shoulder.
    pub hotspot_cost_before: f32,
    pub hotspot_cost_after: f32,
}

#[derive(Clone, Copy)]
struct Guide {
    shoulder: Vec3,
    elbow: Vec3,
    axilla: Vec3,
}

impl Guide {
    fn from_landmarks(target: &AnatomyTarget, left: bool) -> Result<Self, String> {
        let get = |name: &str| target.landmarks.iter().find(|p| p.name == name)
            .map(|p| p.position)
            .ok_or_else(|| format!("Missing shoulder topology landmark: {name}"));
        let shoulder = get(if left { "left_shoulder" } else { "right_shoulder" })?;
        let elbow = get(if left { "left_elbow" } else { "right_elbow" })?;
        let axis = (elbow - shoulder).normalize_or_zero();
        if axis.length_squared() < 0.9 {
            return Err("Shoulder topology guide has an invalid arm axis".into());
        }
        // The axilla is medial and inferior to the glenohumeral pivot.
        let medial = if left { Vec3::X } else { -Vec3::X };
        let axilla = shoulder + medial * (0.068 * target.scale)
            - Vec3::Y * (0.085 * target.scale);
        Ok(Self { shoulder, elbow, axilla })
    }

    fn in_region(&self, p: Vec3, scale: f32) -> bool {
        let axis = self.elbow - self.shoulder;
        let t = (p - self.shoulder).dot(axis) / axis.length_squared();
        let closest = self.shoulder + axis * t.clamp(0.0, 0.50);
        (-0.33..=0.54).contains(&t)
            && p.distance(closest) <= 0.195 * scale
    }

    fn hotspot_weight(&self, p: Vec3, scale: f32) -> f32 {
        let d = p.distance(self.axilla) / (0.205 * scale);
        1.0 + 4.0 * (1.0 - d.clamp(0.0, 1.0)).powi(2)
    }

    fn alignment_error(&self, a: Vec3, b: Vec3) -> f32 {
        let axis = (self.elbow - self.shoulder).normalize();
        let segment = (b - a).normalize_or_zero();
        let axial = segment.dot(axis).powi(2).clamp(0.0, 1.0);
        // Good edge flow is axial or circumferential, not diagonal.
        axial.min(1.0 - axial)
    }
}

type Edge = (u32, u32);
fn edge(a: u32, b: u32) -> Edge { (a.min(b), a.max(b)) }

fn directed_edges(mesh: &SurfaceMesh) -> BTreeMap<Edge, Vec<usize>> {
    let mut out = BTreeMap::<Edge, Vec<usize>>::new();
    for (face, [a,b,c,d]) in mesh.quads.iter().copied().enumerate() {
        for (u,v) in [(a,b),(b,c),(c,d),(d,a)] {
            out.entry(edge(u,v)).or_default().push(face);
        }
    }
    out
}

fn rotate(face: [u32;4], a: u32, b: u32) -> Option<[u32;4]> {
    (0..4).find(|&i| face[i] == a && face[(i+1)%4] == b)
        .map(|i| [
            face[i], face[(i+1)%4],
            face[(i+2)%4], face[(i+3)%4]
        ])
}

fn quad_triangles([a,b,c,d]: [u32;4]) -> [[u32;3];2] {
    [[a,b,c],[a,c,d]]
}

fn area_normal(vertices: &[Vec3], [a,b,c]: [u32;3]) -> Vec3 {
    (vertices[b as usize] - vertices[a as usize])
        .cross(vertices[c as usize] - vertices[a as usize])
}

fn valid_quads(
    vertices: &[Vec3], faces: &[[u32;4];2],
    target: &AnatomyTarget,
    median: f32,
) -> bool {
    for &face in faces {
        let mut min_edge = f32::INFINITY;
        let mut max_edge = 0.0_f32;
        for i in 0..4 {
            let length = vertices[face[i] as usize]
                .distance(vertices[face[(i+1)%4] as usize]);
            min_edge = min_edge.min(length);
            max_edge = max_edge.max(length);
        }
        if min_edge < median * 0.23 || max_edge > median * 2.1
            || max_edge / min_edge > MAX_EDGE_ASPECT
        {
            return false;
        }
        for triangle in quad_triangles(face) {
            let normal = area_normal(vertices, triangle);
            if !normal.is_finite()
                || normal.length_squared() < MIN_TRIANGLE_AREA_SQUARED
            {
                return false;
            }
            let centre = triangle.into_iter()
                .map(|id| vertices[id as usize]).sum::<Vec3>() / 3.0;
            let outward = target.outward_normal(centre);
            if outward.length_squared() < 0.5
                || normal.normalize().dot(outward) < MIN_OUTWARD_ALIGNMENT
            {
                return false;
            }
        }
    }
    true
}

fn pole_energy(
    valence: &[usize], vertex: u32, delta: i32,
    vertices: &[Vec3], guide: &Guide, scale: f32,
) -> f32 {
    let count = (valence[vertex as usize] as i32 + delta - 4) as f32;
    guide.hotspot_weight(vertices[vertex as usize], scale) * count * count
}

fn score_change(
    old: Edge, new: Edge, valence: &[usize],
    vertices: &[Vec3], guide: &Guide, median: f32, scale: f32,
) -> f32 {
    let previous = pole_energy(valence, old.0, 0, vertices, guide, scale)
        + pole_energy(valence, old.1, 0, vertices, guide, scale)
        + pole_energy(valence, new.0, 0, vertices, guide, scale)
        + pole_energy(valence, new.1, 0, vertices, guide, scale);
    let subsequent = pole_energy(valence, old.0, -1, vertices, guide, scale)
        + pole_energy(valence, old.1, -1, vertices, guide, scale)
        + pole_energy(valence, new.0, 1, vertices, guide, scale)
        + pole_energy(valence, new.1, 1, vertices, guide, scale);
    let old_a = vertices[old.0 as usize];
    let old_b = vertices[old.1 as usize];
    let new_a = vertices[new.0 as usize];
    let new_b = vertices[new.1 as usize];
    let alignment = 1.6 * (guide.alignment_error(new_a,new_b)
        - guide.alignment_error(old_a,old_b));
    let old_length = old_a.distance(old_b) / median;
    let new_length = new_a.distance(new_b) / median;
    let strain = 0.35 * ((new_length-1.0).powi(2) - (old_length-1.0).powi(2));
    // Positive result means the replacement is a measurable improvement.
    previous - subsequent - alignment - strain
}

/// In-place neutral topology surgery. Only indexed quad edges are changed.
/// Any rejected candidate leaves the input untouched.
pub(crate) fn apply(
    mesh: &mut SurfaceMesh, target: &AnatomyTarget,
) -> Result<ShoulderPatchReport, String> {
    let guides = [
        Guide::from_landmarks(target, true)?,
        Guide::from_landmarks(target, false)?,
    ];
    let before_faces = mesh.quads.len();
    let before_vertices = mesh.vertices.len();
    let mut report = ShoulderPatchReport::default();
    report.hotspot_cost_before = hotspot_cost(mesh, &guides, target.scale);
    for (side, guide) in guides.iter().enumerate() {
        let mut changed = 0_usize;
        for _ in 0..MAX_ROUNDS {
            let mut accepted = 0;
            // Rebuild the edge graph after each edit; a previous candidate's
            // edge neighborhood must never be reused with stale face indices.
            loop {
                if changed >= MAX_CHANGES_PER_SHOULDER { break; }
                let incidence = directed_edges(mesh);
                if incidence.values().any(|faces| faces.len() != 2) {
                    return Err("Shoulder rewire requires a closed quad manifold".into());
                }
                let mut valence = vec![0_usize; mesh.vertices.len()];
                for &(u,v) in incidence.keys() {
                    valence[u as usize] += 1;
                    valence[v as usize] += 1;
                }

                let mut best: Option<(f32, usize, usize, [[u32;4];2])> = None;
                for (&shared, faces) in &incidence {
                    let midpoint = (mesh.vertices[shared.0 as usize]
                        + mesh.vertices[shared.1 as usize]) * 0.5;
                    if !guide.in_region(midpoint, target.scale) { continue; }
                    let fa = faces[0];
                    let fb = faces[1];
                    let Some([u,v,a,b]) = rotate(mesh.quads[fa], shared.0, shared.1)
                        .or_else(|| rotate(mesh.quads[fa],shared.1,shared.0)) else { continue; };
                    let Some([_,_,c,d]) = rotate(mesh.quads[fb], v, u) else { continue; };
                    if [u,v,a,b,c,d].into_iter().collect::<BTreeSet<_>>().len() != 6 {
                        continue;
                    }
                    // The two old quads enclose this oriented hexagon:
                    // u-c-d-v-a-b. Both alternatives preserve its exact
                    // boundary and therefore are watertight by construction.
                    let perimeter = [u,c,d,v,a,b];
                    let lengths: Vec<f32> = (0..6).map(|i| {
                        mesh.vertices[perimeter[i] as usize]
                            .distance(mesh.vertices[perimeter[(i+1)%6] as usize])
                    }).collect();
                    let median = {
                        let mut sorted = lengths;
                        sorted.sort_by(|a,b| a.total_cmp(b));
                        (sorted[2]+sorted[3])*0.5
                    };
                    if median < 1.0e-5 * target.scale { continue; }
                    for (diagonal, candidate) in [
                        (edge(c,a), [[c,d,v,a],[a,b,u,c]]),
                        (edge(d,b), [[d,v,a,b],[b,u,c,d]]),
                    ] {
                        if incidence.contains_key(&diagonal) { continue; }
                        if !guide.in_region(
                            (mesh.vertices[diagonal.0 as usize] +
                             mesh.vertices[diagonal.1 as usize]) * 0.5,
                            target.scale
                        ) { continue; }
                        if !valid_quads(&mesh.vertices, &candidate, target, median) {
                            continue;
                        }
                        let gain = score_change(shared, diagonal, &valence,
                            &mesh.vertices, guide, median, target.scale);
                        if gain < MIN_GAIN { continue; }
                        if best.as_ref().is_none_or(|item| gain > item.0) {
                            best = Some((gain, fa, fb, candidate));
                        }
                    }
                }
                let Some((_, fa, fb, patch)) = best else { break; };
                mesh.quads[fa] = patch[0];
                mesh.quads[fb] = patch[1];
                accepted += 1;
                changed += 1;
                // Fix preview triangles after every pair swap; subsequent
                // validation always sees the current actual topology.
                for (face, quad) in [(fa,patch[0]),(fb,patch[1])] {
                    let tris = quad_triangles(quad);
                    mesh.triangles[2*face] = tris[0];
                    mesh.triangles[2*face+1] = tris[1];
                }
            }
            if accepted == 0 { break; }
        }
        if side == 0 { report.left_rewires = changed; }
        else { report.right_rewires = changed; }
    }
    if mesh.vertices.len() != before_vertices || mesh.quads.len() != before_faces
        || mesh.triangles.len() != 2 * before_faces || !mesh.caps.is_empty()
    {
        return Err("Shoulder patch changed geometry counts unexpectedly".into());
    }
    mesh.validate_closed()?;
    report.hotspot_cost_after = hotspot_cost(mesh, &guides, target.scale);
    Ok(report)
}

fn hotspot_cost(mesh: &SurfaceMesh, guides: &[Guide;2], scale: f32) -> f32 {
    let edges = directed_edges(mesh);
    let mut degree = vec![0_usize; mesh.vertices.len()];
    for &(a,b) in edges.keys() {
        degree[a as usize] += 1;
        degree[b as usize] += 1;
    }
    mesh.vertices.iter().enumerate().filter_map(|(i,&p)| {
        let mut weight = 0.0_f32;
        for guide in guides {
            if guide.in_region(p,scale) {
                weight = weight.max(guide.hotspot_weight(p,scale));
            }
        }
        if weight == 0.0 { None }
        else { Some(weight * (degree[i] as f32 - 4.0).powi(2)) }
    }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_sided_quad_exchange_preserves_boundary_orientation() {
        // A regular oriented hexagon initially divided along u-v.
        let v: Vec<Vec3> = (0..6).map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 6.0;
            Vec3::new(angle.cos(),angle.sin(),0.0)
        }).collect();
        let old = [[0,3,4,5],[3,0,1,2]];
        let new = [[1,2,3,4],[4,5,0,1]];
        let boundary = |faces: &[[u32;4];2]| {
            let mut counts = BTreeMap::<Edge,usize>::new();
            for face in faces {
                for i in 0..4 {
                    *counts.entry(edge(face[i],face[(i+1)%4])).or_default() += 1;
                }
            }
            counts.into_iter().filter_map(|(k,n)| if n==1 {Some(k)} else {None})
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(boundary(&old),boundary(&new));
        for faces in [&old,&new] {
            for &face in faces {
                for tri in quad_triangles(face) {
                    assert!(area_normal(&v,tri).z > 0.0);
                }
            }
        }
    }
}
