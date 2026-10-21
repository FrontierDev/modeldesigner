//! Anatomical, topology-preserving pelvis-to-thigh bifurcation.
//!
//! Three boundary curves define one connected "pair of trousers" surface:
//! a 32-vertex waist and two 12-vertex upper-thigh roots. Anatomical roots
//! are constructed first; the remaining pelvic quads are harmonically fitted
//! between those boundaries. There is no horizontal pelvic lid or leg tube
//! attached to a separate skirt.

use crate::mesh::MeshBuilder;
use crate::{Recipe, VertexInfluence};
use glam::Vec3;
use std::f32::consts::{PI, TAU};

const GRID: usize = 8;
const THIGH_RING_COUNT: usize = 12;
const UNUSED: u32 = u32::MAX;
const FAIRING_PASSES: usize = 180;
// Per-side midline clearance, in metres for a reference-height humanoid.
// The two legs need room for the inner-thigh contour when girth is maximal.
const MIN_MEDIAL_HALF_GAP: f32 = 0.025;

// Each hole is a 2×4 rectangular patch, so equal angular increments per
// vertex twist the socket when connected to that patch. Map the actual
// perimeter coordinates to a circular cross-section instead. Both sides
// use the same local coordinates: grid reflection reverses the ring indices.
const THIGH_SOCKET_COORDS: [(f32, f32); THIGH_RING_COUNT] = [
    (-1.0, -1.0), (0.0, -1.0), (1.0, -1.0),
    (1.0, -0.5), (1.0, 0.0), (1.0, 0.5),
    (1.0, 1.0), (0.0, 1.0), (-1.0, 1.0),
    (-1.0, 0.5), (-1.0, 0.0), (-1.0, -0.5),
];

fn socket_direction(index: usize) -> (f32, f32) {
    let (u, v) = THIGH_SOCKET_COORDS[index];
    let distance = (u * u + v * v).sqrt();
    (u / distance, v / distance)
}

#[derive(Clone, Copy)]
struct Dimensions {
    scale: f32,
    torso_length: f32,
    hip_width: f32,
    thigh_girth: f32,
    leg_length: f32,
    stance_width: f32,
    pelvis_tilt: f32,
    glute_volume: f32,
    body_mass: f32,
    musculature: f32,
}

impl From<&Recipe> for Dimensions {
    fn from(recipe: &Recipe) -> Self {
        let m = &recipe.morphology;
        Self {
            scale: recipe.height / 1.75,
            torso_length: m.torso_length,
            hip_width: m.hip_width,
            thigh_girth: m.thigh_girth,
            leg_length: recipe.leg_length,
            stance_width: m.stance_width,
            pelvis_tilt: m.pelvis_tilt,
            glute_volume: m.glute_volume,
            body_mass: recipe.body_mass,
            musculature: recipe.musculature,
        }
    }
}

impl Dimensions {
    fn y(self, y: f32) -> f32 {
        (0.78 + (y - 0.78) * self.torso_length) * self.scale
    }

    fn hip(self, side: f32) -> Vec3 {
        Vec3::new(side * 0.097 * self.hip_width * self.scale, self.y(0.755), 0.0)
    }

    fn knee(self, side: f32) -> Vec3 {
        self.hip(side)
            + Vec3::new(
                side * 0.035 * (self.stance_width - 1.0) * self.scale,
                -0.338 * self.leg_length * self.scale,
                0.0,
            )
    }

    /// Expand a thigh around its anatomical center without truncating its
    /// outer circumference when the medial side reaches the crotch clearance.
    ///
    /// The old .min(center_x - gap) radius cap made Thigh Girth visually inert
    /// over most of its upper range. Instead, keep the actual lateral extent
    /// and move the cross-section centre outward *only as needed*.
    fn transverse_section(self, side: f32, base_centre: f32, radius: f32) -> (f32, f32) {
        let outward = base_centre.abs() + radius;
        let inner = (base_centre.abs() - radius)
            .max(MIN_MEDIAL_HALF_GAP * self.scale);
        (side * (outward + inner) * 0.5, (outward - inner) * 0.5)
    }

