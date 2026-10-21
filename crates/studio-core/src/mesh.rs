//! Reusable topology-first mesh construction primitives.
//!
//! Generators build indexed surface loops and connect them explicitly. The
//! engine never performs voxelization or remeshing when parameters change.

use glam::Vec3;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct MeshBuilder {
    pub vertices: Vec<Vec3>,
    pub quads: Vec<[u32; 4]>,
    pub triangles: Vec<[u32; 3]>,
    pub caps: Vec<[u32; 3]>,
}

#[derive(Clone, Debug)]
pub struct SurfaceMesh {
    pub vertices: Vec<Vec3>,
    pub quads: Vec<[u32; 4]>,
    pub triangles: Vec<[u32; 3]>,
    pub caps: Vec<[u32; 3]>,
}

/// Deterministic masked fairing shared by neutral geometry and posed meshes.
fn relax_masked_vertices(
    vertices: &mut [Vec3],
    quads: &[[u32; 4]],
    mask: &[f32],
    iterations: usize,
    rate: f32,
    max_displacement: f32,
) -> Result<(), String> {
        if mask.len() != vertices.len()
            || !rate.is_finite() || !(0.0..=1.0).contains(&rate)
            || !max_displacement.is_finite() || max_displacement <= 0.0
            || mask.iter().any(|w| !w.is_finite() || !(0.0..=1.0).contains(w))
        {
            return Err("Invalid surface-fairing weights or parameters".into());
        }

        let mut neighbours = vec![Vec::<usize>::new(); vertices.len()];
        for &[a, b, c, d] in &quads {
            for (u, v) in [(a, b), (b, c), (c, d), (d, a)] {
                let u = u as usize;
                let v = v as usize;
                if u >= neighbours.len() || v >= neighbours.len() {
                    return Err("Surface fairing encountered an invalid quad index".into());
                }
                neighbours[u].push(v);
                neighbours[v].push(u);
            }
        }
        for ring in &mut neighbours {
            ring.sort_unstable();
            ring.dedup();
        }

        let bind = vertices.to_vec();
        for _ in 0..iterations {
            let before = vertices.to_vec();
            let mut next = before.clone();
            for (i, adjacency) in neighbours.iter().enumerate() {
                if mask[i] == 0.0 || adjacency.is_empty() {
                    continue;
                }
                let average = adjacency.iter()
                    .map(|&j| before[j]).sum::<Vec3>() / adjacency.len() as f32;
                let proposed = before[i] + (average - before[i]) * (rate * mask[i]);
                let offset = proposed - bind[i];
                next[i] = bind[i] + offset.clamp_length_max(max_displacement);
            }
            vertices.copy_from_slice(&next);
        }
        Ok(())
}

impl MeshBuilder {
    pub fn vertex(&mut self, position: Vec3) -> u32 {
        let id = self.vertices.len() as u32;
        self.vertices.push(position);
        id
    }

    pub fn loop_vertices(&mut self, vertices: impl IntoIterator<Item = Vec3>) -> Vec<u32> {
        vertices.into_iter().map(|point| self.vertex(point)).collect()
    }

    pub fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.quads.push([a, b, c, d]);
        self.triangles.push([a, b, c]);
        self.triangles.push([a, c, d]);
    }

    pub fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.triangles.push([a, b, c]);
        self.caps.push([a, b, c]);
    }

    /// Connect matching loops without allocating vertices along their seam.
    pub fn stitch(&mut self, from: &[u32], to: &[u32]) -> Result<(), String> {
        if from.len() != to.len() || from.len() < 3 {
            return Err("Mesh loops must have the same number of vertices (>= 3)".into());
        }
        for i in 0..from.len() {
            let next = (i + 1) % from.len();
            self.quad(from[i], from[next], to[next], to[i]);
        }
        Ok(())
    }

    /// Local smoothing without modifying authored vertex indices or quads.
    pub fn relax_masked(
        &mut self, mask: &[f32], iterations: usize, rate: f32, max_displacement: f32
    ) -> Result<(), String> {
        relax_masked_vertices(&mut self.vertices, &self.quads, mask, iterations, rate, max_displacement)
    }

    /// A triangle-fan closure. Used only for study boundary ends, not joints.
    pub fn cap(&mut self, loop_ids: &[u32], centre: Vec3, reverse: bool) -> Result<(), String> {
        if loop_ids.len() < 3 {
            return Err("Cap requires a loop of at least three vertices".into());
        }
        let centre_id = self.vertex(centre);
        for i in 0..loop_ids.len() {
            let next = (i + 1) % loop_ids.len();
            if reverse {
                self.triangle(centre_id, loop_ids[next], loop_ids[i]);
            } else {
                self.triangle(centre_id, loop_ids[i], loop_ids[next]);
            }
        }
        Ok(())
    }

    pub fn build(self) -> Result<SurfaceMesh, String> {
        let mesh = SurfaceMesh {
            vertices: self.vertices,
            quads: self.quads,
            triangles: self.triangles,
            caps: self.caps,
        };
        mesh.validate_closed()?;
        Ok(mesh)
    }
}

