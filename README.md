# Model Designer

A standalone, native Rust humanoid-model authoring prototype for **Ninth Age**, following the `envmoddesigner` workspace and eframe/egui three-panel UI conventions.

## First milestone

- One **canonical humanoid triangle topology** constructed *once* from a fixed implicit reference body and reused for all edits. Changing proportions **never remeshes or changes vertex indices**.
- Six live parameters: height, shoulder width, arm length, leg length, musculature, and body mass.
- A fixed, parented **19-joint humanoid A-pose skeleton**, updated to the current body proportions.
- Deterministic per-vertex nearest-segment skin influence metadata (four weighted joint indices per vertex) for subsequent rigging experiments.
- Shaded orbit/zoom/pan preview, wireframe and skeleton overlays; three-panel preset, preview, and inspector workflow.
- Versioned JSON recipes, New/Open/Save/Save As, and OBJ geometry plus companion `.rig.json` export containing named bones, parents, positions, and per-vertex influences.
- Tests for topology invariance, deterministic generation, skeleton hierarchy, skin-weight normalization, parameter validation and exported index ranges.

### Important boundaries

This is a **topology-preservation/authoring prototype, not a production retopology or skinned-animation system**. The canonical surface is *triangulated from an implicit field*; it is not a hand-crafted deformation-ready quad cage. The model is one indexed mesh, but joint edge flow and actual deformation/animation quality remain to be validated. Skinning influence metadata is preliminary; **OBJ does not encode a rig**, and exporting the companion JSON does not make an immediately playable Unity Humanoid asset. No textures, clothing, armour, animation, inverse kinematics or automatic LODs are included.

Body geometry is deliberately kept in `studio-core`, independent of Unity and the UI. The next step should be replacing the canonical implicit reference mesh with a hand-reviewed quad topology and validating skin deformation under a shared humanoid animation suite, without changing recipe semantics.

## Build

Rust **1.95+**:

```bash
cargo run -p modeldesigner --release
```

Validation:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Project layout

- `crates/studio-core`: recipe validation, canonical mesh, morphology, bones/influences, OBJ and rig export, tests.
- `crates/studio-app`: desktop UI, recipe lifecycle, preview and export dialogs.

All mesh and rig outputs are generated from the same validated recipe as the preview.