    // Proximal femur is inside the pelvic mass. The root circumference is
    // *not* a horizontal ring: lateral greater-trochanter contours climb
    // towards the hips while the medial edges descend into the crotch.
    fn root(self, side: f32, index: usize) -> Vec3 {
        let (cos, sin) = socket_direction(index);
        // Keep the greater trochanter around the femoral head but model a
        // full proximal thigh, not a narrow cylinder below the waist.
        // Medial clearance shifts the *contour* outward rather than silently
        // discarding the thigh-girth slider's expansion.
        let (centre_x, radius_x) = self.transverse_section(
            side,
            0.115 * self.hip_width * self.scale,
            0.092 * self.thigh_girth
                * (1.0 + 0.10 * (self.body_mass - 0.5)) * self.scale,
        );
        let radius_z = 0.125 * self.thigh_girth
            * (1.0 + 0.12 * (self.body_mass - 0.5))
            * self.scale;
        let outward = side * cos;
        let anterior = sin.max(0.0);
        let posterior = (-sin).max(0.0);
        Vec3::new(
            centre_x + cos * radius_x,
            self.y(0.665) + (0.077 * outward
                + 0.028 * (self.pelvis_tilt - 0.5) * sin) * self.scale,
            sin * radius_z
                + (0.008 * anterior.powi(2)
                    - 0.011 * posterior.powi(2)
                    - 0.009
                    - 0.042 * (self.glute_volume - 0.5) * posterior)
                    * self.scale,
        )
    }
    /// Neutral thigh loft from a fixed proximal contour to the knee.
    ///
    /// Regional bulges fade to zero at both boundaries: quadriceps anterior,
    /// hamstrings posterior, vastus lateralis externally, and adductors on
    /// the proximal medial side. No extra geometry or floating muscle shells.
    fn thigh_point(
        self,
        side: f32,
        index: usize,
        origin: Vec3,
        knee: Vec3,
        fraction: f32,
    ) -> Vec3 {
        let (cos, sin) = socket_direction(index);
        let smooth = fraction * fraction * (3.0 - 2.0 * fraction);
        let (knee_centre, knee_radius) = self.transverse_section(
            side, knee.x, 0.068 * self.thigh_girth * self.scale,
        );
        let target = Vec3::new(
            knee_centre + cos * knee_radius,
            knee.y,
            sin * 0.084 * self.thigh_girth * self.scale - 0.010 * self.scale,
        );
        let mut point = origin.lerp(target, smooth);
        point.y = origin.y * (1.0 - fraction) + knee.y * fraction;

        let bulge = (PI * fraction).sin().max(0.0);
        // Full thigh thickness grows along the *whole* shaft. Regional
        // muscle detail is a secondary contour, not the primary girth effect.
        let muscle = (0.70 + 0.60 * self.musculature)
            * self.thigh_girth * self.scale;
        let anterior = sin.max(0.0);
        let posterior = (-sin).max(0.0);
        let lateral = (side * cos).max(0.0);
        let medial = (-side * cos).max(0.0);
        point.z += (0.012 * anterior.powi(2) - 0.012 * posterior.powi(2))
            * bulge * muscle;
        let vastus = 0.006 * lateral.powi(2) * bulge * muscle;
        // Adductors round the proximal inner thigh, but their displacement
        // must not push either leg through the other at extreme girth.
        let adductor = 0.009 * medial.powi(2) * bulge
            * (1.0 - fraction) * muscle;
        point.x += side * vastus - side * adductor;
        let medial_limit = MIN_MEDIAL_HALF_GAP * self.scale;
        if side * point.x < medial_limit {
            point.x = side * medial_limit;
        }
        point
    }

}

pub fn joint_positions(recipe: &Recipe, side: f32) -> (Vec3, Vec3) {
    let d = Dimensions::from(recipe);
    (d.hip(side), d.knee(side))
}

pub struct LowerBody {
    pub influences: Vec<(u32, VertexInfluence)>,
    pub leg_rings: [Vec<Vec<u32>>; 2],
}

fn weight(pelvis: f32, hip: u16) -> VertexInfluence {
    VertexInfluence {
        joint_indices: [0, hip, 0, 0],
        weights: [pelvis, 1.0 - pelvis, 0.0, 0.0],
    }
}

