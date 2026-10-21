# Shoulder skinning and correction milestone

## Implementation

`skinning.rs` provides hierarchical joint-matrix evaluation, inverse bind matrices, linear-blend vertex skinning and posed joint transforms. It is model-independent and can be used by future non-humanoid generators.

The upper-body generator `study.rs` now authors **neutral geometry only**, together with named region influences and clavicle/shoulder corrective-loop metadata. No generated ring is rotated during topology construction.

**Bind influences** are based on the anatomical patch structure:
- The sewn torso boundary blends chest, clavicle and a small shoulder contribution over nearby torso rows and columns; only the active side moves.
- The first deltoid-transition loop is dominated by the chest, with clavicle and small shoulder influences.
- The deltoid loop blends chest, clavicle and shoulder, with individual vertices weighted differently around its circumference: the upper deltoid follows the humerus more, while the anterior/inferior axilla stays attached to the chest longer.
- The insertion loop is dominated by shoulder with a small clavicle contribution.
- Distal upper-arm and elbow loops follow the shoulder bone.
- Torso regions are assigned pelvis, spine and chest influences.

## Pose and corrections

`PreviewPose` carries independent left/right `ArmPose` values for arm elevation (0–70°) and forward reach (0–55°). The clavicle pivots near the sternoclavicular attachment (rather than the lateral shoulder); approximately 18% of the preview rotation is assigned to the clavicle and the remainder to the shoulder. The hierarchy propagates transforms to the elbow. A bounded shoulder-volume correction restores lost cross-sectional radius perpendicular to the moving upper-arm axis, using each ring's skinned centroid; the maximum displacement is 10 mm. It does not modify the torso seam or expand an already-full ring. These are **not finished corrective blendshapes**.

The neutral recipe and mesh are unchanged when dragging pose sliders. The preview mesh is evaluated from the original bind pose and recomputed normals every time. Neutral OBJ export and rig metadata include reference influences and per-joint inverse bind matrices.

## Validation

Tests cover identity bind transforms, hierarchical child rotation, stable weights and indices, independent arm motion, localized moving torso seams with a stationary sternum/opposite side, neutral rig export and validation of pose ranges. An independent numerical reproduction checked the neutral, left-only, right-only and combined 70° raise/55° forward configurations (580 vertices, 512 quads, 1,120 triangles), finding no degenerate triangles. At 60° raise/35° forward the active upper chest seam moved around 9 mm while the central chest remained stationary; real Rust unit tests and application rendering remain to be run locally.

Run:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p modeldesigner --release
```

In the viewport compare neutral, left-only, right-only and mixed arm poses from front, side and rear. Look for collapsing surfaces, pinched armpits, excessive bulging and visible left/right movement coupling.

This study still has **no full humanoid, no animation clips, no mesh self-intersection test, no verified production skinning, and no Unity integration**.
