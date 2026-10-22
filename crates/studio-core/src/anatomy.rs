//! Anatomy-driven surface target for future fixed-topology fitting.
//!
//! This is NOT the production character mesh, retopology, or a rig. Anatomical
//! volumes are blended into a continuous signed field, then sampled into a
//! disposable triangulated *preview* using marching tetrahedra. Persistent
//! vertex indices, skin weights and UVs must come from a later quad-cage fitter.

use crate::Recipe;
use crate::torso_target::TorsoTarget;
use glam::Vec3;
use std::collections::HashMap;

const PREVIEW_STEP_METRES: f32 = 0.030;
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
    torso: TorsoTarget,
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
        let abdomen_y = 0.856 + 0.055 * (m.torso_length - 1.0);
        let stance_displacement = 0.022 * (m.stance_width - 1.0);

        let mut landmarks = vec![
            Landmark { name: "waist", position: Vec3::new(0.0, waist_y, 0.0) * s },
            Landmark { name: "sacrum", position: Vec3::new(0.0, 0.74, -0.067) * s },
            Landmark { name: "pubic_symphysis", position: Vec3::new(0.0, 0.665, 0.072) * s },
        ];
        let mut volumes = vec![
            // The abdominal bridge follows the authored waist proportions.
            // Previously this ellipsoid used hip width alone and forced an
            // unrelated bulge into the narrower lower torso.
            tissue("pelvic_core", s, Vec3::new(0.0, 0.740 + y_shift, 0.0),
                Vec3::new(0.176 * hip_width, 0.148, 0.135)),
            tissue("lower_abdomen", s, Vec3::new(0.0, abdomen_y - 0.012, 0.018),
                Vec3::new(
                    0.143 * (0.65 * m.waist_width + 0.35 * hip_width),
                    0.152,
                    0.118 * m.waist_depth + 0.018 * mass,
                )),
            tissue("pubic_adductor_bridge", s, Vec3::new(0.0, 0.667 + y_shift, 0.051),
                Vec3::new(0.083, 0.073, 0.079)),
        ];

        for side in [-1.0_f32, 1.0] {
            let hip = Vec3::new(side * hip_x, 0.750 + y_shift, 0.0) * s;
            let thigh_x = leg_x + stance_displacement;
            let trochanter = Vec3::new(side * (hip_x + 0.032), 0.705, -0.005) * s;
            let knee = Vec3::new(
                side * (leg_x + 0.040 * (m.stance_width - 1.0)),
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
                    // Iliac volume should support the lateral hip, but
                    // stop below the waist so it cannot create a shelf.
                    s, Vec3::new(side * hip_x, 0.749 + y_shift, -0.008),
                    Vec3::new(0.091, 0.105, 0.115)),
                tissue(if side < 0.0 { "left_gluteal" } else { "right_gluteal" },
                    s, Vec3::new(side * (hip_x - 0.003), 0.709, -0.067),
                    Vec3::new(0.100 + 0.010 * mass, 0.123, 0.102 + 0.039 * glute)),
                tissue(if side < 0.0 { "left_thigh_core" } else { "right_thigh_core" },
                    s, Vec3::new(side * thigh_x, 0.528 + y_shift, -0.003),
                    Vec3::new(thigh_rx, 0.248 * recipe.leg_length, thigh_rz)),
                tissue(if side < 0.0 { "left_quadriceps" } else { "right_quadriceps" },
                    s, Vec3::new(side * (thigh_x + 0.003), 0.534, 0.046),
                    Vec3::new(0.079 * girth, 0.204 * recipe.leg_length,
                        0.071 * girth * (1.0 + 0.08 * muscle))),
                tissue(if side < 0.0 { "left_hamstrings" } else { "right_hamstrings" },
                    s, Vec3::new(side * (thigh_x + 0.005), 0.536, -0.049),
                    Vec3::new(0.077 * girth, 0.206 * recipe.leg_length,
                        0.070 * girth * (1.0 + 0.08 * muscle))),
                tissue(if side < 0.0 { "left_adductors" } else { "right_adductors" },
                    s, Vec3::new(side * (thigh_x - 0.026), 0.615, 0.009),
                    Vec3::new(0.057 * girth, 0.139, 0.073 * girth)),
            ]);
        }
        let torso = TorsoTarget::from_recipe(recipe);
        landmarks.extend([
            Landmark { name: "sternum", position: Vec3::new(0.0, 1.31, 0.125) * s },
            Landmark { name: "neck_top", position: torso.neck_top },
        ]);
        for i in 0..2 {
            let left = i == 0;
            landmarks.push(Landmark {
                name: if left { "left_shoulder" } else { "right_shoulder" },
                position: torso.shoulders[i],
            });
            landmarks.push(Landmark {
                name: if left { "left_elbow" } else { "right_elbow" },
                position: torso.elbows[i],
            });
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
            torso,
            blend_width: 0.044 * s,
        })
    }

    /// Negative inside anatomical tissue, positive outside.
    pub fn field(&self, point: Vec3) -> f32 {
        let mut distance = self.torso.field(point);
        for part in &self.volumes {
            // Broader union only around the overlapping lower torso, iliac
            // crest and abdominal volumes. The gluteal/leg contours retain
            // their existing, more localized transitions.
            let blend = match part.name {
                "pelvic_core" | "lower_abdomen"
                | "left_iliac" | "right_iliac" => 0.060 * self.scale,
                _ => self.blend_width,
            };
            distance = smooth_union(distance, ellipsoid_field(point, part), blend);
        }
        distance
    }

    /// Outward field gradient for constrained quad-cage projection.
    pub fn outward_normal(&self, p: Vec3) -> Vec3 {
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
        // Full upper body, including extended upper arms, waist and thighs.
        // Keep generous clearance from the sampled surface at every slider
        // extreme so the disposable isosurface remains closed.
        let min = Vec3::new(-0.73, 0.12, -0.42) * self.scale;
        let max = Vec3::new(0.73, 1.76, 0.42) * self.scale;
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
                            surface.normals.push(self.outward_normal(p).to_array());
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
                            // Tiny but nonzero triangles are essential to
                            // the edge-paired preview. Rejecting merely small
                            // faces creates cracks at near-grid-corner cuts.
                            if normal.length_squared() <= 1.0e-20 * self.scale.powi(4) {
                                continue;
                            }
                            let centre = (a+b+c)/3.0;
                            if normal.dot(self.outward_normal(centre)) < 0.0 {
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
    fn all_pelvic_and_thigh_controls_change_the_anatomical_field() {
        use crate::{BuildSection, BUILD_CONTROLS};
        let normal = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        let probes = [
            Vec3::new(0.00, 0.83, 0.05),
            Vec3::new(0.12, 0.77, -0.06),
            Vec3::new(0.12, 0.70, -0.17),
            Vec3::new(0.20, 0.48, 0.03),
            Vec3::new(0.14, 0.37, -0.09),
            Vec3::new(0.00, 0.68, 0.07),
        ];
        for control in BUILD_CONTROLS.iter().copied()
            .filter(|control| control.implemented()
                && matches!(control.section, BuildSection::Pelvis | BuildSection::Legs))
        {
            let mut recipe = Recipe::default();
            let alternative = if (control.max - control.default).abs() > 1e-5 {
                control.max
            } else { control.min };
            control.set(&mut recipe, alternative);
            let modified = AnatomyTarget::from_recipe(&recipe).unwrap();
            assert!(
                probes.iter().any(|&p| (normal.field(p) - modified.field(p)).abs() > 1e-5),
                "Anatomical target does not respond to {}",control.id
            );
        }
    }

    #[test]
    fn target_volumes_are_distinct_from_fixed_topology_or_skin_data() {
        let anatomy = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        assert!(anatomy.volumes.iter().any(|v| v.name == "left_adductors"));
        assert!(anatomy.volumes.iter().any(|v| v.name == "right_quadriceps"));
        assert!(anatomy.volumes.iter().any(|v| v.name == "left_gluteal"));
        let sampled = anatomy.sample_preview().unwrap();
        assert!(!sampled.triangles.is_empty());
        assert!(sampled.normals.iter()
            .flatten().all(|component| component.is_finite()));
    }

    #[test]
    fn pelvis_and_torso_are_one_continuous_anatomical_target() {
        let target = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        // Torso, waist, lower abdomen and pelvis must form a contiguous
        // negative-field interior through the body centreline.
        for y in [0.71, 0.78, 0.87, 0.99, 1.12, 1.28, 1.44, 1.53] {
            assert!(target.field(Vec3::new(0.0, y, 0.0)) < 0.0,
                "Target is disconnected at torso height {y}");
        }
        assert!(target.field(Vec3::new(0.0, 1.74, 0.0)) > 0.0);
        assert!(target.field(Vec3::new(0.49, 1.15, 0.0)) < 0.0,
            "Upper arm should be continuous");
        assert!(target.landmarks.iter().any(|p| p.name == "sternum"));
        assert!(target.landmarks.iter().any(|p| p.name == "left_shoulder"));
    }

    #[test]
    fn full_target_stays_inside_preview_bounds_at_extreme_proportions() {
        for extreme in [false, true] {
            let mut recipe = Recipe::default();
            if extreme {
                recipe.shoulder_width = 0.62;
                recipe.arm_length = 1.18;
                recipe.leg_length = 1.18;
                recipe.body_mass = 1.0;
                recipe.morphology.hip_width = 1.25;
                recipe.morphology.thigh_girth = 1.4;
                recipe.morphology.torso_length = 1.15;
                recipe.morphology.neck_length = 1.15;
                recipe.morphology.chest_width = 1.25;
                recipe.morphology.chest_depth = 1.30;
            }
            let target = AnatomyTarget::from_recipe(&recipe).unwrap();
            for i in 0..=8 {
                for j in 0..=8 {
                    let t = i as f32 / 8.0;
                    let u = j as f32 / 8.0;
                    for point in [
                        Vec3::new(-0.73, 0.12 + 1.64 * t, -0.42 + 0.84 * u),
                        Vec3::new(0.73, 0.12 + 1.64 * t, -0.42 + 0.84 * u),
                        Vec3::new(-0.73 + 1.46 * t, 0.12, -0.42 + 0.84 * u),
                        Vec3::new(-0.73 + 1.46 * t, 1.76, -0.42 + 0.84 * u),
                        Vec3::new(-0.73 + 1.46 * t, 0.12 + 1.64 * u, -0.42),
                        Vec3::new(-0.73 + 1.46 * t, 0.12 + 1.64 * u, 0.42),
                    ] {
                        assert!(target.field(point * target.scale) > 0.0,
                            "Anatomy target intersects the preview bounds at {point:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn full_torso_parameters_deform_target_without_regenerating_rig() {
        let neutral = AnatomyTarget::from_recipe(&Recipe::default()).unwrap();
        let mut broad = Recipe::default();
        broad.morphology.chest_width = 1.25;
        let wide = AnatomyTarget::from_recipe(&broad).unwrap();
        assert!(wide.field(Vec3::new(0.19, 1.30, 0.0))
            < neutral.field(Vec3::new(0.19, 1.30, 0.0)));
        let mut deep = Recipe::default();
        deep.morphology.chest_depth = 1.30;
        let deeper = AnatomyTarget::from_recipe(&deep).unwrap();
        assert!(deeper.field(Vec3::new(0.03, 1.23, 0.14))
            < neutral.field(Vec3::new(0.03, 1.23, 0.14)));
        let mut shoulders = Recipe::default();
        shoulders.shoulder_width = 0.62;
        let wider = AnatomyTarget::from_recipe(&shoulders).unwrap();
        assert!(wider.landmarks.iter().find(|p| p.name == "right_shoulder").unwrap()
            .position.x > neutral.landmarks.iter()
            .find(|p| p.name == "right_shoulder").unwrap().position.x);
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