// The outside of the 9x9 grid is the *existing* lower torso circumference.
// Column 0 faces forward, 8 is right lateral, 16 is posterior, 24 left.
fn outside_index(i: usize, j: usize) -> Option<usize> {
    if j == GRID {
        return Some(if i <= 4 { (28 + i) % 32 } else { i - 4 });
    }
    if i == GRID {
        return Some(12 - j);
    }
    if j == 0 {
        return Some(20 - i);
    }
    if i == 0 {
        return Some(20 + j);
    }
    None
}

/// All three strictly interior lattice points of each 2×4 thigh socket
/// belong exclusively to omitted cells. Allocating any of them produces
/// orphaned vertices (and shifts deterministic output counts).
fn unused_inside_hole(i: usize, j: usize) -> bool {
    (i == 2 || i == 6) && (3..=5).contains(&j)
}

fn is_hole_cell(i: usize, j: usize) -> bool {
    ((1..3).contains(&i) || (5..7).contains(&i)) && (2..6).contains(&j)
}

fn thigh_hole(grid: &[[u32; 9]; 9], start: usize) -> Vec<u32> {
    let end = start + 2;
    let mut rim = Vec::with_capacity(THIGH_RING_COUNT);
    for i in start..end {
        rim.push(grid[2][i]);
    }
    for row in 2..6 {
        rim.push(grid[row][end]);
    }
    for i in (start + 1..=end).rev() {
        rim.push(grid[6][i]);
    }
    for row in (3..=6).rev() {
        rim.push(grid[row][start]);
    }
    rim
}

fn add_edge(neighbours: &mut [Vec<usize>], a: u32, b: u32) {
    neighbours[a as usize].push(b as usize);
    neighbours[b as usize].push(a as usize);
}

/// Solve the free pelvic vertices between the fixed waist and both thigh
/// roots. Iteration uses only authored quad edges (never rendering diagonals).
/// The waist, thigh sockets, and existing shoulder topology remain untouched.
fn fit_pelvis(
    mesh: &mut MeshBuilder,
    grid: &[[u32; 9]; 9],
    pinned: &[bool],
    dimensions: Dimensions,
) -> Result<(), String> {
    let mut neighbours = vec![Vec::<usize>::new(); mesh.vertices.len()];
    for row in 0..GRID {
        for col in 0..GRID {
            if is_hole_cell(col, row) {
                continue;
            }
            let q = [
                grid[row][col],
                grid[row][col + 1],
                grid[row + 1][col + 1],
                grid[row + 1][col],
            ];
            for (a, b) in [(q[0], q[1]), (q[1], q[2]), (q[2], q[3]), (q[3], q[0])] {
                if a == UNUSED || b == UNUSED {
                    return Err("Pelvis uses an uninitialized socket vertex".into());
                }
                add_edge(&mut neighbours, a, b);
            }
        }
    }
    let mut free = Vec::new();
    for row in grid {
        for &id in row {
            if id != UNUSED && !pinned[id as usize] && !free.contains(&(id as usize)) {
                free.push(id as usize);
            }
        }
    }
    for neighbours in &mut neighbours {
        neighbours.sort_unstable();
        neighbours.dedup();
    }
    for _ in 0..FAIRING_PASSES {
        let before = mesh.vertices.clone();
        for &id in &free {
            let adjacent = &neighbours[id];
            if adjacent.is_empty() {
                return Err("Disconnected pelvic patch vertex".into());
            }
            let average = adjacent.iter().map(|&other| before[other]).sum::<Vec3>()
                / adjacent.len() as f32;
            mesh.vertices[id] = before[id].lerp(average, 0.72);
        }
    }
    // Pelvic contour refinement on *free* interior vertices only. The
    // original waist and curved thigh-root seams remain exact. Gradual
    // posterior gluteal volume and a shallow anterior pubic contour prevent
    // an abrupt shelf at the greater trochanter or a sharp central V.
    for &id in &free {
        let p = mesh.vertices[id];
        let posterior = (-p.z / (0.12 * dimensions.scale))
            .clamp(0.0, 1.0);
        let anterior = (p.z / (0.12 * dimensions.scale))
            .clamp(0.0, 1.0);
        let hip_x = 0.100 * dimensions.hip_width * dimensions.scale;
        let lateral_hip = (1.0 - (p.x.abs() - hip_x).abs() / hip_x)
            .clamp(0.0, 1.0);
        let centre = (1.0 - p.x.abs() / hip_x).clamp(0.0, 1.0);
        let lower = (1.0 - (p.y - dimensions.y(0.665)).abs()
            / (0.14 * dimensions.scale)).clamp(0.0, 1.0);
        // Pubic and proximal-adductor mass should read as a softly rounded
        // anterior volume, not a flat triangular crotch panel. The shallow
        // bilateral inguinal depressions frame that volume without creating
        // a separate mesh or introducing a hard groin crease.
        let inguinal = (1.0
            - (p.x.abs() - 0.065 * dimensions.scale).abs()
                / (0.058 * dimensions.scale))
            .clamp(0.0, 1.0);
        let pubic = 0.028 * anterior * centre.powi(2) * lower;
        let inguinal_fold = 0.007 * anterior * inguinal * lower;
        mesh.vertices[id].z += dimensions.scale * (
            -0.018 * (0.65 + 0.70 * dimensions.glute_volume)
                * posterior * lateral_hip * lower
            + pubic - inguinal_fold
        );
        // A small central descent rounds the transition to both medial
        // thighs, while the fixed three boundary rings stay untouched.
        mesh.vertices[id].y -= 0.011 * dimensions.scale
            * anterior * centre.powi(2) * lower;
    }
    Ok(())
}

