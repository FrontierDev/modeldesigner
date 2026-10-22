//! Structured, anatomy-guided neutral shoulder topology.
//!
//! Remove a connected annular shoulder band, preserve its two interface
//! loops and replace its surface-net connectivity with circumferential
//! support rings. This runs only while constructing the cached neutral cage.

use crate::anatomy::AnatomyTarget;
use crate::mesh::SurfaceMesh;
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

type Edge = (u32, u32);
type EdgeUse = (usize, u32, u32);
const MIN_PATCH_FACES: usize = 32;
const MIN_RING: usize = 8;
const MAX_RING: usize = 64;

#[derive(Clone, Copy, Debug, Default)]
pub struct RingPatchReport {
    pub left_removed_quads: usize,
    pub right_removed_quads: usize,
    pub left_patch_quads: usize,
    pub right_patch_quads: usize,
    pub left_torso_ring: usize,
    pub right_torso_ring: usize,
    pub left_arm_ring: usize,
    pub right_arm_ring: usize,
}

#[derive(Clone, Copy)]
struct Guide {
    shoulder: Vec3,
    axis: Vec3,
    axilla: Vec3,
    scale: f32,
}

impl Guide {
    fn new(target: &AnatomyTarget, left: bool) -> Result<Self, String> {
        let landmark = |name: &str| target.landmarks.iter()
            .find(|p| p.name == name).map(|p| p.position)
            .ok_or_else(|| format!("Missing shoulder topology landmark: {name}"));
        let shoulder = landmark(if left { "left_shoulder" } else { "right_shoulder" })?;
        let elbow = landmark(if left { "left_elbow" } else { "right_elbow" })?;
        let axis = elbow - shoulder;
        if !axis.is_finite() || axis.length_squared() < 0.01 * target.scale.powi(2) {
            return Err("Shoulder topology has an invalid arm axis".into());
        }
        let medial = if left { Vec3::X } else { -Vec3::X };
        let axilla = shoulder + medial * (0.068 * target.scale)
            - Vec3::Y * (0.085 * target.scale);
        Ok(Self { shoulder, axis, axilla, scale: target.scale })
    }

    fn t(self, p: Vec3) -> f32 {
        (p - self.shoulder).dot(self.axis) / self.axis.length_squared()
    }

    fn radius(self, p: Vec3) -> f32 {
        p.distance(self.shoulder + self.axis * self.t(p))
    }

    fn front(self) -> Vec3 {
        let axis = self.axis.normalize();
        (Vec3::Z - axis * axis.dot(Vec3::Z)).normalize()
    }
}

fn edge(a: u32, b: u32) -> Edge { (a.min(b), a.max(b)) }

fn edge_uses(mesh: &SurfaceMesh) -> BTreeMap<Edge, Vec<EdgeUse>> {
    let mut uses = BTreeMap::<Edge, Vec<EdgeUse>>::new();
    for (face, quad) in mesh.quads.iter().enumerate() {
        for i in 0..4 {
            let a = quad[i];
            let b = quad[(i + 1) % 4];
            uses.entry(edge(a, b)).or_default().push((face, a, b));
        }
    }
    uses
}

fn adjacency(mesh: &SurfaceMesh) -> Result<(Vec<Vec<usize>>, BTreeMap<Edge, Vec<EdgeUse>>), String> {
    let uses = edge_uses(mesh);
    let mut adjacent = vec![Vec::new(); mesh.quads.len()];
    for incidence in uses.values() {
        if incidence.len() != 2 {
            return Err("Shoulder patch requires a closed quad manifold".into());
        }
        let (a, b) = (incidence[0].0, incidence[1].0);
        adjacent[a].push(b);
        adjacent[b].push(a);
    }
    Ok((adjacent, uses))
}

