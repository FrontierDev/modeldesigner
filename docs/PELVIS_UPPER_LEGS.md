# Pelvis and upper-leg generator — anatomical Y junction

## Problem fixed

The first implementation attached legs to a broad, horizontal pelvic lid. Its silhouette resembled a skirt over cylindrical legs, even after removing the extra skirt ring. Both constructions began with the lower torso surface and attempted to insert cylindrical legs afterward. They were topologically closed but visually incorrect.

The replacement is **boundary-driven and leg-first**: define the waist, the left upper-thigh root, and the right upper-thigh root, then derive a single curved pelvic surface that connects all three.

## Construction

1. Reuse the 32 existing lowest torso vertices as the waist boundary. Preserve the entire upper-body/shoulder generator.
2. Compute anatomical hip/femur landmarks from build parameters.
3. Create two **12-vertex curved thigh root contours** with centers beneath the hip joints. Their outside edges sit higher near the greater trochanter and their inside edges descend toward the perineum. The contours are *not planar*.
4. Reserve a stable quad-patch topology connecting the waist and the two thigh roots. This is topologically a pair-of-trousers/three-boundary surface; the two separate socket openings are part of its original topology.
5. Fix the three anatomical boundaries and solve the remaining pelvic vertices through deterministic quad-edge harmonic relaxation. This replaces the former flat Coons-patch lid.
6. Add a limited posterior gluteal contour to the *interior* of the patch while leaving the sewn boundaries fixed.
7. Sweep four successive 12-vertex upper-thigh loops **from each root contour**, down to a provisional knee plane. The loft smoothly blends from the curved proximal cross-section to the narrower knee.
8. Cap only the provisional knees and the existing neck/elbows. No mesh booleans, union cylinders, voxel remeshing, or imported base mesh.

The generator no longer allocates the two unused points that were hidden inside the old grid's thigh openings.

## Implemented controls

Hip width changes waist/pelvic outline and both thigh socket positions. Pelvic tilt changes the front/back pelvis profile and thigh root inclination. Glute volume controls the posterior root and hip fullness. Thigh girth modifies the anatomical thigh roots and knee loops. Leg length modifies the hip-to-knee segment in the current partial model, and stance width modifies the knee position relative to the hip. Height, body mass, and torso length are also inherited. The rest of the existing upper-body controls remain active.

## Skeleton and skinning

The original ten joints are retained without reindexing. Left hip/knee are 10/11 and right hip/knee are 12/13. Pelvic bridge points remain pelvis-bound; the thigh roots transition gradually from pelvis influence to upper-leg/hip influence. Hip posing and deformation correctives are **not** part of this implementation.

## Expected neutral counts

- **720 vertices**
- **656 quads**
- **1,400 render triangles** (including temporary end caps)
- **14 joints**

These are construction-derived counts, not yet verified by a Rust compiler in this environment.

## Regression criteria

- Exactly one closed, consistently oriented connected surface, including both hip junctions.
- No unused vertices or independent tube geometry.
- Thigh roots higher on the outside and lower at the crotch.
- Positive separation between the two inner thighs at every supported girth/hip-width combination.
- Every corresponding upper-leg ring vertex moves down towards the knee, never folding back.
- The same indexed faces and joint count across all build parameter combinations.
- No degenerate triangle areas, finite positions and skin weights summing to one.

An independent Python implementation of the 3-boundary harmonic surface passed **192 combinations** of hip width, thigh girth, glute volume, pelvic tilt, leg length and stance width. It reported oriented-manifold edges, nondegenerate triangles and separated thigh roots for those combinations. It is not a substitute for compiling and viewing the Rust model.

## Neutral pelvis/thigh contour refinement — 21 October 2026

The first viable leg-first pelvic construction still had an abrupt outer hip bulge, a sharply pitched crotch and largely cylindrical thighs. The following refinement **changes vertex positions only**, without modifying the patch graph:

- Reduced proximal root lateral radius from 0.083 to 0.074 of the metre reference, while retaining a proportional medial clearance bound.
- Reduced the lateral-to-medial thigh-root slope from 0.094 to 0.077 and slightly raised the centre of the root contour. This softens the V-shaped groin junction.
- Balanced the proximal front/back radii and added modest anterior pubic and posterior gluteal surface projection. All pelvic interior correctives remain constrained by fixed waist and leg-root seams.
- Replaced the uniform leg loft interpolation with an anatomical contour function. Quadriceps shape the anterior surface, hamstrings the posterior, lateral vastus the outer thigh and adductors the proximal medial thigh. These variations fade out towards the knee and are influenced by the existing musculature slider.
- Kept the existing four 12-vertex thigh rings, 14-joint skeleton, stable indices, temporary knee caps and active morphology controls.

**Local validation:** An independent Python reproduction of the modified surfaces passed 432 lower-body parameter combinations (the root/loft profile) and a further 144 combinations incorporating the revised pelvic free-vertex contour. In those sampled cases there were no unmatched oriented mesh edges, degenerate triangle areas or root-gap closures. These are *geometric checks*, not Rust unit-test execution or a guarantee that the anatomy now looks correct from every angle.

No GitHub Actions run was started; CI remains manual-only. To test the Rust implementation locally:

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo run -p modeldesigner --release
```

Inspect front, side, rear and three-quarter silhouettes in both solid and wireframe, including extreme hip-width, thigh-girth and musculature combinations. Hip posing and corrective skinning should remain out of scope until the neutral silhouette is accepted.

## Run local checks

```powershell
git pull origin main
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo run -p modeldesigner --release
```

GitHub Actions is **manual-only** and must not be triggered until the user explicitly authorizes it.

## Next milestone

Review the neutral pelvis and hip-to-thigh silhouette in the native viewport from front, profile and rear, with and without wireframe. In particular, ensure the torso-to-hip contour does not form a hem, the crotch looks like a saddle instead of a detached bridge, and the upper thighs follow the hip mass continuously. Only then proceed with independent hip skeletal rotation and correctives.

## Socket grid correction

The 2×4 thigh openings remove six strictly interior lattice vertices (three per opening), not two. The earlier version omitted only the central two and allocated four unreferenced vertices. The six interior points are now skipped entirely, bringing the construction-derived neutral model to 720 vertices, 656 quads and 1,400 renderer triangles. Tests assert both the exact omitted coordinates and that every generated vertex participates in a face. No automatic CI should be run.
