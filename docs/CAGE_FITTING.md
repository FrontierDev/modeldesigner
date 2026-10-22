# Anatomy-derived quad cage — first automatic quadrangulation pass

## What changed

The older cage fitter was using the 720-vertex/656-quad **legacy topology**
as its starting mesh, despite that topology's established defects. That
source has been replaced.

`studio-core/src/quad_template.rs` builds an entirely new neutral all-quad
surface from the signed-distance field returned by `AnatomyTarget`. The
neutral topology is cached in a `OnceLock` and is reused for all subsequent
morphology changes. No player slider regenerates the quad connectivity.

The user can compare three viewport modes:

- **Anatomy target:** temporary marching-tetrahedra triangle preview.
- **Auto quad cage:** new anatomy-derived, fixed-index quad mesh after fitting.
- **Topology:** the previous legacy quad study, kept for comparison.

Only the legacy topology view retains its historical OBJ/rig export. The
new automatic quad cage has **no verified rig, skin weights or UVs** and cannot
be exported as production character geometry.

## Surface-net extraction

- Sample the neutral SDF once on an approximately 25 mm cubic grid.
- Require positive field values on the entire grid boundary (surface must
  be completely inside; fail explicitly on clipping).
- For each cell with a sign transition, place one vertex at the mean of its
  crossing-edge intersection positions.
- For each sign-changing *grid edge*, connect its four neighboring cell
  vertices as a consistently oriented quad.
- Reuse the neutral quad indices and render-triangle split exactly for every
  subsequent character.
- Validate closure, oriented edge pairing, face areas and single-component
  connectivity before publishing the template.
- Never substitute the old 656-quad mesh as a fallback if extraction fails.

This is a **dual-grid/surface-net quadrangulation**, not a robust field-aligned
QuadriFlow/Instant-Meshes-style solution. Its edges reflect grid directions
rather than ideal deltoid, axillary, inguinal and knee loops.

## Shrink-wrap fitting

`cage::fit` now obtains `neutral_template()?.clone()` rather than calling
the legacy `generate(recipe)` function. Initial vertices scale with overall
height and then move towards the current recipe's anatomical SDF. Signed
distance and surface gradients drive projection, with conservative
tangential smoothing.

The fit protects the crotch and shoulder-armpit junction from excessive
movement. It checks face inversion, triangle areas and quad-edge distortion,
and backs off movements that break those constraints. It reports:

- Initial and final **vertex-to-field** mean absolute error.
- Maximum vertex error.
- Mean and maximum **triangle-centroid-to-field** absolute error.
- Accepted iterations and maximum vertex displacement.

These residuals are diagnostic field distances, not full bidirectional
Hausdorff-distance or animation-deformation guarantees.

## Important limitations

- All-quad topology is now **independent of the former flawed cage**, but
  does not yet follow anatomical feature lines.
- There are no guarantees of ideal pole placement or skeletal edge-loop flow
  around shoulders, pelvis and crotch.
- Region-aware meshing/quad alignment, UVs and final skinning remain future
  phases, along with surface-to-surface error and foldover evaluation under
  extreme slider values.
- Fitting a neutral template to substantially different body proportions can
  still distort some quads. Repeated runtime remeshing would violate the
  project's stable-topology requirement, so this should be solved with
  better neutral topology and constrained morphology fitting, not by changing
  face connectivity on slider input.

## Verification status

An independent local Python implementation of the same surface-net edge
ordering was exercised on a closed implicit sphere. It produced **2,528
vertices, 2,526 quads and 5,052 triangles**, with every edge occurring twice
in opposite directions, positive face orientation and no degenerate
triangles. This verifies that specific prototype, **not the Rust humanoid
implementation**.

Rust compilation and application preview have not yet been verified. No
GitHub Actions workflows were started; CI is manual-only. No new regression
suite was added in response to the user's preference for visual inspection.

Run locally:

```powershell
git pull --ff-only origin main
cargo check --workspace
cargo run -p modeldesigner --release
```

In **Auto quad cage**, enable Wireframe and compare quad counts, silhouette
and reported vertex/surface residuals against Anatomy target and legacy
Topology, both at default and broad-shoulder/high-thigh-girth settings.

## Anatomy-guided neutral quad vertex optimization

`studio-core/src/quad_flow.rs` now adds an optional conservative neutral
template stage immediately after closed surface-net extraction. The
optimizer uses **left/right shoulder–elbow and hip–knee** landmarks to create
four longitudinal direction fields. Near each upper limb's interior it
reduces misaligned tangent-edge components so existing edges follow either
the limb axis or its circumference. Contributions fade to zero at shoulder,
groin and temporary limb ends to avoid assigning a misleading single
direction to branching geometry.

Algorithmic safeguards:

- All neutral quad and triangle indices remain unchanged.
- Updates are tangential to the signed field, with bounded re-projection.
- Total displacement is limited relative to the original extracted neutral
  vertices.
- A global backtracking step rejects face orientation reversals, collapsed
  triangles and extreme edge length changes.
- A change is accepted only when the weighted limb-direction alignment
  error decreases. If no safe improvement exists, the original positions
  remain valid; no substitute topology is generated.

This runs **once** when the `OnceLock` neutral archetype template is
created. Runtime slider changes continue to use the same quad indices
through `cage::fit`.

The editor now exposes **Poles** in Auto quad cage mode. Yellow markers show
non-valence-4 vertices; red marks valence 6 or greater. Only front-facing
poles are shown. Neutral-template metrics include the alignment error
before/after optimization, accepted passes, irregular vertices, maximum
quad edge-length aspect and minimum interior corner angle. Existing
vertex-to-SDF and face-centroid fitting residuals remain available.

**Not yet implemented:** field-aligned connectivity reconstruction,
singularity relocation, shoulder/hip concentric animation loops,
validated skin weights, rigged output or normal/tangent-map generation.
This vertex-only pass can improve wireframe layout but cannot move
topological poles or fix problematic connections on its own.

Local visual validation: enable Wireframe and Poles, compare left/right
shoulders and thigh roots at neutral and high body-mass/girth settings.
Check actual before/after alignment numbers rather than assuming the
optimizer always accepts steps. No extra automated regression tests were
added and GitHub CI remains manual-only.