/// Keep only the connected arm component within an anatomical cylinder.
fn band_faces(
    mesh: &SurfaceMesh, guide: Guide, adjacent: &[Vec<usize>],
    start: f32, end: f32, radius: f32,
) -> Option<Vec<bool>> {
    let mut eligible = vec![false; mesh.quads.len()];
    let mut seed = None;
    let mut best = f32::INFINITY;
    for (i, quad) in mesh.quads.iter().enumerate() {
        let p = quad.iter().map(|&id| mesh.vertices[id as usize]).sum::<Vec3>() / 4.0;
        let t = guide.t(p);
        let radial = guide.radius(p);
        if !(start..=end).contains(&t) || radial > radius * guide.scale { continue; }
        eligible[i] = true;
        let score = (t - 0.5 * (start + end)).abs() * 0.08 * guide.scale + radial;
        if score < best { best = score; seed = Some(i); }
    }
    let first = seed?;
    let mut selected = vec![false; eligible.len()];
    selected[first] = true;
    let mut stack = vec![first];
    let mut count = 0;
    let mut axillary_faces = 0;
    while let Some(face) = stack.pop() {
        count += 1;
        let centre = mesh.quads[face].iter()
            .map(|&id| mesh.vertices[id as usize]).sum::<Vec3>() / 4.0;
        if centre.distance(guide.axilla) < 0.15 * guide.scale {
            axillary_faces += 1;
        }
        for &next in &adjacent[face] {
            if eligible[next] && !selected[next] {
                selected[next] = true;
                stack.push(next);
            }
        }
    }
    (count >= MIN_PATCH_FACES && axillary_faces >= 4).then_some(selected)
}

/// Boundary half-edges point in the removed face's original winding.
/// That winding must be reused by the replacement, opposing retained faces.
fn boundary_loops(
    selected: &[bool], uses: &BTreeMap<Edge, Vec<EdgeUse>>,
) -> Option<Vec<Vec<u32>>> {
    let mut next = BTreeMap::<u32, u32>::new();
    let mut incoming = BTreeMap::<u32, usize>::new();
    for incidence in uses.values() {
        if incidence.len() != 2 { return None; }
        let (a, b) = (incidence[0], incidence[1]);
        if selected[a.0] == selected[b.0] { continue; }
        let (_, from, to) = if selected[a.0] { a } else { b };
        if next.insert(from, to).is_some() { return None; }
        *incoming.entry(to).or_default() += 1;
    }
    if next.is_empty() || next.len() != incoming.len()
        || incoming.values().any(|&count| count != 1)
        || next.keys().any(|id| !incoming.contains_key(id))
    { return None; }
    let mut unseen = next.keys().copied().collect::<BTreeSet<_>>();
    let mut loops = Vec::new();
    while let Some(&start) = unseen.iter().next() {
        let mut ring = Vec::new();
        let mut vertex = start;
        loop {
            if !unseen.remove(&vertex) { return None; }
            ring.push(vertex);
            vertex = *next.get(&vertex)?;
            if vertex == start { break; }
        }
        loops.push(ring);
    }
    Some(loops)
}

fn rotate_to_front(ring: &mut [u32], mesh: &SurfaceMesh, guide: Guide) {
    let forward = guide.front();
    let (index, _) = ring.iter().enumerate().map(|(index, &id)| {
        let p = mesh.vertices[id as usize];
        let centre = guide.shoulder + guide.axis * guide.t(p);
        (index, (p - centre).dot(forward))
    }).max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    ring.rotate_left(index);
}

fn sample(ring: &[u32], vertices: &[Vec3], fraction: f32) -> Vec3 {
    let lengths: Vec<f32> = (0..ring.len()).map(|i| {
        vertices[ring[i] as usize]
            .distance(vertices[ring[(i + 1) % ring.len()] as usize])
    }).collect();
    let mut remaining = lengths.iter().sum::<f32>() * fraction;
    for (i, &length) in lengths.iter().enumerate() {
        if remaining <= length || i == ring.len() - 1 {
            return vertices[ring[i] as usize].lerp(
                vertices[ring[(i + 1) % ring.len()] as usize],
                (remaining / length.max(f32::EPSILON)).clamp(0.0, 1.0),
            );
        }
        remaining -= length;
    }
    vertices[ring[0] as usize]
}

fn project(target: &AnatomyTarget, mut p: Vec3) -> Option<Vec3> {
    for _ in 0..8 {
        let normal = target.outward_normal(p);
        let distance = target.field(p);
        if !normal.is_finite() || normal.length_squared() < 0.5 || !distance.is_finite() {
            return None;
        }
        p -= normal * distance.clamp(-0.018 * target.scale, 0.018 * target.scale);
    }
    (p.is_finite() && target.field(p).abs() <= 0.009 * target.scale)
        .then_some(p)
}

