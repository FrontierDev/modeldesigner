# Rigging view — deformation diagnostics

## Goal

Make shoulder and hip topology failures visible while the humanoid is still incomplete. This is deliberately not a final skeleton, skin-weighting solution, collision simulation or rig export.

## Implementation

- Use the current fitted quad cage and unchanged face indices.
- Build a provisional 12-joint pelvis/spine/chest/neck plus bilateral shoulder/elbow and hip/knee hierarchy from named anatomical landmarks.
- Derive continuous pelvis/spine/chest trunk weights by vertical anatomical location, then smoothly blend up to four strongest trunk and bilateral limb contributions. The shoulder no longer defaults to the pelvis as its non-limb influence.
- Upper arms and thighs have stronger attachment to their actual shoulder/hip pivots. Contributions fade into the torso, while a protective central-groin weighting blend prevents opposing upper legs from completely monopolizing crotch vertices.
- Keep up to four deterministic, normalized skin influences per vertex. They are recalculated on morphology changes and always applied to the fitted cage.
- Use hierarchical linear-blend skinning for independently adjustable left/right arm and leg Outward (abduction) and Forward (flexion) sliders.
- Do not persist poses in character recipes. Reset pose to recover the neutral fitted cage.

## Diagnostics

The viewport can show the joint skeleton, quad wireframe, extraordinary poles and red markers for vertices on inverted/collapsed faces or excessively stretched/compressed edges. The statistics now distinguish **stretched versus compressed** quad edges and report the representative worst ratio with the actual rest/posed edge lengths in millimetres. Extremely short neutral edges are counted separately because their raw ratios can be misleading; the unfiltered worst ratio is still visible. A short reference edge with a genuinely large absolute change is still flagged.

An inverted face can reflect insufficient topology, poor skin weighting or an unrealistic extreme pose. Correlate markers and poles before changing the neutral quad layout.

## Manual review

Test each arm alone and each hip alone, then both together. Inspect upper arms, axilla, groin and thigh roots. Repeat for narrow shoulders with broad chest and for extreme hip/thigh build sliders. Run cargo check --workspace and cargo run -p modeldesigner --release locally. GitHub CI remains manual-only.

Future: complete forearms, knees, lower legs, head, hands and feet; optimize anatomical joint loops and pole placement; develop validated skin weights and joint-dependent corrective deformation before enabling real exports.

## Interpretation and limitations

This pass improves provisional weights, not topology connectivity. A 90° shoulder or hip pose can still produce volume loss from ordinary linear-blend skinning, even with correct pivots. Compare moderate (30° / 60°) poses against extreme angles (90° where permitted), and inspect Defects + Wireframe before deciding to restructure edge loops.

The worst *representative* ratio excludes edges shorter than 15% of the median neutral edge length (or below 1 mm). These edges remain included in a separate unfiltered maximum, and short edges with large absolute deformation remain marked. This separates near-zero reference-edge artifacts from significant topological strain without concealing the raw measurement.

No GitHub CI was run. No new regression test suite was added; native compilation and visual results require local verification.
