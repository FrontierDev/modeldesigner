//! Anatomy-driven surface target for future fixed-topology fitting.
//!
//! This is NOT the production character mesh, retopology, or a rig. Anatomical
//! volumes are blended into a continuous signed field, then sampled into a
//! disposable triangulated *preview* using marching tetrahedra. Persistent
//! vertex indices, skin weights and UVs must come from a later quad-cage fitter.

use crate::Recipe;
use glam::Vec3;
use std::collections::HashMap;

const PREVIEW_STEP_METRES: f32 = 0.026;
const MAX_GRID_POINTS: usize = 180_000;
const TETS: [[usize; 4]; 6] = [
    [0, 5, 1, 6], [0, 1, 2, 6], [0, 2, 3, 6],
    [0, 3, 7, 6], [0, 7, 4, 6], [0, 4, 5, 6],
];
#[derive(Clone, Debug)]
pub struct Landmark {
    pub name: &'static str,
    pub position: Vec3,
}

#[derive(Clone, Debug)]
pub struct TissueVolume {
    pub name: &'static str,
    pub centre: Vec3,
    pub radii: Vec3,
}

/// Input anatomy. Values are in metres and the mesh is symmetric in bind
/// pose. Neither anatomical tissue volumes nor sample vertices are skin data.
#[derive(Clone, Debug)]
pub struct AnatomyTarget {
    pub landmarks: Vec<Landmark>,
    pub volumes: Vec<TissueVolume>,
    pub scale: f32,
    blend_width: f32,
}