fn support_ring(
    mesh: &mut SurfaceMesh, torso: &[u32], arm: &[u32],
    count: usize, alpha: f32, target: &AnatomyTarget,
) -> Option<Vec<u32>> {
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        let u = i as f32 / count as f32;
        let from = sample(torso, &mesh.vertices, u);
        let to = sample(arm, &mesh.vertices, u);
        let point = project(target, from.lerp(to, alpha))?;
        ids.push(mesh.vertices.len() as u32);
        mesh.vertices.push(point);
    }
    Some(ids)
}

fn push_quad(mesh: &mut SurfaceMesh, [a, b, c, d]: [u32; 4]) {
    mesh.quads.push([a, b, c, d]);
    mesh.triangles.extend([[a, b, c], [a, c, d]]);
}

/// Keep the unavoidable count-transition poles in the least axillary part
/// of the support band, while maintaining approximately even spacing.
fn transition_slots(
    mesh: &SurfaceMesh, shorter: &[u32], extras: usize, axilla: Vec3,
) -> BTreeSet<usize> {
    if extras == 0 { return BTreeSet::new(); }
    let mut chosen = BTreeSet::new();
    let mut best = f32::NEG_INFINITY;
    for phase in 0..shorter.len() {
        let slots = (0..extras).map(|k|
            ((k + 1) * shorter.len() / (extras + 1) + phase) % shorter.len())
            .collect::<BTreeSet<_>>();
        if slots.len() != extras { continue; }
        let clearance = slots.iter().map(|&i|
            mesh.vertices[shorter[i] as usize].distance(axilla)).sum::<f32>();
        if clearance > best {
            chosen = slots;
            best = clearance;
        }
    }
    chosen
}

/// Winding-preserving quad zipper. Boundary sizes may differ by an even
/// number; pairs of edges absorb the difference in the support bands.
fn bridge(mesh: &mut SurfaceMesh, outer: &[u32], inner: &[u32], axilla: Vec3) -> Option<()> {
    let (m, n) = (outer.len(), inner.len());
    if m < 4 || n < 4 || (m + n) % 2 != 0 { return None; }
    let (smaller, extras) = (m.min(n), m.abs_diff(n) / 2);
    if extras >= smaller { return None; }
    let slots = transition_slots(
        mesh, if m <= n { outer } else { inner }, extras, axilla
    );
    let mut i = 0;
    let mut j = 0;
    if m <= n {
        for step in 0..m {
            if slots.contains(&step) {
                push_quad(mesh, [outer[i % m], inner[(j + 2) % n],
                    inner[(j + 1) % n], inner[j % n]]);
                j += 2;
            }
            push_quad(mesh, [outer[i % m], outer[(i + 1) % m],
                inner[(j + 1) % n], inner[j % n]]);
            i += 1;
            j += 1;
        }
    } else {
        for step in 0..n {
            if slots.contains(&step) {
                push_quad(mesh, [outer[i % m], outer[(i + 1) % m],
                    outer[(i + 2) % m], inner[j % n]]);
                i += 2;
            }
            push_quad(mesh, [outer[i % m], outer[(i + 1) % m],
                inner[(j + 1) % n], inner[j % n]]);
            i += 1;
            j += 1;
        }
    }
    (i == m && j == n).then_some(())
}

/// Remove vertices stranded by deleted automatic faces and rebuild render
/// triangles from the welded quad surface. Indices are fixed from this point.
fn compact(mesh: &mut SurfaceMesh, guides: &mut Vec<[f32; 2]>) {
    assert_eq!(mesh.vertices.len(), guides.len());
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    let mut positions = Vec::new();
    let mut preserved_guides = Vec::new();
    for quad in &mut mesh.quads {
        for id in quad {
            let old = *id as usize;
            if remap[old] == u32::MAX {
                remap[old] = positions.len() as u32;
                positions.push(mesh.vertices[old]);
                preserved_guides.push(guides[old]);
            }
            *id = remap[old];
        }
    }
    mesh.vertices = positions;
    *guides = preserved_guides;
    mesh.triangles.clear();
    for &[a, b, c, d] in &mesh.quads {
        mesh.triangles.extend([[a, b, c], [a, c, d]]);
    }
}

