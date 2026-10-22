# Model Designer engineering requirements

- Maintain separate model-independent geometry operations and model-specific generator definitions.
- No imported humanoid mesh is a prerequisite. Signed-distance fields and marching tetrahedra are permitted **only for a temporary anatomical shape target**, never as the production fixed-topology, rigged humanoid mesh.
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
- **Pelvis construction rule:** Build the anatomical left/right 12-vertex thigh root contours *first*. Solve the connecting 3-boundary quad surface from the existing 32-vertex torso circumference and those roots. Do not restore flat Coons lids, perimeter skirts, horizontal root rings or attached cylinder shells.
- All pelvic harmonic relaxation is deterministic and acts only on interior quad vertices; pinned waist and thigh-root boundary coordinates must not change. Preserve hip/knee joint indices 10–13.
- Never allocate unused grid vertices in the two thigh openings. Validate all vertices are referenced by faces and each root is higher laterally than medially.
- The lower-body generator's anatomically tilted thigh root, bounded lateral hip contour and quad-patch harmonic fitting must remain a single connected manifold surface. Do not recreate the old skirt or generic thigh cylinders.
- Anatomical thigh contour variation (quadriceps anterior, hamstrings posterior, vastus lateralis outer, adductors medial) is a deformation of the existing 12-vertex rings. Keep its contribution zero at the proximal root and knee boundaries so topology and downstream shin connection remain unchanged.
- Only alter interior pelvic vertices for pubic/gluteal surface correctives; waist and thigh-root boundary positions must stay pinned. Preserve neutral rigging weights and the 720-vertex/656-quad/1,400-triangle reference topology until a deliberate topology milestone.
- The 12-vertex thigh socket contour MUST use the nonuniform 2×4 perimeter mapping, NOT equal angular sampling. Mirrored socket coordinates must produce symmetric left/right root positions at neutral shape, including maximum thigh girth.
- Preserve the two **fixed** alternative triangulation diagonals at waist-pelvis grid cells (7,0) and (0,7). Never choose those diagonals dynamically based on slider positions; preserve quad and triangle indices across all recipes.
- Constrain root and provisional knee medial radii by anatomical midline clearance, particularly for narrow hips + high thigh girth + narrow stance. Test the whole loft rather than just the socket opening.
- Thigh Girth is a **whole upper-leg circumference control**, not merely a muscle bulge. The medial half-gap is a collision constraint: shift the thigh's cross-sectional center outward when required, rather than saturating the entire radius with `min(center_x-gap)`.
- Preserve substantial neutral thigh volume and taper it smoothly to the knee. Every angular vertex in all root/loft rings must respond to increased girth, and the front/back depth must expand proportionally as well as the outer width.
- The groin surface should have a smooth pubic/adductor and inguinal transition, not a flat pelvis lid. Contour-only correctives must modify free patch vertices, leaving waist and thigh root boundaries unchanged.
- Maintain the **Anatomy target** as a separate unrigged visualization from the existing topology study. The default UI view may render its temporary triangles, but OBJ/rig exports must continue to use only the explicit fixed-topology path.
- `anatomy.rs` provides generic continuous tissue-volume evaluation and named humanoid landmarks. Later quad fitting must use the target's signed field and preserve cage connectivity and skin-compatible vertex correspondence; do not attach primitive legs beneath a flat pelvis lid.
- A preview mesh with temporary marching-tetrahedra triangles is not production topology. Test its finite normals and two-face edge pairing, bilateral symmetry, smooth tissue continuity and meaningful responses to hip/girth/stance/build sliders.
- GitHub Actions CI remains **manual-only**, and must not be run, enabled on pushes, or dispatched without explicit user approval. Local compilation/tests may be used when available.
- The anatomical **target** now includes continuous torso, shoulder, upper-arm and neck fields in `torso_target.rs`, smoothly unioned with the pelvis and thighs in `anatomy.rs`. Do not replace them with a single round torso ellipsoid or separate capsule meshes.
- Torso-target sections must remain vertically ordered and vary width, anterior/posterior depth and curvature with the existing recipe controls. Preserve a continuous waist-to-pelvis negative field.
- Preview sampling bounds and default camera must cover the whole neck-to-upper-thigh anatomy under supported size extremes. The geometry remains a **temporary isosurface**, not fixed topology; the existing quad generator and export route must not be altered for this milestone.
- For torso target sections, use shared, shape-preserving cubic derivatives at every adjacent section. Do not reintroduce per-segment smoothstep with zero tangents at intermediate knots.
- Couple the upper ribcage envelope and shoulder placement sufficiently to support wide-chest/narrow-shoulder combinations; do not represent the deltoid as a separately unioned sphere.
- Lower-abdominal target dimensions should respect both waist and hip morphology; iliac volumes must not create an abrupt shelf at the waist.

- The experimental fitter in `cage.rs` must preserve indexed quads,
  render triangles and caps. Prevent face inversion, extreme edge-length
  ratios and large movements around the crotch and axillary junctions.
  Its output must remain explicitly **unrigged and non-exportable** until
  anatomical correspondence and skin weighting are validated.
