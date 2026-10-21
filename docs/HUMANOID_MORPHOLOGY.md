# Humanoid morphology — first regional authoring milestone

## Scope

The humanoid uses one deterministic reference surface **per generator version**, with a 19-joint rest skeleton for versions 1–2 and a 21-joint clavicle-enabled skeleton for version 3. This milestone extends the six original controls to **30**, keeps original values and serialized keys unchanged, and adds an optional `morphology` object with 24 new parameters. A legacy recipe missing that object uses neutral defaults. Its `schema_version` remains `1`. New recipes default to generator `3`; saved generator `1` and `2` recipes continue to use their original reference mesh and rest skeleton.

## Parameter sections

| Section | Parameters |
| --- | --- |
| Proportions | height, shoulder_width, arm_length, leg_length |
| Physique | musculature, body_mass |
| Torso | torso_length, shoulder_slope, chest_width, chest_depth, ribcage_taper, waist_width, abdomen_fullness, back_curvature |
| Arms & hands | upper_arm_girth, forearm_girth, wrist_width, hand_size |
| Pelvis & legs | hip_width, pelvis_tilt, glute_volume, thigh_girth, calf_girth, ankle_width, foot_length, foot_width, stance_width |
| Head & neck | head_size, neck_length, neck_width |

The fields are stored in their native units. Height/shoulder width are metres internally and displayed in centimetres; unitless proportional controls use ratios or 0–1 range internally and are displayed as percentages.

## Data and UI contract

`studio-core/src/controls.rs` defines stable IDs, range limits, neutral defaults, section names, display units and player-editable flags. It is the single metadata source for recipe validation, desktop controls and `Export Build Schema`. The export includes all 30 controls, even those hidden in Player mode.

Player mode presents a curated subset. Advanced mode presents all 30 without changing the saved data representation. These are authoring-mode filters only; the game has not yet consumed the schema.

## Mesh and joint behavior

- Each versioned canonical reference mesh is constructed once. Morphology only adjusts existing vertex positions; triangle indices and vertex counts are stable.
- Structural shape adjustments (torso length, neck length, shoulder slope, hip width, pelvis tilt and stance width) also move skeletal rest landmarks.
- Other sliders apply localized symmetric surface offsets around anatomical regions. No automatic re-topology or animation-corrective skinning is performed.
- Initial nearest-segment skin weights still refer to the existing 19-joint hierarchy and are not automatically optimized for new body shapes.

## Validation and next work

`cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` should pass before treating the milestone as validated.

Tests cover parameter uniqueness/defaults/ranges, JSON backward compatibility and round trips, finite vertex data, fixed connectivity, per-control visible displacement, skeletal response for structural controls, and the exported UI metadata.

This is a **reference-cage deformation milestone**, not anatomically precise retopology, production skinning, or a fully playable Unity humanoid. Later work must validate extreme poses, self-intersection, limb articulation, facial topology and equipment fitting.

## Later anatomy revision

Humanoid generator v3 adds clavicles, flatter pectorals, a reduced deltoid silhouette and a defined axillary recess, with a constrained mesh budget. See [HUMANOID_SHOULDER_V3.md](HUMANOID_SHOULDER_V3.md). Legacy recipes are not migrated automatically.
