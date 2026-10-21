# Humanoid morphology

The application has **one editable humanoid mesh generator** and one **21-joint rest skeleton**.

The shared `BUILD_CONTROLS` defines 30 parameters, including their stable ID, range, neutral default, units, section and player-editability. The Player and Advanced authoring modes edit identical recipe data. `Export Build Schema` exposes these metadata to the future Unity character editor.

## Categories

- **Proportions:** height, shoulder width, arm length, leg length
- **Physique:** musculature, body mass
- **Torso:** torso length, shoulder slope, chest width/depth, ribcage taper, waist width, abdomen fullness, back curvature
- **Arms:** upper-arm girth, forearm girth, wrist width, hand size
- **Legs:** hip width, pelvis tilt, gluteal volume, thigh/calf girth, ankle width, foot length/width, stance width
- **Head/neck:** head size, neck length, neck width

## Surface and rig

The current neutral surface is in `crates/studio-core/src/humanoid.rs`. It uses a ribcage form, thin sloped pectoral fans, clavicular ridges, a neck-to-acromion trapezius transition, tapered deltoids and shallow axillary recesses. The 42×76×28 scalar grid is meshed into a single indexed triangle surface.

The clavicles connect chest → clavicle → shoulder → elbow → wrist → hand on each side. Spine and legs continue with their existing hierarchy. Skin weights are preliminary, and no full animation validation has been done.

There are no retained numbered generators or old-generation recipe migrations. Parameter changes preserve the indexed topology **within the current neutral reference**. Editing the reference generator code itself may change vertex and triangle indices.

Run tests locally with `cargo test --workspace`; visually inspect both shoulders from front, profile and raised-arm poses before considering the geometry final.