/// Cutting the shoulder annulus separates the retained arm surface from the
/// retained trunk. Flood-fill the distal side before welding the new rings;
/// spatial proximity alone cannot distinguish the two around the armpit.
fn distal_vertices(
    mesh: &SurfaceMesh, selected: &[bool], arm: &[u32], torso: &[u32],
) -> Option<Vec<bool>> {
    let mut neighbours = vec![Vec::new(); mesh.vertices.len()];
    for (face, quad) in mesh.quads.iter().enumerate() {
        if selected[face] { continue; }
        for i in 0..4 {
            let a = quad[i] as usize;
            let b = quad[(i + 1) % 4] as usize;
            neighbours[a].push(b);
            neighbours[b].push(a);
        }
    }
    let mut distal = vec![false; mesh.vertices.len()];
    let mut stack = vec![arm[0] as usize];
    distal[arm[0] as usize] = true;
    while let Some(vertex) = stack.pop() {
        for &next in &neighbours[vertex] {
            if !distal[next] {
                distal[next] = true;
                stack.push(next);
            }
        }
    }
    (arm.iter().all(|&id| distal[id as usize])
        && torso.iter().all(|&id| !distal[id as usize]))
        .then_some(distal)
}

fn valid_new_quads(
    mesh: &SurfaceMesh, start: usize, target: &AnatomyTarget,
) -> bool {
    for &quad in &mesh.quads[start..] {
        let mut shortest = f32::INFINITY;
        let mut longest = 0.0_f32;
        for i in 0..4 {
            let length = mesh.vertices[quad[i] as usize]
                .distance(mesh.vertices[quad[(i + 1) % 4] as usize]);
            shortest = shortest.min(length);
            longest = longest.max(length);
        }
        if shortest <= 0.003 * target.scale || longest / shortest > 6.0 {
            return false;
        }
        for ids in [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]] {
            let [a, b, c] = ids.map(|id| mesh.vertices[id as usize]);
            let normal = (b - a).cross(c - a);
            let centre = (a + b + c) / 3.0;
            let outward = target.outward_normal(centre);
            if !normal.is_finite() || normal.length_squared() < 1.0e-12
                || !outward.is_finite() || outward.length_squared() < 0.5
                || normal.normalize().dot(outward) < 0.045
                || target.field(centre).abs() > 0.036 * target.scale
            {
                return false;
            }
        }
    }
    true
}

#[derive(Clone, Copy)]
struct PatchCounts {
    removed: usize,
    generated: usize,
    torso: usize,
    arm: usize,
}

