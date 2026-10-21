# Model Designer engineering requirements

- Maintain separate model-independent geometry operations and model-specific generator definitions.
- No use of imported humanoid meshes as a prerequisite; no SDF/marching-tetrahedra humanoid generator.
- `crates/studio-core/src/mesh.rs` owns explicit indexed patch topology, loop stitching, face triangulation and mesh validation.
- `crates/studio-core/src/study.rs` constructs the first upper-body geometry from anatomical cross-sections and sewn shoulder-hole boundaries. Do not use duplicated seam vertices or separate capsule volumes for the shoulders.
- Keep one active, evolving generator, not numbered anatomy versions. Preserve only Default preset.
- Topology must remain identical across all implemented controls. Unsupported sliders must be hidden and their non-neutral recipe values rejected.
- Validate oriented manifold closure, no degenerate faces, no non-finite vertex positions, deterministic indices and skin weight normalization.
- Shared build schema contains only implemented controls; new anatomy stages explicitly expand the implemented list.
- Keep editor panels and raw indexed preview; top-right statistics must show quads as well as triangles.
- Do not claim human anatomical realism, production skinning, fully rigged humanoid or animation correctness for a torso-only topology study.
- Before merging further anatomy, run cargo fmt/check/test/clippy and evaluate silhouette, edge flow and bone movement.
- Preview wireframe shows quad boundaries, not internal render-only triangulation diagonals.
- Keep interactive preview pose values separate from `Recipe`; exporting OBJ and rig data always uses the neutral reference mesh.
- Upper-arm pose must retain the shared torso seam, consistent mesh face indices, manifold closure and neutral influence weights. Pose controls are a topology stress test, not production skinning.