/// One shared surface with three anatomical boundaries: a torso ring and two
/// curved thigh roots. A fixed quad-patch graph connects them, and its
/// positions are *solved from the roots* rather than generated as a flat lid.
/// The reserved original crotch vertex keeps shoulder indices unchanged.
pub fn append(
    mesh: &mut MeshBuilder,
    torso_bottom: &[u32],
    crotch_id: u32,
    recipe: &Recipe,
) -> Result<LowerBody, String> {
    if torso_bottom.len() != 32 || crotch_id as usize >= mesh.vertices.len() {
        return Err("Pelvis requires the existing 32-vertex waist and crotch anchor".into());
    }
    let d = Dimensions::from(recipe);
    let mut grid = [[UNUSED; 9]; 9];

    // Allocate *only* vertices used by faces. Each 2×4 socket omits three
    // grid vertices along its centreline, not just the central one.
    for j in 0..=GRID {
        for i in 0..=GRID {
            grid[j][i] = if let Some(column) = outside_index(i, j) {
                torso_bottom[column]
            } else if unused_inside_hole(i, j) {
                UNUSED
            } else if i == 4 && j == 4 {
                crotch_id
            } else {
                mesh.vertex(Vec3::new(
                    (i as f32 / GRID as f32 - 0.5) * 0.30 * d.scale,
                    d.y(0.70),
                    (j as f32 / GRID as f32 - 0.5) * 0.25 * d.scale,
                ))
            };
        }
    }

    let mut pinned = vec![false; mesh.vertices.len()];
    for &id in torso_bottom {
        pinned[id as usize] = true;
    }

    let mut influences = Vec::new();
    let mut root_loops = [Vec::<u32>::new(), Vec::<u32>::new()];
    for (which, (side, start, hip_joint)) in [
        (-1.0_f32, 1_usize, 10_u16),
        (1.0_f32, 5_usize, 12_u16),
    ].into_iter().enumerate() {
        let rim = thigh_hole(&grid, start);
        if rim.len() != THIGH_RING_COUNT || rim.contains(&UNUSED) {
            return Err("Invalid anatomical thigh socket boundary".into());
        }
        for (i, &id) in rim.iter().enumerate() {
            mesh.vertices[id as usize] = d.root(side, i);
            pinned[id as usize] = true;
            influences.push((id, weight(0.75, hip_joint)));
        }
        root_loops[which] = rim;
    }

    // The patch is a connected 3-boundary surface, not a closing pelvis cap.
    for j in 0..GRID {
        for i in 0..GRID {
            if is_hole_cell(i, j) {
                continue;
            }
            // The two diagonal corners have three vertices on the waist
            // circle. Their default diagonal creates a horizontal triangle
            // adjacent to a steeply sloping pelvic triangle. Choose the
            // other fixed diagonal to keep the surface normals coherent.
            let flip = (j == 0 && i == GRID - 1)
                || (j == GRID - 1 && i == 0);
            mesh.quad_with_diagonal(
                grid[j][i],
                grid[j][i + 1],
                grid[j + 1][i + 1],
                grid[j + 1][i],
                flip,
            );
        }
    }
    fit_pelvis(mesh, &grid, &pinned, d)?;

    for row in &grid {
        for &id in row {
            if id != UNUSED && !pinned[id as usize] {
                influences.push((id, weight(1.0, 0)));
            }
        }
    }

    let mut leg_rings = [Vec::new(), Vec::new()];
    for (side_index, (side, joint)) in [(-1.0_f32, 10_u16), (1.0, 12_u16)]
        .into_iter()
        .enumerate()
    {
        let root = &root_loops[side_index];
        let knee = d.knee(side);
        let mut previous = root.clone();
        // Anatomical thigh surfaces grow continuously *from* the socket ring.
        // The curved socket is high laterally and low at the inner crotch.
        for (band, fraction) in [0.18_f32, 0.42, 0.70, 1.0].into_iter().enumerate() {
            let roots: Vec<Vec3> = root.iter()
                .map(|&id| mesh.vertices[id as usize]).collect();
            let next = mesh.loop_vertices(
                roots.into_iter().enumerate()
                    .map(|(i, origin)| d.thigh_point(side, i, origin, knee, fraction))
            );
            if previous.iter().zip(&next).any(|(&a, &b)| {
                mesh.vertices[b as usize].y >= mesh.vertices[a as usize].y - 1.0e-5
            }) {
                return Err(format!("Thigh ring {band} folds upward toward the pelvis"));
            }
            mesh.stitch(&previous, &next)?;
            for &id in &next {
                influences.push((id, weight([0.42, 0.17, 0.0, 0.0][band], joint)));
            }
            leg_rings[side_index].push(next.clone());
            previous = next;
        }
        let centre_id = mesh.vertices.len() as u32;
        mesh.cap(&previous, knee - Vec3::Y * 0.006 * d.scale, false)?;
        influences.push((centre_id, weight(0.0, joint)));
    }
    Ok(LowerBody { influences, leg_rings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn girth_uniformly_expands_front_back_and_outer_thigh_from_root_to_knee() {
        for hip_width in [0.8, 1.0, 1.25] {
            let mut normal = Recipe::default();
            normal.morphology.hip_width = hip_width;
            let mut enlarged = normal.clone();
            enlarged.morphology.thigh_girth = 1.4;
            let small = Dimensions::from(&normal);
            let big = Dimensions::from(&enlarged);
            for side in [-1.0_f32, 1.0] {
                for fraction in [0.0, 0.18, 0.42, 0.70, 1.0] {
                    let extent = |d: Dimensions| {
                        let points: Vec<Vec3> = (0..THIGH_RING_COUNT)
                            .map(|index| d.thigh_point(
                                side, index, d.root(side, index), d.knee(side), fraction
                            )).collect();
                        let outside = points.iter().map(|p| side * p.x)
                            .fold(f32::NEG_INFINITY, f32::max);
                        let front = points.iter().map(|p| p.z)
                            .fold(f32::NEG_INFINITY, f32::max);
                        let back = points.iter().map(|p| p.z)
                            .fold(f32::INFINITY, f32::min);
                        (outside, front - back)
                    };
                    let (small_outside, small_depth) = extent(small);
                    let (big_outside, big_depth) = extent(big);
                    assert!(
                        big_outside > small_outside + 0.020 * big.scale,
                        "No lateral girth growth: hip={hip_width} side={side} t={fraction}"
                    );
                    assert!(
                        big_depth > small_depth * 1.20,
                        "No front/back girth growth: hip={hip_width} side={side} t={fraction}"
                    );
                }
            }
        }
    }

    #[test]
    fn medial_clearance_keeps_lateral_expansion_when_saturated() {
        let d = Dimensions::from(&Recipe::default());
        let (small_center, small_radius) =
            d.transverse_section(1.0, 0.105, 0.07);
        let (big_center, big_radius) =
            d.transverse_section(1.0, 0.105, 0.135);
        assert!(big_center + big_radius > small_center + small_radius + 0.060);
        assert!(big_center - big_radius >= MIN_MEDIAL_HALF_GAP * d.scale - 1e-6);
    }

    #[test]
    fn supported_girth_stance_and_hip_extremes_keep_thighs_separate() {
        for width in [0.8, 1.0, 1.25] {
            for girth in [0.7, 1.0, 1.4] {
                for stance in [0.8, 1.0, 1.2] {
                    let mut recipe = Recipe::default();
                    recipe.morphology.hip_width = width;
                    recipe.morphology.thigh_girth = girth;
                    recipe.morphology.stance_width = stance;
                    let d = Dimensions::from(&recipe);
                    for fraction in [0.0, 0.18, 0.42, 0.70, 1.0] {
                        let mut left_inner = f32::NEG_INFINITY;
                        let mut right_inner = f32::INFINITY;
                        for index in 0..THIGH_RING_COUNT {
                            let left = d.thigh_point(-1.0, index,
                                d.root(-1.0, index), d.knee(-1.0), fraction);
                            let right = d.thigh_point(1.0, index,
                                d.root(1.0, index), d.knee(1.0), fraction);
                            left_inner = left_inner.max(left.x);
                            right_inner = right_inner.min(right.x);
                        }
                        assert!(
                            right_inner - left_inner > 0.030 * d.scale,
                            "Thigh overlap or unsafe crotch gap at hip={width},                              girth={girth}, stance={stance}, fraction={fraction}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn paired_thigh_socket_rims_are_exact_mirrors() {
        for girth in [0.7, 1.0, 1.4] {
            for width in [0.8, 1.0, 1.25] {
                let mut recipe = Recipe::default();
                recipe.morphology.thigh_girth = girth;
                recipe.morphology.hip_width = width;
                let dims = Dimensions::from(&recipe);
                for k in 0..THIGH_RING_COUNT {
                    // The reflected grid position is index (2-k) modulo 12.
                    let reflected = (THIGH_RING_COUNT + 2 - k) % THIGH_RING_COUNT;
                    let l = dims.root(-1.0, k);
                    let r = dims.root(1.0, reflected);
                    assert!((l.x + r.x).abs() < 1.0e-6,
                        "Thigh-root X mismatch at girth={girth}, k={k}");
                    assert!((l.y - r.y).abs() < 1.0e-6);
                    assert!((l.z - r.z).abs() < 1.0e-6);
                }
            }
        }
    }

    #[test]
    fn waist_to_pelvis_corner_quads_use_stable_alternative_diagonals() {
        for girth in [0.7, 1.0, 1.4] {
        let mut recipe = Recipe::default();
        recipe.morphology.thigh_girth = girth;
        let mut builder = MeshBuilder::default();
        let waist = builder.loop_vertices((0..32).map(|i| {
            let theta = TAU * i as f32 / 32.0;
            Vec3::new(theta.sin() * 0.159, 0.78, theta.cos() * 0.122)
        }));
        let upper = builder.loop_vertices((0..32).map(|i| {
            let theta = TAU * i as f32 / 32.0;
            Vec3::new(theta.sin() * 0.151, 0.86, theta.cos() * 0.115)
        }));
        builder.stitch(&waist, &upper).unwrap();
        let crotch = builder.vertex(Vec3::ZERO);
        builder.cap(&upper, Vec3::new(0.0, 0.86, 0.0), false).unwrap();
        append(&mut builder, &waist, crotch, &recipe).unwrap();
        // First 32 quads connect waist to a mock torso ring. The two
        // special pelvis quads are at indices 39 and 72.
        for index in [39_usize, 72_usize] {
            let [a, b, c, d] = builder.quads[index];
            // Temporary neck and knee caps are interleaved with the quad
            // render triangles, so locate the authored adjacent pair rather
            // than incorrectly assuming triangle_index == 2 * quad_index.
            assert!(
                builder.triangles.windows(2).any(|pair| {
                    pair[0] == [a, b, d] && pair[1] == [b, c, d]
                }),
                "Corner quad {index} did not retain its fixed diagonal"
            );
            let va = builder.vertices[a as usize];
            let vb = builder.vertices[b as usize];
            let vc = builder.vertices[c as usize];
            let vd = builder.vertices[d as usize];
            let n1 = (vb - va).cross(vd - va).normalize();
            let n2 = (vc - vb).cross(vd - vb).normalize();
            assert!(n1.dot(n2) > 0.75, "Corner crease at girth={girth}");
        }
        }
    }

    #[test]
    fn anatomical_roots_slope_toward_the_crotch() {
        let d = Dimensions::from(&Recipe::default());
        for side in [-1.0, 1.0] {
            let root = (0..12).map(|index| d.root(side, index)).collect::<Vec<_>>();
            let outside = root.iter().filter(|v| v.x * side > 0.14).map(|v| v.y)
                .fold(f32::NEG_INFINITY, f32::max);
            let inside = root.iter().filter(|v| v.x * side < 0.06).map(|v| v.y)
                .fold(f32::INFINITY, f32::min);
            assert!(outside > inside + 0.12);
        }
    }

    #[test]
    fn every_socket_interior_vertex_is_skipped() {
        let hole_points: Vec<(usize, usize)> = (1..8)
            .flat_map(|i| (1..8).map(move |j| (i, j)))
            .filter(|&(i,j)| unused_inside_hole(i,j))
            .collect();
        assert_eq!(hole_points, vec![
            (2,3), (2,4), (2,5), (6,3), (6,4), (6,5)
        ]);
        for start in [1, 5] {
            for j in 3..=5 {
                assert!(unused_inside_hole(start + 1, j));
            }
        }
    }

    #[test]
    fn three_boundary_patch_has_no_unused_socket_vertices() {
        let mut mesh = MeshBuilder::default();
        let waist = mesh.loop_vertices((0..32).map(|i| {
            let theta = TAU * i as f32 / 32.0;
            Vec3::new(theta.sin() * 0.159, 0.78, theta.cos() * 0.122)
        }));
        let upper = mesh.loop_vertices((0..32).map(|i| {
            let theta = TAU * i as f32 / 32.0;
            Vec3::new(theta.sin() * 0.150, 0.86, theta.cos() * 0.116)
        }));
        mesh.stitch(&waist, &upper).unwrap();
        let crotch = mesh.vertex(Vec3::ZERO);
        mesh.cap(&upper, Vec3::new(0.0, 0.86, 0.0), false).unwrap();
        let before = mesh.vertices.len();
        let lower = append(&mut mesh, &waist, crotch, &Recipe::default()).unwrap();
        assert_eq!(mesh.vertices.len() - before, 140);
        assert_eq!(lower.leg_rings[0].len(), 4);
        assert_eq!(lower.leg_rings[1].len(), 4);
        let surface = mesh.build().unwrap();
        let referenced: HashSet<u32> = surface.triangles.iter().flatten().copied().collect();
        assert_eq!(referenced.len(), surface.vertices.len(), "Unused / detached vertices");
        assert!(surface.vertices.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn hip_and_thigh_ranges_preserve_anatomical_geometry() {
        for hip in [0.8, 1.0, 1.25] {
            for girth in [0.7, 1.0, 1.4] {
                for tilt in [0.0, 1.0] {
                    let mut recipe = Recipe::default();
                    recipe.morphology.hip_width = hip;
                    recipe.morphology.thigh_girth = girth;
                    recipe.morphology.pelvis_tilt = tilt;
                    let d = Dimensions::from(&recipe);
                    let left = (0..12).map(|i| d.root(-1.0, i)).collect::<Vec<_>>();
                    let right = (0..12).map(|i| d.root(1.0, i)).collect::<Vec<_>>();
                    let left_inner = left.iter().map(|v| v.x).fold(f32::NEG_INFINITY, f32::max);
                    let right_inner = right.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
                    assert!(right_inner - left_inner > 0.020, "Thigh root overlap");
                    assert!(d.knee(-1.0).y < left.iter().map(|v| v.y)
                        .fold(f32::INFINITY, f32::min));
                }
            }
        }
    }

    #[test]
    fn neutral_hip_contour_and_crotch_gap_have_anatomical_bounds() {
        let d = Dimensions::from(&Recipe::default());
        for side in [-1.0_f32, 1.0] {
            let root: Vec<Vec3> = (0..THIGH_RING_COUNT)
                .map(|i| d.root(side, i)).collect();
            let lateral = root.iter().map(|p| side * p.x)
                .fold(f32::NEG_INFINITY, f32::max);
            // Waist is ~0.159 m wide at the neutral hip section.
            assert!((0.19..0.23).contains(&lateral));
            let outer_y = root.iter()
                .filter(|p| side * p.x > 0.14)
                .map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
            let inner_y = root.iter()
                .filter(|p| side * p.x < 0.06)
                .map(|p| p.y).fold(f32::INFINITY, f32::min);
            assert!(outer_y - inner_y > 0.12);
        }
        let left_inner = (0..THIGH_RING_COUNT)
            .map(|i| d.root(-1.0, i).x).fold(f32::NEG_INFINITY, f32::max);
        let right_inner = (0..THIGH_RING_COUNT)
            .map(|i| d.root(1.0, i).x).fold(f32::INFINITY, f32::min);
        assert!(right_inner - left_inner > 0.045);
    }

    #[test]
    fn pelvis_interior_is_softly_contoured_without_moving_waist_or_hip_seams() {
        let mut mesh = MeshBuilder::default();
        let waist = mesh.loop_vertices((0..32).map(|i| {
            let angle = TAU * i as f32 / 32.0;
            Vec3::new(angle.sin() * 0.159, 0.78, angle.cos() * 0.122)
        }));
        let before = waist.iter()
            .map(|&i| mesh.vertices[i as usize]).collect::<Vec<_>>();
        let crotch = mesh.vertex(Vec3::ZERO);
        let result = append(&mut mesh, &waist, crotch, &Recipe::default()).unwrap();
        for (&id, &old) in waist.iter().zip(&before) {
            assert_eq!(mesh.vertices[id as usize], old);
        }
        for rings in &result.leg_rings {
            assert_eq!(rings.len(), 4);
        }
        let mesh = mesh.build().unwrap();
        assert!(mesh.vertices.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn thigh_anatomical_profiles_shape_front_back_and_medial_surface() {
        let d = Dimensions::from(&Recipe::default());
        let side = 1.0;
        let knee = d.knee(side);
        let fraction = 0.42_f32;
        // Match vertices with positive (front) and negative (rear) Z.
        let front_i = 5; // positive sine on this ring
        let back_i = 1;  // negative sine on this ring
        let front_rest = d.root(side, front_i);
        let back_rest = d.root(side, back_i);
        let front = d.thigh_point(side, front_i, front_rest, knee, fraction);
        let back = d.thigh_point(side, back_i, back_rest, knee, fraction);
        let unshaped = |index: usize, origin: Vec3| {
            let angle = -3.0 * PI / 4.0 + TAU * index as f32 / THIGH_RING_COUNT as f32;
            let t = fraction * fraction * (3.0 - 2.0 * fraction);
            let target = Vec3::new(
                knee.x + angle.cos() * 0.052 * d.thigh_girth * d.scale,
                knee.y,
                angle.sin() * 0.064 * d.thigh_girth * d.scale - 0.010 * d.scale,
            );
            origin.lerp(target, t)
        };
        assert!(front.z > unshaped(front_i, front_rest).z);
        assert!(back.z < unshaped(back_i, back_rest).z);

        let mut muscular = Recipe::default();
        muscular.musculature = 1.0;
        let stronger = Dimensions::from(&muscular);
        assert!(stronger.thigh_point(side, front_i, stronger.root(side, front_i),
            stronger.knee(side), fraction).z > front.z);
        // Knee endpoint remains exactly identical to the unshaped loft,
        // so downstream shin patches can join the same boundary.
        for index in 0..THIGH_RING_COUNT {
            let source = d.root(side, index);
            let neutral_end = d.thigh_point(side, index, source, knee, 1.0);
            assert!((neutral_end.y - knee.y).abs() < 1e-6);
        }
    }

    #[test]
    fn hip_width_and_stance_move_correct_bones() {
        let mut edited = Recipe::default();
        edited.morphology.hip_width = 1.25;
        edited.morphology.stance_width = 1.20;
        let (hip, knee) = joint_positions(&Recipe::default(), 1.0);
        let (wide_hip, wide_knee) = joint_positions(&edited, 1.0);
        assert!(wide_hip.x > hip.x);
        assert!(wide_knee.x > knee.x);
    }
}
