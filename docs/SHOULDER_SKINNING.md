# Shoulder skinning and correction milestone

## Implementation

`skinning.rs` provides hierarchical joint-matrix evaluation, inverse bind matrices, linear-blend vertex skinning and posed joint transforms. It is model-independent and can be used by future non-humanoid generators.

The upper-body generator `study.rs` now authors **neutral geometry only**, together with named region influences and clavicle/shoulder corrective-loop metadata. No generated ring is rotated during topology construction.

**Bind influences** are based on the anatomical patch structure:
- The sewn torso boundary remains attached to the chest.
- The first deltoid-transition loop is dominated by the chest, with clavicle and small shoulder influences.
- The deltoid loop blends chest, clavicle and shoulder.
- The insertion loop is dominated by shoulder with a small clavicle contribution.
- Distal upper-arm and elbow loops follow the shoulder bone.
- Torso regions are assigned pelvis, spine and chest influences.

## Pose and corrections

`PreviewPose` carries independent left/right `ArmPose` values for arm elevation (0–70°) and forward reach (0–55°). A small portion of each rotation is assigned to the clavicle; the remainder to the shoulder. The hierarchy propagates transforms to the elbow. A limited shoulder-volume correction restores some of the loop radius lost to LBS near the deltoid and axilla, with a capped maximum displacement. These are **not finished corrective blendshapes**.

The neutral recipe and mesh are unchanged when dragging pose sliders. The preview mesh is evaluated from the original bind pose and recomputed normals every time. Neutral OBJ export and rig metadata include reference influences and per-joint inverse bind matrices.

## Validation

Tests cover identity bind transforms, hierarchical child rotation, stable weights and indices, independent arm motion, stationary torso seams, neutral rig export and validation of pose ranges. An independent numerical check of sampled left/right/extreme rotations found no degenerate triangle areas in the tested configurations; real Rust unit tests and application rendering remain to be run locally.

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
