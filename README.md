# Model Designer

Ninth Age procedural character modeller. Current humanoid anatomy comprises the neck stump, torso, upper arms, pelvis and upper thighs. The application now uses one pipeline instead of the retired handcrafted topology generator.

## Pipeline

1. **Anatomy target:** blended tissue fields and anatomical landmarks (anatomy.rs / torso_target.rs); temporary triangles are for visualization only.
2. **Fitted cage:** quad_template.rs derives neutral all-quad topology once; shoulder_patch.rs performs constrained neutral shoulder-edge rewires; quad_flow.rs refines neutral vertex distribution; cage.rs adjusts vertices to player morphology without changing indices.
3. **Rigging:** rig_preview.rs creates a provisional anatomical skeleton, assigns temporary normalized weights to the fitted cage and uses skinning.rs to pose it. This is for locating deformation defects, not production animation.

## Views and controls

- **Anatomy target:** continuous anatomical surface; optional landmarks and wireframe.
- **Fitted cage:** the automatic quad mesh, fit-strength, edge-flow diagnostics, wireframe and extraordinary poles.
- **Rigging:** the same fitted quad mesh with separate left/right arm and leg Outward and Forward pose sliders. Toggle Skeleton, Wireframe, Poles and Defects. Red markers flag inverted/collapsed faces or high edge strain.

Build parameters are stored in the recipe; pose angles are never saved. The quad topology is fixed across build changes. There is no old Topology mode or legacy mesh/rig export. Build Schema export remains available.

The rigging stage is a **diagnostic prototype**. Its 12-joint skeleton, continuous but simplified weights and pose transforms have not been validated for game animation. The mesh currently ends at elbows and knees. Further anatomy, joint-specific edge loops and production weights are still required.

## Core modules

| File | Purpose |
| --- | --- |
| anatomy.rs / torso_target.rs | Anatomical implicit surface and landmarks |
| quad_template.rs / shoulder_patch.rs / quad_flow.rs | Cached neutral quads, shoulder connectivity rewires and vertex flow optimization |
| cage.rs | Morphology-sensitive safe vertex fitting |
| rig_preview.rs / skinning.rs | Temporary skeleton, skin weights and posed deformation |
| mesh.rs | Indexed surface operations and validation |
| controls.rs / morphology.rs | Character build sliders and recipe parameters |
| studio-app/src/main.rs | Native viewport and editing UI |

## Local build

Run cargo fmt --all -- --check, cargo check --workspace, cargo test --workspace and cargo run -p modeldesigner --release from the repository root.

GitHub Actions remains **manual-only** and must not be triggered without approval. Native compilation and visual acceptance were not completed in this edit session.

See docs/ANATOMICAL_TARGET.md, docs/CAGE_FITTING.md and docs/RIGGING_PREVIEW.md.

### Rigging weight and diagnostic refinement

The provisional rig now blends normalised pelvis/spine/chest and left/right limb influences (up to four per vertex), with stronger weights at each actual limb root and smoother transitions into the shoulder/chest and pelvis/thigh regions. This replaces the earlier pelvis-plus-one-limb assignment that overstretched shoulders and the groin.

Rigging diagnostics distinguish stretched and compressed quad edges and show actual rest and posed lengths for the worst representative edge. Extremely short neutral edges are counted and the raw maximum ratio remains visible, so numerical outliers do not obscure meaningful deformation. The production topology, export restrictions and animation-ready status are unchanged.

### Neutral shoulder connectivity refinement

The first topology-aware pass now changes **quad connections** near the
anatomical shoulder and axilla, rather than only smoothing grid-derived
vertices. In the cached neutral template it identifies pairs of adjacent
quads, considers the two alternative splits of their six-sided boundary,
and accepts only changes that improve landmark-weighted pole distribution
and limb alignment while preserving outward face orientation, sensible
quad edge lengths and proximity to the anatomical surface.

Both left and right shoulder regions use mirrored anatomical landmarks.
The Fitted cage view reports the number of accepted rewires per shoulder
and the regional irregular-pole cost before/after. **Vertex/quad counts
stay constant**; their connectivity changes only at neutral template
construction, never on player slider input.

This is a deliberately conservative **local quad-patch surgery**, not yet
a fully authored set of concentric deltoid/armpit loops. Zero accepted
rewires are reported as zero; if the current surface-net mesh offers
no safe improving exchanges, the next step is a full guided boundary-loop
patch rather than relaxing manifold safety.

No GitHub CI workflow is run by this change.