fn replace_band(
    source: &SurfaceMesh, selected: &[bool], mut loops: Vec<Vec<u32>>,
    guide: Guide, target: &AnatomyTarget, labels: &[[f32; 2]], side: usize,
) -> Option<(SurfaceMesh, Vec<[f32; 2]>, PatchCounts)> {
    if loops.len() != 2 { return None; }
    loops.sort_by(|a, b| {
        let mean = |ring: &Vec<u32>| ring.iter()
            .map(|&id| guide.t(source.vertices[id as usize])).sum::<f32>()
            / ring.len() as f32;
        mean(a).total_cmp(&mean(b))
    });
    let mut arm = loops.pop()?;
    let mut torso = loops.pop()?;
    let (m, n) = (torso.len(), arm.len());
    if !(MIN_RING..=MAX_RING).contains(&m)
        || !(MIN_RING..=MAX_RING).contains(&n)
        || m.abs_diff(n) > 20 || (m + n) % 2 != 0
        || m.abs_diff(n) / 2 >= m.min(n)
    { return None; }
    let average_t = |ring: &[u32]| ring.iter()
        .map(|&id| guide.t(source.vertices[id as usize])).sum::<f32>()
        / ring.len() as f32;
    if average_t(&arm) - average_t(&torso) < 0.22 { return None; }

    // The boundaries of an oriented annulus have opposite winding. Reverse
    // the distal boundary so both parameterised rings advance together.
    arm.reverse();
    rotate_to_front(&mut torso, source, guide);
    rotate_to_front(&mut arm, source, guide);

    let distal = distal_vertices(source, selected, &arm, &torso)?;
    let mut retained_labels = labels.to_vec();
    for (label, is_distal) in retained_labels.iter_mut().zip(distal) {
        if is_distal { label[side] = 1.0; }
    }

    let removed = selected.iter().filter(|&&v| v).count();
    let retained = SurfaceMesh {
        vertices: source.vertices.clone(),
        quads: source.quads.iter().enumerate().filter_map(|(i, &quad)|
            (!selected[i]).then_some(quad)).collect(),
        triangles: Vec::new(),
        caps: Vec::new(),
    };
    let start = retained.quads.len();
    let count = m.min(n);
    // A 'frontmost' vertex is only an approximate phase reference on a
    // nonuniform surface-net ring. Try its immediate neighbours as well:
    // one edge of mismatch can fold the patch even with valid boundary loops.
    for shift in [0, n - 1, 1, n - 2, 2] {
        let mut mesh = retained.clone();
        let mut patch_labels = retained_labels.clone();
        let mut phased_arm = arm.clone();
        phased_arm.rotate_left(shift);
        // Torso interface -> deltoid cap -> underarm support -> upper-arm loop.
        // The interior rings are projected onto the same neutral anatomy field.
        let Some(deltoid) = support_ring(&mut mesh, &torso, &phased_arm, count, 0.32, target)
            else { continue; };
        patch_labels.extend((0..deltoid.len()).map(|_| {
            let mut label = [0.0; 2];
            label[side] = 0.32;
            label
        }));
        let Some(axilla) = support_ring(&mut mesh, &torso, &phased_arm, count, 0.68, target)
            else { continue; };
        patch_labels.extend((0..axilla.len()).map(|_| {
            let mut label = [0.0; 2];
            // The distal support ring is mostly upper arm, even at the
            // axillary underside. This reduces shear into its arm boundary.
            label[side] = 0.78;
            label
        }));
        let hotspot = guide.axilla;
        if bridge(&mut mesh, &torso, &deltoid, hotspot).is_none()
            || bridge(&mut mesh, &deltoid, &axilla, hotspot).is_none()
            || bridge(&mut mesh, &axilla, &phased_arm, hotspot).is_none()
            || !valid_new_quads(&mesh, start, target)
        { continue; }
        let generated = mesh.quads.len() - start;
        compact(&mut mesh, &mut patch_labels);
        if mesh.validate_closed().is_err() { continue; }
        let mut seen = BTreeSet::new();
        let unique = mesh.quads.iter().all(|quad| {
            if quad.iter().copied().collect::<BTreeSet<_>>().len() != 4 {
                return false;
            }
            let mut ids = *quad;
            ids.sort_unstable();
            seen.insert(ids)
        });
        if unique {
            return Some((mesh, patch_labels,
                PatchCounts { removed, generated, torso: m, arm: n }));
        }
    }
    None
}

fn one_side(
    mesh: &mut SurfaceMesh, labels: &mut Vec<[f32; 2]>,
    target: &AnatomyTarget, left: bool,
) -> Result<PatchCounts, String> {
    let guide = Guide::new(target, left)?;
    let (adjacent, uses) = adjacency(mesh)?;
    let mut bands = 0;
    let mut two_boundaries = 0;
    // Search proximal/distal cuts from anatomical landmarks. A candidate is
    // eligible only if it yields exactly two closed, simple boundary loops.
    for start in [0.155, 0.115, 0.195, 0.075, 0.035, -0.015, -0.055, -0.095] {
        for end in [0.57, 0.50, 0.43] {
            for radius in [0.17, 0.195, 0.225] {
                let Some(selected) = band_faces(
                    mesh, guide, &adjacent, start, end, radius
                ) else { continue; };
                bands += 1;
                let Some(loops) = boundary_loops(&selected, &uses) else { continue; };
                if loops.len() == 2 { two_boundaries += 1; }
                if let Some((candidate, candidate_labels, counts)) =
                    replace_band(mesh, &selected, loops, guide, target,
                        labels, if left { 0 } else { 1 })
                {
                    *mesh = candidate;
                    *labels = candidate_labels;
                    return Ok(counts);
                }
            }
        }
    }
    Err(format!(
        "No safe {} shoulder torso/arm boundary pair for the structured quad patch \
         ({bands} regions, {two_boundaries} two-loop boundaries)",
        if left { "left" } else { "right" },
    ))
}

