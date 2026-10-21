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
- Upper-arm pose must retain the shared torso seam connectivity, mesh face indices, manifold closure and neutral skinning weights. The local chest seam may deform, but the sternum and opposite side should not move. Both arms are independently posed using hierarchical linear-blend skinning and inverse bind transforms.
- Store neutral, anatomical per-ring influence weights in the generator; do not regress to nearest-bone selection or re-author posed mesh geometry.
- Apply bounded corrective deformations only after skeletal skinning; calculate preview normals from the deformed surface. Exports always use neutral bind-pose data.
- Place clavicle pivots at the medial attachment near the sternum; maintain the shoulder as their child and distribute elevation through both joints.
- Avoid abrupt uniform ring weights across the deltoid. Blend weights angularly around the upper-arm ring and smoothly across nearby torso sections. Correct transverse shoulder volume relative to the posed bone frame.
- Shoulder socket loops must never move inward through their corresponding torso-hole boundary vertices. The first three transition rings enforce outward progression; keep regression tests for lateral ordering and adjacent-face creasing.
- Do not attempt to repair a geometric fold using skin weights alone. Investigate neutral patch construction and face orientation before adding deformation correctives.
- The humeral-head pivot is at 0.240 m lateral from the centre in the default neutral study, compared with the nearby torso contour at approximately 0.225 m. Keep the shoulder pivot appropriately medial rather than placing it at the deltoid silhouette.
- Smooth the authored *upper* socket surface using a bounded, masked quad-adjacency fairing pass; do not smooth the entire torso or the already-corrected lower axilla. Reuse the same operator for a smaller pose-space correction after skinning, using a neutral-reference mask.
- Track adjacent-face normals across the superior shoulder junction and validate independently posed arms as well as the neutral pose. Topology, neutral weights and vertex order must remain unchanged.

- The current humanoid study exposes **35 implemented sliders**, separated into Proportions, Physique, Neck, Torso, Chest, Shoulders and Arms. Do not expose unimplemented anatomy; control definitions, description metadata, recipe validation and JSON schema must agree.
- New sliders must alter actual surface geometry (or justified joint placement), retain quad indices, and be testable at both bounds without degeneracy. Zero effect is a bug, not a fallback.
- Preserve neutral morphology defaults when adding independent controls; do not change the Default shape inadvertently. Keep Player/Advanced filters and reset behaviour consistent.
- No automatic GitHub CI. `.github/workflows/rust.yml` uses `workflow_dispatch` only. Do not run the workflow or restore push/PR triggers without explicit user permission.
- Pelvic topology must branch the single 32-vertex torso bottom into two 12-vertex thigh boundaries using a connected 9×9 quad grid with two holes, never overlapping primitive cylinders.
- Keep hip and knee joint indices 10–13 after the 10 existing upper-body joints. Pelvis and thigh skin weights derive from the procedural patches. Preserve existing shoulder indices 0–9 and their controls.
- For this milestone, upper legs terminate at knees; knee caps are temporary. Do not claim full lower legs, gait-ready skinning or hip pose correction.
- Every build control added to the schema must genuinely alter geometry or rest joint positions; test bounds and topology invariance.

- Do not reinstate a separate outer pelvis skirt/flare. The pelvis grid must use the torso's existing bottom 32 vertices and bend its interior into the crotch, with first thigh rings interpolated from actual socket boundaries. Keep direct mesh connection and no extra perimeter ring.
