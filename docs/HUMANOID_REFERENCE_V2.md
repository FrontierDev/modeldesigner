# Humanoid generator v2: anatomical reference and model statistics

## Goal

Improve the default humanoid's **recognizably human** silhouette and facial landmarks without breaking v1 recipes or introducing unvalidated retopology.

## Changes

- Head: cranium, jaw/chin, cheeks, brows, nose bridge/tip, lips, subtle eye and mouth recesses, ears.
- Torso: ribcage, waist, pelvis, soft pectoral and gluteal transitions.
- Arms: slimmer deltoid, tapered upper arm/forearm, wrist, palm and thumb.
- Legs: distinct thighs, knees, calves, ankles, heels and forward-facing forefeet.
- Finer reference sampling grid for v2 (60 × 106 × 44) with bounded extents.
- Area-weighted vertex normals recomputed after morphology changes for smooth preview lighting.
- Top-right preview statistics: model type, generator version, height, vertex count, triangle count and joint count.

## Compatibility

Generator v1 retains its original field, sampling grid and indexing. Saving/opening a v1 recipe does not migrate it. New recipes and presets use generator v2. The JSON recipe schema remains version 1. OBJ exports remain material-free; rig companion metadata records the **actual** generator version rather than the current default.

For either version, deformations preserve the canonical reference topology for that version. **Do not treat v1 and v2 vertex indexes as interchangeable.** Any future animation, clothing fitting or skin-weight authoring must be topology-version-aware.

## Limitations

The face, hands and feet are still simplified. Fingers, articulated facial topology, expression rigging, cloth, UVs and skin deformation validation remain future tasks. In particular, the underlying marching-tetrahedron triangles are not a substitute for deformation-friendly quad loops around joints.

## Validation

After pulling, run:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p modeldesigner --release
```

Review the neutral mesh from front, profile and back with wireframe off, and inspect facial features, shoulders, groin, thumbs, knees and feet. Confirm v1 saved recipes load without changing geometry and that model statistics reflect the selected version.
