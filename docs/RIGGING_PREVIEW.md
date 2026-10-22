# Rigging view — deformation diagnostics

## Goal

Make shoulder and hip topology failures visible while the humanoid is still incomplete. This is deliberately not a final skeleton, skin-weighting solution, collision simulation or rig export.

## Implementation

- Use the current fitted quad cage and unchanged face indices.
- Build a provisional 12-joint pelvis/spine/chest/neck plus bilateral shoulder/elbow and hip/knee hierarchy from named anatomical landmarks.
- Compute smoothly faded skin influences along each upper arm and thigh, with remaining weight assigned to the pelvis. Influences are normalized and rebuilt when body morphology changes.
- Use hierarchical linear-blend skinning for independently adjustable left/right arm and leg Outward (abduction) and Forward (flexion) sliders.
- Do not persist poses in character recipes. Reset pose to recover the neutral fitted cage.

## Diagnostics

The viewport can show the joint skeleton, quad wireframe, extraordinary poles and red markers for vertices on inverted/collapsed faces or excessively stretched/compressed edges. The statistics include inverted face count, collapsed face count, strained edge count, worst edge ratio and maximum displacement.

An inverted face can reflect insufficient topology, poor skin weighting or an unrealistic extreme pose. Correlate markers and poles before changing the neutral quad layout.

## Manual review

Test each arm alone and each hip alone, then both together. Inspect upper arms, axilla, groin and thigh roots. Repeat for narrow shoulders with broad chest and for extreme hip/thigh build sliders. Run cargo check --workspace and cargo run -p modeldesigner --release locally. GitHub CI remains manual-only.

Future: complete forearms, knees, lower legs, head, hands and feet; optimize anatomical joint loops and pole placement; develop validated skin weights and joint-dependent corrective deformation before enabling real exports.