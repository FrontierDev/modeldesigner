# Model Designer engineering instructions

- One active pipeline: anatomy target -> cached neutral auto-quads -> morphology fit -> provisional rigging diagnostics. Never reintroduce the deleted handcrafted torso/pelvis study generator or its separate Topology viewport.
- Generate anatomy first, fit a fixed neutral quad template to each morphology recipe. Do not regenerate quad connectivity on slider changes.
- The anatomy sample triangles are disposable. Only the quad cage can be used by the rigging diagnostic.
- Reusable mesh validation and hierarchical linear-blend skinning should stay in shared core modules.
- Rigging previews must use **the actual fitted cage**, with a skeleton derived from current anatomical landmarks; joint poses are independent, ephemeral and never stored in Recipe.
- Display deformation defects (inverted faces, collapsed faces, excessive edge stretch) rather than claiming animation readiness. Keep wireframe, skeleton, pole and defect overlays.
- Current anatomy stops at the neck, elbows and knees. Do not claim head/hand/foot or gait-ready rigging.
- The preview skeleton and weights are provisional. Do not export auto-quad rigs or present them as production-ready geometry.
- Maintain stable face/vertex indices under morphs and poses. Posing must not rebuild the neutral template; normals should be recomputed after skinning.
- Do not introduce extra public presets. Preserve Default, parameter definitions, Player/Advanced filters, build-schema export and top-right mesh statistics.
- When generation fails, report the failure rather than silently showing obsolete fallback meshes.
- Prefer the user's manual visual slider/pose evaluations for this milestone; no substantial new regression suite requested.
- **Do not run GitHub CI without explicit instruction.** The workflow must remain workflow_dispatch-only, with no push/PR triggers.
- Validate local Rust builds when possible. Record honestly when the native build cannot be run.
- Provisional rig weights are continuous, normalised, multi-bone influences: distribute trunk mass among pelvis/spine/chest and limb roots according to anatomical proximity. Do not revert to the former pelvis + strongest-single-limb weighting.
- Defect diagnostics must report stretched and compressed edges independently, identify unusually short neutral edges, and retain visibility of raw ratios and true measured edge lengths. Do not dismiss meaningful absolute deformation merely because an edge is short.
- Continue using the same quad indices in every posed frame; changes to rig weights must not change character morphology parameters or silently create new topology.
- `shoulder_patch.rs` is the first neutral-only topological stage. It must
  change actual quad connectivity (two-quad six-sided boundary rewrites),
  not merely reposition vertices. Keep accepted edits constrained by
  anatomical landmarks, outgoing SDF normals, face area, field proximity,
  valence penalty and reasonable quad proportions.
- Perform shoulder connectivity refinement only when building the cached
  **neutral** template, prior to `quad_flow`. After editing face indices,
  update both triangles associated with each quad and validate closed,
  oriented topology. Never rewire for live morphology or pose sliders.
- Do not claim the current local rewire pass creates complete concentric
  shoulder loops or animation-validated skinning. If patch rewires are
  blocked, surface the accepted count rather than substituting unsafe
  topology or removing checks.
- `shoulder_rings.rs` is a second neutral-only patch stage after the local
  shoulder rewires and before `quad_flow`. It removes a closed annular shoulder
  band, preserves the original torso and arm boundary vertices, inserts two
  circumferential support rings, and compacts unused surface-net vertices.
- Try the left anatomical patch first, validate the closed oriented mesh,
  then construct the mirrored right side. If either side has no unambiguous
  safe annulus, return a visible generation error; do not quietly retain the
  unpatched automatic band.
- Validate indexed quad winding, manifold triangle edges, nonzero triangle
  area, anatomical surface proximity, outward normals and duplicate faces.
  Put count-transition poles as far from the axillary hotspot as possible.
- Neither local rewires nor the structured annulus imply complete production
  chest-to-deltoid flow, approved weights, or measured deformation improvement.
  Validate moderate one-sided and asymmetric shoulder poses in Rigging view.