/// Complete the left shoulder and validate it before mirroring the same
/// anatomical construction on the right. The source is left untouched if
/// either patch fails. Posed and morphology-fitted meshes never call this.
pub(crate) fn apply(
    mesh: &mut SurfaceMesh, target: &AnatomyTarget,
) -> Result<(RingPatchReport, Vec<[f32; 2]>), String> {
    let mut candidate = mesh.clone();
    let mut labels = vec![[0.0; 2]; candidate.vertices.len()];
    let left = one_side(&mut candidate, &mut labels, target, true)?;
    candidate.validate_closed()?;
    let right = one_side(&mut candidate, &mut labels, target, false)?;
    candidate.validate_closed()?;
    if labels.len() != candidate.vertices.len() {
        return Err("Shoulder ownership lost alignment with cage vertices".into());
    }
    *mesh = candidate;
    Ok((RingPatchReport {
        left_removed_quads: left.removed,
        right_removed_quads: right.removed,
        left_patch_quads: left.generated,
        right_patch_quads: right.generated,
        left_torso_ring: left.torso,
        right_torso_ring: right.torso,
        left_arm_ring: left.arm,
        right_arm_ring: right.arm,
    }, labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal_even_rings_stitch_without_boundary_cracks() {
        let mut mesh = SurfaceMesh {
            vertices: Vec::new(), quads: Vec::new(),
            triangles: Vec::new(), caps: Vec::new(),
        };
        let mut outer = Vec::new();
        let mut inner = Vec::new();
        for i in 0..12 {
            let t = i as f32 * std::f32::consts::TAU / 12.0;
            outer.push(mesh.vertices.len() as u32);
            mesh.vertices.push(Vec3::new(t.cos(), 0.0, t.sin()));
        }
        for i in 0..16 {
            let t = i as f32 * std::f32::consts::TAU / 16.0;
            inner.push(mesh.vertices.len() as u32);
            mesh.vertices.push(Vec3::new(t.cos(), 0.3, t.sin()));
        }
        bridge(&mut mesh, &outer, &inner, Vec3::new(0.0, -0.4, -1.0)).unwrap();
        let uses = edge_uses(&mesh);
        let outer_edges = (0..outer.len()).map(|i|
            edge(outer[i], outer[(i + 1) % outer.len()]))
            .collect::<BTreeSet<_>>();
        let inner_edges = (0..inner.len()).map(|i|
            edge(inner[i], inner[(i + 1) % inner.len()]))
            .collect::<BTreeSet<_>>();
        for (edge, faces) in uses {
            let count = if outer_edges.contains(&edge) || inner_edges.contains(&edge) {
                1
            } else { 2 };
            assert_eq!(faces.len(), count);
            if faces.len() == 2 {
                assert_eq!((faces[0].1, faces[0].2), (faces[1].2, faces[1].1));
            }
        }
    }

    #[test]
    fn neutral_template_has_bilateral_shoulder_support_rings() {
        let model = crate::quad_template::neutral_template().unwrap();
        let report = crate::quad_template::neutral_shoulder_ring_report().unwrap();
        assert!(report.left_patch_quads > 0 && report.right_patch_quads > 0);
        assert_eq!(model.triangles.len(), 2 * model.quads.len());
        assert_eq!(model.shoulder_guides.len(), model.vertices.len());
        for side in 0..2 {
            for value in [0.32, 0.78, 1.0] {
                assert!(model.shoulder_guides.iter()
                    .any(|guide| guide[side] == value),
                    "Missing guided shoulder region {side} at {value}");
            }
        }
        assert!(model.shoulder_guides.iter().all(|guide|
            guide[0] >= 0.0 && guide[0] <= 1.0
                && guide[1] >= 0.0 && guide[1] <= 1.0
                && guide[0] * guide[1] == 0.0));
    }
}