#[derive(Clone, Debug)]
pub struct TargetSurface {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// Ellipsoidal distance approximation. Exact sign, approximate distance:
/// blend widths are calibrated for this field and should not be interpreted
/// as measurements of muscle attachment or real tissue composition.
fn ellipsoid_field(point: Vec3, part: &TissueVolume) -> f32 {
    let relative = (point - part.centre) / part.radii;
    (relative.length() - 1.0) * part.radii.min_element()
}

fn smooth_union(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

fn tissue(name: &'static str, scale: f32, centre: Vec3, radii: Vec3) -> TissueVolume {
    TissueVolume { name, centre: centre * scale, radii: radii * scale }
}

impl AnatomyTarget {
    pub fn from_recipe(recipe: &Recipe) -> Result<Self, String> {
        recipe.validate()?;
        let m = &recipe.morphology;
        let s = recipe.height / 1.75;
        let hip_width = m.hip_width;
        let girth = m.thigh_girth;
        let mass = recipe.body_mass - 0.5;
        let muscle = recipe.musculature - 0.5;
        let hip_x = 0.112 * hip_width;
        // Limb spacing has anatomical priority over a narrower pelvis at high
        // girth: do not discard circumference using a hard radius clamp.
        let thigh_rx = 0.090 * girth * (1.0 + 0.09 * mass);
        let thigh_rz = 0.103 * girth * (1.0 + 0.10 * mass);
        let leg_x = (0.127 * hip_width).max(thigh_rx + 0.025);
        let y_shift = 0.025 * (m.pelvis_tilt - 0.5);
        let knee_y = 0.53 - 0.13 * recipe.leg_length;
        let waist_y = 0.87 + 0.07 * (m.torso_length - 1.0);

        let mut landmarks = vec![
            Landmark { name: "waist", position: Vec3::new(0.0, waist_y, 0.0) * s },
            Landmark { name: "sacrum", position: Vec3::new(0.0, 0.74, -0.067) * s },
            Landmark { name: "pubic_symphysis", position: Vec3::new(0.0, 0.665, 0.072) * s },
        ];
        let mut volumes = vec![
            tissue("pelvic_core", s, Vec3::new(0.0, 0.775 + y_shift, 0.0),
                Vec3::new(0.176 * hip_width, 0.176, 0.130)),
            tissue("lower_abdomen", s, Vec3::new(0.0, 0.856, 0.026),
                Vec3::new(0.163 * hip_width, 0.144, 0.125 + 0.018 * mass)),
            tissue("pubic_adductor_bridge", s, Vec3::new(0.0, 0.667 + y_shift, 0.051),
                Vec3::new(0.083, 0.073, 0.079)),
        ];

        for side in [-1.0_f32, 1.0] {
            let hip = Vec3::new(side * hip_x, 0.750 + y_shift, 0.0) * s;
            let trochanter = Vec3::new(side * (hip_x + 0.032), 0.705, -0.005) * s;
            let knee = Vec3::new(
                side * (leg_x + 0.030 * (m.stance_width - 1.0)),
                knee_y, 0.0,
            ) * s;
            landmarks.extend([
                Landmark {
                    name: if side < 0.0 { "left_hip" } else { "right_hip" },
                    position: hip,
                },
                Landmark {
                    name: if side < 0.0 { "left_trochanter" } else { "right_trochanter" },
                    position: trochanter,
                },
                Landmark {
                    name: if side < 0.0 { "left_knee" } else { "right_knee" },
                    position: knee,
                },
            ]);
            let glute = 0.5 + 0.5 * m.glute_volume;
            volumes.extend([
                tissue(if side < 0.0 { "left_iliac" } else { "right_iliac" },
                    s, Vec3::new(side * hip_x, 0.790 + y_shift, -0.008),
                    Vec3::new(0.090, 0.121, 0.115)),
                tissue(if side < 0.0 { "left_gluteal" } else { "right_gluteal" },
                    s, Vec3::new(side * (hip_x - 0.003), 0.709, -0.067),
                    Vec3::new(0.100 + 0.010 * mass, 0.123, 0.102 + 0.039 * glute)),
                tissue(if side < 0.0 { "left_thigh_core" } else { "right_thigh_core" },
                    s, Vec3::new(side * leg_x, 0.528 + y_shift, -0.003),
                    Vec3::new(thigh_rx, 0.248 * recipe.leg_length, thigh_rz)),
                tissue(if side < 0.0 { "left_quadriceps" } else { "right_quadriceps" },
                    s, Vec3::new(side * (leg_x + 0.003), 0.534, 0.046),
                    Vec3::new(0.079 * girth, 0.204 * recipe.leg_length,
                        0.071 * girth * (1.0 + 0.08 * muscle))),
                tissue(if side < 0.0 { "left_hamstrings" } else { "right_hamstrings" },
                    s, Vec3::new(side * (leg_x + 0.005), 0.536, -0.049),
                    Vec3::new(0.077 * girth, 0.206 * recipe.leg_length,
                        0.070 * girth * (1.0 + 0.08 * muscle))),
                tissue(if side < 0.0 { "left_adductors" } else { "right_adductors" },
                    s, Vec3::new(side * (leg_x - 0.026), 0.615, 0.009),
                    Vec3::new(0.057 * girth, 0.139, 0.073 * girth)),
            ]);
        }
        if volumes.iter().any(|p| !p.centre.is_finite()
            || !p.radii.is_finite() || p.radii.min_element() <= 0.0)
        {
            return Err("Anatomical volume has invalid dimensions".into());
        }
        Ok(Self {
            landmarks,
            volumes,
            scale: s,
            blend_width: 0.044 * s,
        })
    }

    /// Negative inside anatomical tissue, positive outside.
    pub fn field(&self, point: Vec3) -> f32 {
        let mut iter = self.volumes.iter();
        let first = iter.next().expect("anatomical target always has a pelvic core");
        let mut distance = ellipsoid_field(point, first);
        for part in iter {
            distance = smooth_union(distance, ellipsoid_field(point, part), self.blend_width);
        }
        distance
    }

    fn normal(&self, p: Vec3) -> Vec3 {
        let h = 0.0035 * self.scale;
        Vec3::new(
            self.field(p + Vec3::X * h) - self.field(p - Vec3::X * h),
            self.field(p + Vec3::Y * h) - self.field(p - Vec3::Y * h),
            self.field(p + Vec3::Z * h) - self.field(p - Vec3::Z * h),
        ).normalize_or_zero()
    }

    /// Preview-only marching tetrahedra. A shared edge cache prevents cracks
    /// between cells. This output is never rigged, UV-mapped or exported as a
    /// final fixed-topology humanoid.
    pub fn sample_preview(&self) -> Result<TargetSurface, String> {
        let min = Vec3::new(-0.39, 0.17, -0.36) * self.scale;
        let max = Vec3::new(0.39, 1.09, 0.36) * self.scale;
        let step = PREVIEW_STEP_METRES * self.scale;
        let extent = max - min;
        let nx = (extent.x / step).ceil() as usize;
        let ny = (extent.y / step).ceil() as usize;
        let nz = (extent.z / step).ceil() as usize;
        let dims = [nx + 1, ny + 1, nz + 1];
        let total = dims[0] * dims[1] * dims[2];
        if total > MAX_GRID_POINTS {
            return Err("Anatomical preview sampling exceeds memory budget".into());
        }
        let grid_position = |x: usize, y: usize, z: usize| -> Vec3 {
            min + Vec3::new(
                extent.x * x as f32 / nx as f32,
                extent.y * y as f32 / ny as f32,
                extent.z * z as f32 / nz as f32,
            )
        };
        let index = |x: usize, y: usize, z: usize| (z * dims[1] + y) * dims[0] + x;
        let mut positions = Vec::with_capacity(total);
        let mut values = Vec::with_capacity(total);
        for z in 0..dims[2] {
            for y in 0..dims[1] {
                for x in 0..dims[0] {
                    let point = grid_position(x, y, z);
                    positions.push(point);
                    values.push(self.field(point));
                }
            }
        }
        let mut surface = TargetSurface {
            vertices: Vec::new(), normals: Vec::new(), triangles: Vec::new(),
        };
        let mut cache = HashMap::<(usize, usize), u32>::new();
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let corners = [
                        index(x,y,z), index(x+1,y,z),
                        index(x+1,y+1,z), index(x,y+1,z),
                        index(x,y,z+1), index(x+1,y,z+1),
                        index(x+1,y+1,z+1), index(x,y+1,z+1),
                    ];
                    if corners.iter().all(|&i| values[i] >= 0.0)
                        || corners.iter().all(|&i| values[i] < 0.0)
                    { continue; }
                    for tet in TETS {
                        let nodes = tet.map(|local| corners[local]);
                        let inside: Vec<_> = nodes.into_iter()
                            .filter(|&node| values[node] < 0.0).collect();
                        let outside: Vec<_> = nodes.into_iter()
                            .filter(|&node| values[node] >= 0.0).collect();
                        if inside.is_empty() || outside.is_empty() { continue; }

                        // Vertices belong to grid EDGES, not individual cells.
                        let mut edge_vertex = |a: usize, b: usize| -> u32 {
                            let key = (a.min(b), a.max(b));
                            if let Some(&existing) = cache.get(&key) {
                                return existing;
                            }
                            let denominator = values[a] - values[b];
                            let t = if denominator.abs() < 1.0e-10 {
                                0.5
                            } else {
                                (values[a] / denominator).clamp(0.0, 1.0)
                            };
                            let p = positions[a].lerp(positions[b], t);
                            let new_id = surface.vertices.len() as u32;
                            surface.vertices.push(p.to_array());
                            surface.normals.push(self.normal(p).to_array());
                            cache.insert(key, new_id);
                            new_id
                        };
                        let mut candidates: Vec<[u32;3]> = Vec::new();
                        match inside.len() {
                            1 => {
                                let a = inside[0];
                                candidates.push([
                                    edge_vertex(a, outside[0]),
                                    edge_vertex(a, outside[1]),
                                    edge_vertex(a, outside[2]),
                                ]);
                            }
                            3 => {
                                let a = outside[0];
                                candidates.push([
                                    edge_vertex(a, inside[0]),
                                    edge_vertex(a, inside[1]),
                                    edge_vertex(a, inside[2]),
                                ]);
                            }
                            2 => {
                                let (a, b) = (inside[0], inside[1]);
                                let (c, d) = (outside[0], outside[1]);
                                let ac = edge_vertex(a,c);
                                let ad = edge_vertex(a,d);
                                let bc = edge_vertex(b,c);
                                let bd = edge_vertex(b,d);
                                candidates.extend([[ac,ad,bc],[ad,bd,bc]]);
                            }
                            _ => unreachable!(),
                        }
                        for mut tri in candidates {
                            let [a,b,c] = tri.map(|v| Vec3::from_array(surface.vertices[v as usize]));
                            let normal = (b-a).cross(c-a);
                            if normal.length_squared() <= 1.0e-14 * self.scale.powi(4) {
                                continue;
                            }
                            let centre = (a+b+c)/3.0;
                            if normal.dot(self.normal(centre)) < 0.0 {
                                tri.swap(1,2);
                            }
                            surface.triangles.push(tri);
                        }
                    }
                }
            }
        }
        if surface.triangles.is_empty() || surface.vertices.is_empty() {
            return Err("Anatomical volume did not intersect its preview bounds".into());
        }
        if surface.vertices.iter().flatten().any(|v| !v.is_finite())
            || surface.normals.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("Anatomical preview produced non-finite coordinates".into());
        }
        Ok(surface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn neutral_anatomical_regions_have_bilateral_symmetry() {
        let target = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        assert_eq!(target.volumes.len(), 15);
        for &y in &[0.33,0.43,0.58,0.72,0.84] {
            for &x in &[0.0,0.07,0.15,0.25] {
                for &z in &[-0.10,0.0,0.12] {
                    let p = Vec3::new(x,y,z);
                    assert!((target.field(p)-target.field(p*Vec3::new(-1.0,1.0,1.0))).abs() < 1e-5);
                }
            }
        }
        assert!(target.landmarks.iter().any(|p| p.name=="pubic_symphysis"));
    }

    #[test]
    fn target_is_continuous_and_girth_changes_thigh_field() {
        let normal = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        let mut wider = Recipe::default();
        wider.morphology.thigh_girth = 1.4;
        let big = AnatomyTarget::from_recipe(&wider).unwrap();
        let point = Vec3::new(0.23, 0.46, 0.0);
        assert!(big.field(point) < normal.field(point));
        assert!(normal.field(Vec3::new(0.0,0.79,0.0)) < 0.0);
        assert!(normal.field(Vec3::new(0.35,0.79,0.0)) > 0.0);
    }

    #[test]
    fn preview_is_nonempty_finite_and_has_paired_edges() {
        let target = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        let surface = target.sample_preview().unwrap();
        assert!(surface.triangles.len() > 500);
        let mut edges = HashMap::<(u32,u32),usize>::new();
        for tri in &surface.triangles {
            for &(a,b) in &[(tri[0],tri[1]),(tri[1],tri[2]),(tri[2],tri[0])] {
                *edges.entry((a.min(b),a.max(b))).or_default() += 1;
            }
        }
        assert!(edges.values().all(|&count| count==2),
            "Preview triangles should make one closed, manifold target");
    }

    #[test]
    fn landmark_names_and_preview_topology_are_not_game_rig_data() {
        let a=AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        assert!(a.landmarks.len()>=9);
        let generated=a.sample_preview().unwrap();
        assert_eq!(generated.vertices.len(),generated.normals.len());
        assert!(generated.triangles.iter().flatten()
            .all(|&i| (i as usize) < generated.vertices.len()));
    }
}
