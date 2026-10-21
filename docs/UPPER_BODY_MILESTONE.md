# Upper-body topology milestone

## Objective

Prove that a human-like torso and upper arms can be **constructed procedurally from geometry rules**, with intentional edge flow through the shoulder joint rather than an SDF-blended collection of volumes.

## Current implementation

1. Construct a torso with 32 explicitly indexed vertices around each of 13 anatomical cross-sections.
2. Reserve a 4×4 quad hole on each lateral chest wall, aligned with the shoulder.
3. Reuse that hole's perimeter vertices as the starting boundary of a shoulder patch.
4. Loft **five** corresponding cross-sectional rings outward to each elbow: transitional axilla, deltoid cap, deltoid insertion, mid upper-arm and elbow. Stitch each to the preceding ring with quads.
5. Cap the temporary pelvis/neck/elbow ends for closed-manifold validation. These caps are study boundaries, not future human anatomy.
6. Derive clavicle, shoulder and elbow landmarks from the same measurements controlling the surface.
7. Export the quads and end-cap triangles with preliminary joint metadata and skin weights.

## What this proves

- Connected topology across torso–shoulder–arm junctions, with no overlapping primitive volume shells or independent stitched seam vertex copies.
- Deterministic quad/triangle indexing across supported slider values.
- Reusable mesh construction functions that can be used for quadrupeds, avians, tails, apparel and similar models.
- Closed, consistently oriented manifold output and numeric validation.
- Ability to evolve the model without importing a human base mesh.

## Shoulder refinement and pose testing

- Two intermediate anatomical rings soften the sharp transition between the rectangular logical torso patch opening and the elliptical deltoid.
- A shallow anterior pectoral plane tapers toward the axilla; the medial sternum has a slight depression.
- The wireframe viewport displays the authored quad boundaries, not internal preview triangle diagonals.
- Non-persistent pose controls rotate shoulder-patch loops progressively from the torso seam toward the elbow. Raise: 0–70°; forward reach: 0–55°.
- The neutral rig and mesh, not the posed preview, are exported.
- Rust tests check mesh closure, face-index determinism, bone count, pose geometry and reference influences.
- An independent Python reproduction of the geometric construction yielded 580 vertices, 512 quads and 1,120 triangles and confirmed oriented manifold edges and non-degenerate triangles at the neutral, 45° raise, 40° reach, 65°/45° and 70°/55° combinations. This is **not a substitute for Rust compilation or visual animation tests**.

## Remaining shortcomings

This study is **not yet an anatomical human model**. The shoulder openings are still rectangular in logical patch space and the cross-sectional contours are simplified. It does not include the head, full forearms, hands, pelvis-to-feet surface, scapular deformation, finished UV layout, joints weights or pose correction.

## Next milestone

Use the new interactive raise/reach preview to inspect the armpit, pectoral-to-deltoid roll and clavicle region from multiple angles. Refine the authored shoulder transition contours and add real bone skinning plus corrective deformation, then validate raised-arm, forward reach and torso rotation. Only after that should we expand to neck/head, pelvis and remaining limbs. Once successful, expand the same machinery to neck/head, pelvis and remaining limbs.

## Tests

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Review geometry in 3/4, front, side and back views and ensure no patch self-intersection at parameter extremes.
