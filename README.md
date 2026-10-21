# Model Designer

A standalone, native Rust procedural model-authoring application for **Ninth Age**, following the `envmoddesigner` workspace and eframe/egui three-panel UI conventions. Humanoids are the first supported model type; the model browser is designed for other generators to be added later.

## First milestone

- One **canonical humanoid triangle topology** constructed *once* from a fixed implicit reference body and reused for all edits. Changing proportions **never remeshes or changes vertex indices**.
- Thirty live build sliders: the original height, shoulder width, arm length, leg length, musculature and body mass controls, plus 24 region-specific morphology controls spanning torso, neck/head, arms/hands, pelvis and legs. Bounds, defaults and player-editability are defined centrally in `studio-core`.
- A fixed, parented **19-joint humanoid A-pose skeleton**, updated to the current body proportions.
- Deterministic per-vertex nearest-segment skin influence metadata (four weighted joint indices per vertex) for subsequent rigging experiments.
- Shaded orbit/zoom/pan preview, with optional wireframe and skeleton overlays in a preview toolbar. Three panels: model types and presets, preview, and character build.
- Versioned JSON recipes, New/Open/Save/Save As, and OBJ geometry plus companion `.rig.json` export containing named bones, parents, positions, and per-vertex influences.
- Tests for topology invariance, deterministic generation, skeleton hierarchy, skin-weight normalization, parameter validation and exported index ranges.

### Important boundaries

This is a **topology-preservation/authoring prototype, not a production retopology or skinned-animation system**. The canonical surface is *triangulated from an implicit field*; it is not a hand-crafted deformation-ready quad cage. The model is one indexed mesh, but joint edge flow and actual deformation/animation quality remain to be validated. Skinning influence metadata is preliminary; **OBJ does not encode a rig**, and exporting the companion JSON does not make an immediately playable Unity Humanoid asset. No textures, clothing, armour, animation, inverse kinematics or automatic LODs are included.

Regional morphology deforms the same indexed reference mesh, with structural offsets also applied to the skeleton's rest positions. Legacy v1 recipes with no `morphology` object load neutral defaults. The reference still comes from a triangulated implicit field rather than animation-ready retopology.

Body geometry is deliberately kept in `studio-core`, independent of Unity and the UI. The next step should be replacing the canonical implicit reference mesh with a hand-reviewed quad topology and validating skin deformation under a shared humanoid animation suite, without changing recipe semantics.

## Character customization UI

- **Models (left):** Humanoid is the only available generator. The browser can later expose other model types without crowding the build editor.
- **Build (right):** Player mode shows the curated in-game sliders; Advanced exposes all 30. Controls are grouped into Proportions, Physique, Torso, Arms & hands, Pelvis & legs, and Head & neck. Bounds and units come from `studio-core::BUILD_CONTROLS`; measurements use centimetres and unitless shape ratios display as percentages.
- **Preview (centre):** Model rotation, panning, zooming, optional technical overlays and Reset view. Technical information is kept out of the character-customization controls.
- **Files (top):** New, Open, Save, Save As, OBJ + rig export, and **Export Build Schema** (portable JSON parameter metadata).

The in-game editor can follow the exported JSON control schema and use the same recipe values. **Unity integration, runtime deformation and multiplayer validation are not implemented yet.**

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