impl SurfaceMesh {
    /// Preserve topology while smoothing an already built / posed surface.
    pub fn relax_masked(
        &mut self, mask: &[f32], iterations: usize, rate: f32, max_displacement: f32
    ) -> Result<(), String> {
        relax_masked_vertices(&mut self.vertices, &self.quads, mask, iterations, rate, max_displacement)
    }

    pub fn validate_closed(&self) -> Result<(), String> {
        if self.vertices.is_empty() || self.triangles.is_empty() {
            return Err("Generated mesh must contain vertices and faces".into());
        }
        if self.vertices.iter().any(|p| !p.is_finite()) {
            return Err("Generated mesh contains non-finite vertices".into());
        }
        let mut edges = HashMap::<(u32, u32), (u32, i32)>::new();
        for face in &self.triangles {
            for &vertex in face {
                if vertex as usize >= self.vertices.len() {
                    return Err("Mesh face contains an invalid vertex index".into());
                }
            }
            let (a, b, c) = (face[0], face[1], face[2]);
            if a == b || b == c || c == a {
                return Err("Mesh face contains repeated vertex indices".into());
            }
            let area = (self.vertices[b as usize] - self.vertices[a as usize])
                .cross(self.vertices[c as usize] - self.vertices[a as usize]);
            if area.length_squared() < 1.0e-14 {
                return Err("Mesh contains a degenerate triangle".into());
            }
            for (u, v) in [(a, b), (b, c), (c, a)] {
                let key = (u.min(v), u.max(v));
                let edge = edges.entry(key).or_insert((0, 0));
                edge.0 += 1;
                edge.1 += if u < v { 1 } else { -1 };
            }
        }
        if let Some((edge, counts)) = edges.iter().find(|(_, counts)| counts.0 != 2 || counts.1 != 0) {
            return Err(format!("Mesh is not closed/oriented at edge {edge:?}: {counts:?}"));
        }
        Ok(())
    }

    pub fn normals(&self) -> Vec<[f32; 3]> {
        let mut normals = vec![Vec3::ZERO; self.vertices.len()];
        for &[a, b, c] in &self.triangles {
            let (a, b, c) = (a as usize, b as usize, c as usize);
            let n = (self.vertices[b] - self.vertices[a])
                .cross(self.vertices[c] - self.vertices[a]);
            for vertex in [a, b, c] {
                normals[vertex] += n;
            }
        }
        normals.into_iter().map(|n| n.normalize_or_zero().to_array()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unmatched_stitch_loops() {
        let mut builder = MeshBuilder::default();
        assert!(builder.stitch(&[0, 1, 2], &[3, 4]).is_err());
    }

    #[test]
    fn masked_fairing_keeps_indices_and_protects_untouched_vertices() {
        let mut b = MeshBuilder::default();
        for point in [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.3),
            Vec3::new(0.0, 1.0, 0.0),
        ] {
            b.vertex(point);
        }
        b.quad(0, 1, 2, 3);
        let face = b.quads.clone();
        let untouched = b.vertices[0];
        let old = b.vertices[2];
        b.relax_masked(&[0.0, 0.0, 1.0, 0.0], 4, 0.2, 0.05).unwrap();
        assert_eq!(b.quads, face);
        assert_eq!(b.vertices[0], untouched);
        assert!(b.vertices[2].distance(old) > 0.0);
        assert!(b.vertices[2].distance(old) <= 0.05001);
        assert!(b.relax_masked(&[0.0], 1, 0.2, 0.05).is_err());
    }

    #[test]
    fn posed_mesh_fairing_preserves_indices_and_fixed_vertices() {
        let mut builder = MeshBuilder::default();
        for vertex in [
            Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.2), Vec3::new(-1.0, 1.0, 0.0),
        ] { builder.vertex(vertex); }
        builder.quad(0, 1, 2, 3);
        let mut surface = SurfaceMesh {
            vertices: builder.vertices,
            quads: builder.quads,
            triangles: builder.triangles,
            caps: builder.caps,
        };
        let before = surface.triangles.clone();
        let untouched = surface.vertices[0];
        surface.relax_masked(&[0.0, 0.0, 1.0, 0.0], 4, 0.2, 0.010).unwrap();
        assert_eq!(surface.triangles, before);
        assert_eq!(surface.vertices[0], untouched);
    }

    #[test]
    fn closed_box_has_consistent_edges() {
        let mut builder = MeshBuilder::default();
        let v = [
            Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, 1.0), Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0), Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0), Vec3::new(-1.0, 1.0, 1.0),
        ];
        for p in v { builder.vertex(p); }
        builder.quad(0, 3, 2, 1);
        builder.quad(4, 5, 6, 7);
        builder.quad(0, 1, 5, 4);
        builder.quad(1, 2, 6, 5);
        builder.quad(2, 3, 7, 6);
        builder.quad(3, 0, 4, 7);
        let mesh = builder.build().unwrap();
        assert_eq!(mesh.quads.len(), 6);
        assert_eq!(mesh.triangles.len(), 12);
    }
}