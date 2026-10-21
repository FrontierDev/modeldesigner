# Pelvis and upper-leg implementation

## Purpose

Extend the procedural humanoid beyond the torso and shoulders without importing any anatomical mesh and without changing the geometry engine. The result is **one connected manifold surface**, not a collection of intersecting leg primitives.

## Topology construction

- Preserve the existing 13-row torso and its shoulder-to-upper-arm quad patches.
- Remove the temporary *bottom torso cap* and reuse its former centre vertex as the actual centre of the pelvic/crotch bridge. The preceding shoulder vertex indices are therefore retained.
- Shape the **existing 32-vertex lower torso ring** into the hip contour using hip width, pelvic inclination and posterior gluteal projection; **do not add a second skirt ring**.
- Build a **9×9 (8×8-cell)** transfinite/Coons-interpolated pelvic underside, with its 32 perimeter vertices reused directly from the torso. Lower the interior into a soft crotch saddle, keeping the outer perimeter anchored to the original torso topology.
- Omit two separated 2×4 blocks of quads from the underside. These openings are the left and right leg sockets; each hole has an ordered **12-vertex boundary**, with a continuous crotch strip between them.
- Loft four 12-vertex quad rings from each socket toward a temporary knee plane. The first rings interpolate the existing socket perimeter towards rounded thigh contours so that no independent cylinders or abrupt root rings appear. Each corresponding ring descends monotonically. Cap the open knees only for the current isolated study; no new vertices are created along sewn boundaries.
- Apply the existing closed/oriented-manifold check to the combined surface and preserve identical face indices under parameter changes.

This is a *procedural two-hole patch*, not voxelization, boolean unions, marching tetrahedra, or post-hoc retopology.

## Parameters

Existing values now enabled:

| Control | Stored range | Neutral | Function |
| --- | --- | --- | --- |
| Hip width | 0.8–1.25 | 1.0 | Pelvic width and hip-joint spacing |
| Pelvis tilt | 0–1 | 0.5 | Inclination of the lower pelvic section |
| Glute volume | 0–1 | 0.5 | Rear projection of the pelvic contour |
| Thigh girth | 0.7–1.4 | 1.0 | Circumference of proximal/distal upper leg rings |
| Stance width | 0.8–1.2 | 1.0 | Side-to-side knee offset relative to hip |
| Leg length | 0.82–1.18 | 1.0 | Temporary hip-to-knee segment length (not full leg yet) |

The upper-body controls remain active, resulting in **35 functional sliders**. The same recipe IDs are used by the desktop editor and exported game UI schema. Unsupported shin, ankle, foot and forearm controls are still hidden.

## Skeleton and skinning

- Preserve the original first ten joints and their indices.
- Add `left_hip` (10), `left_knee` (11), `right_hip` (12), `right_knee` (13).
- Pelvic and crotch surfaces remain bound to the pelvis/root. The first proximal upper-leg loop blends pelvis and hip, followed by predominantly hip-bound rings.
- The left and right arm pose system remains unchanged. Hip/knee posing and gait-corrective deformation are **not implemented yet**, and leg controls modify neutral anatomy only.

## Preliminary topology targets

- **726 vertices**
- **656 quads**
- **1,400 renderer triangles**, including temporary neck, elbow and knee end caps
- **14 joints**

These are derived from construction counts and should be confirmed by executing Rust unit tests locally.

## Verification

The included Rust tests check two distinct 12-vertex openings, 32-vertex perimeter reuse, four rings on each leg, anatomical sliders, joint endpoints, finite geometry, face-index invariance and manifold closure across several extreme body shapes.

An independent Python prototype of the same 8×8-cell, two-hole grid and thigh loft passed oriented-edge pairing and triangle-area checks at hip-width 0.8/1.0/1.25 and thigh-girth 0.7/1.0/1.4. **This does not replace compiling and testing the exact Rust implementation.**

Local commands:

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo run -p modeldesigner --release
```

GitHub CI was set to **manual-only** at the user's request and must not be triggered automatically.

## Next objectives

Evaluate the pelvis silhouette and crotch topology from front, back and side. Then add independent hip flexion/abduction with correct skeletal weighting and hip-volume preservation before extending below the knee. The torso-to-pelvis profile and upper-thigh stance should be visually checked at parameter extremes before proceeding.

## 2026-10-21 pelvis correction

The previous implementation created an extra **32-vertex lateral skirt below the torso**, then attached four elliptical thigh rings through a nearly flat, downward-facing underside. Even though the mesh was manifold, the silhouette looked like a flared garment hovering over separate legs.

That construction was removed rather than smoothed. The lower torso ring is now the actual perimeter of the pelvis. The interior of the pelvic quad grid drops towards the crotch, and the first thigh rings interpolate **from their socket boundaries** instead of appearing as independent cylinders. The waist/hip contour and the hip-width, tilt and glute controls are evaluated on the actual lower torso vertices. The revision removes 32 redundant skirt vertices and 32 quads while preserving the 14-joint skeleton.

Geometry was evaluated with a local independent Python reproduction at default and several extreme parameter combinations, including combined narrow hips/wide thighs and wide hips/narrow thighs. These sampled surfaces retained consistently paired directed edges and nonzero triangle areas. Rust compilation and the rendered viewport still require verification on the user's machine; those numerical checks are not a claim of completed anatomical realism.
