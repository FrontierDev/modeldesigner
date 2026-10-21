# Humanoid generator v3 — chest, clavicles and shoulders

## Scope

This milestone addresses the spherical shoulder caps and inflated pectoral blobs seen in v2. The reference is grounded in external anatomical diagrams of the shoulder girdle and superficial pectoral/deltoid muscles, but remains a procedural approximation, not a production anatomical sculpt.

References:
- [NCBI / StatPearls, superficial chest and shoulder muscles](https://www.ncbi.nlm.nih.gov/books/NBK525991/figure/article-26785.image.f1/)
- [US anatomy of the shoulder — pectoralis major and deltoid](https://pmc.ncbi.nlm.nih.gov/articles/PMC3553044/)
- [Clavicle anatomy, anatomy of the shoulder girdle](https://www.ncbi.nlm.nih.gov/books/NBK525990/)

## Generator v3

- Broader thoracic ribcage with a flatter front and a distinct taper into the waist.
- Pectoral regions modelled as gently rounded **flattened fans** from sternum towards the anterior axilla, not separate spherical masses.
- Clavicle shelf runs laterally from the upper sternum towards the acromion. Upper trapezius creates a posterior slope from neck to acromion.
- Deltoid represented by a downward taper from acromion to humeral shaft, not an ellipsoidal shoulder balloon.
- Limited subtraction around the armpit distinguishes pectoral and upper-arm surfaces without creating an open mesh.
- Head, hands and lower body are carried over as provisional geometry and **not** considered finished.

## Rig compatibility and versioning

| Generator | Skeleton | Reference field | Nominal triangles |
| --- | --- | --- | --- |
| v1 | 19 joints | Original implicit humanoid | 34,400 |
| v2 | 19 joints | v2 anthropomorphic reference | 50,120 |
| v3 (default) | 21 joints | Revised chest/shoulders | ~23,260 |

Only v3 introduces left and right clavicle joints. Each clavicle is parented to the chest, and its corresponding shoulder joint is parented to the clavicle. The rest of the limb chain is unchanged in meaning but gains new indices. No scapular bones have yet been introduced; this is sufficient for the next rig experiment, not guaranteed animation correctness.

The v3 bone order is:

```
hips
  spine
    chest
      neck
        head
      left_clavicle -> left_shoulder -> left_elbow -> left_wrist -> left_hand
      right_clavicle -> right_shoulder -> right_elbow -> right_wrist -> right_hand
  left_hip -> left_knee -> left_ankle
  right_hip -> right_knee -> right_ankle
```

Version 1 and 2 recipes continue using their respective exact indexed reference meshes and original 19-joint hierarchy. Never assume matching indices or skinning arrays across different generator versions. The rig metadata must export the selected generator version.

## Triangle budget

The v3 scalar grid is **42 × 76 × 28**, lower than the 60 × 106 × 44 v2 grid. An independent numerical sampling of the v3 field using its six-tetrahedra decomposition counted ~23,260 triangles. The core regression test enforces fewer than 26,000 triangles. Results from the Rust implementation and visual mesh quality should be verified locally.

This reduces a costly uniform-grid surface but **does not solve retopology**. The next major geometry milestone should transition toward a controlled topology cage with shoulder/axilla edge loops, rather than endlessly increasing SDF grid resolution.

## Validation

Run from the repository root:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p modeldesigner --release
```

Inspect neutral v3 from **front, three-quarter, profile and back**, toggling skeleton and wireframe. Check chest-to-deltoid continuity, armpit recess and clavicle prominence, plus all 30 build controls at low/high values. Load old v1/v2 recipes and verify exact mesh/rig compatibility.

**Not yet tested in Unity:** clavicle animation, extreme arm elevation, skin deformation, clothing or equipment fitting.
