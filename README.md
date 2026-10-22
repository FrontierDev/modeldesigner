# Model Designer

Native Rust procedural model designer for Ninth Age. The architecture supports multiple anatomy archetypes. Two **distinct preview paths** exist: an anatomical-volume target for exploring human form, and the older connected torso/pelvis/upper-leg quad study for topology and rig experiments.

## Architecture

- `studio-core/src/mesh.rs`: model-independent indexed quad patches, loop stitching, end caps, face triangulation, normal generation and closed/oriented-manifold validation.
- `studio-core/src/study.rs`: the first anatomical generator, building a torso from parameterized cross-sections, cutting explicit shoulder openings and stitching upper-arm ring surfaces directly into those openings.
- `studio-core/src/anatomy.rs`: anatomy landmarks and blended pelvis/thigh volumes, unified field and disposable triangulated preview.
- `studio-core/src/torso_target.rs`: tapered variable-section waist/ribcage/chest/neck profile, connected deltoid and upper-arm fields, and upper-body morphology controls.
- `studio-core/src/controls.rs`: shared parameter bounds and a machine-readable subset of currently implemented controls for future client UI reuse.
- `studio-app`: native editor with preview, recipe controls and OBJ export.

The previous SDF-as-final-mesh generator was removed. A **new, explicitly preview-only** anatomical field sampler is now available; its marching-tetrahedra triangles are not the character's permanent topology, UVs or skinning. **No reference mesh needs to be imported.**

## Anatomical target milestone

**The editor opens in Anatomy target mode.** This now shows a unified neutral target from the **neck through the torso, shoulders and upper arms to the pelvis and upper thighs**. The separate, older fixed-topology prototype is preserved for comparison. The control at the top of the viewport switches between **Anatomy target** and **Topology**. The target uses anatomical landmarks for the waist, sacrum, pubic symphysis, bilateral hip joints, greater trochanters and knees. It blends a cross-section-driven waist, ribcage, chest, back and neck with the shoulder and upper-arm fields and existing pelvic, gluteal, quadriceps, hamstring and adductor volumes into **one continuous implicit surface**.

The resulting triangulated preview is **temporary**: no fixed topology, skin weights, UVs, character export or animation-ready joints are claimed. The previous neutral quad study and its OBJ/rig export remain unchanged and are accessible from Topology mode. The Anatomy target mode intentionally blocks export to avoid presenting the disposable triangles as a game-ready model. Pose sliders apply only to Topology mode. Optional **Landmarks** and **Wireframe** overlays support anatomical review.

This phase deliberately focuses on **surface shape before retopology**. The immediate goal is to review the integrated front, side and rear body proportions—including the waist-to-pelvis transition—before constructing an authored fixed-topology quad cage. The anatomy preview is **not** a rigged or exportable quad model.

See [Anatomical target design](docs/ANATOMICAL_TARGET.md) for mathematical definitions, implementation limitations and validation steps.

## First milestone: upper-body study

The current output is intentionally **not a complete human**. It consists of a torso, neck termination, a pelvis, two upper arms ending at the elbows, and two upper legs ending at the knees. It demonstrates connected chest/shoulder/arm surfaces without remeshing, and the same topology under supported parameter changes. The current study has **14 anatomical joints**, including clavicles, hips and knees; skin weights remain provisional.

**35 functional build sliders** are available across nine anatomical groups: Proportions, Physique, Neck, Torso & back, Chest & pectorals, Shoulders & deltoids, Upper arms, Pelvis, and Upper legs. The editor provides Player/Advanced filtering, per-group resets, global reset and parameter tooltips. All supported values deform the current surface; future head, forearm and lower-body parameters remain hidden and cannot be set to non-neutral values.

The shoulder continues to use intermediate **transition and deltoid-insertion loops**. The connected pelvic split and upper thighs bring the neutral study to **720 vertices, 656 quads and 1,400 render triangles** (including temporary boundary caps). Wireframe mode shows **actual quad edges**, not rendering-only triangulation diagonals.

The preview now provides **independent left/right shoulder Raise (0–70°) and Forward (0–55°)** sliders. A shared `skinning.rs` engine evaluates hierarchical joint rotations using pose-world × inverse-bind matrices, and the anatomy generator supplies explicit torso/clavicle/shoulder skinning weights. Clavicles now pivot from their medial/sternal ends. Shoulder attachment weights vary continuously around the axilla and across nearby torso columns, while the deltoid correction preserves transverse radius relative to the posed upper-arm axis. The sternum and opposite arm remain stationary when only one side moves.

### Shoulder socket topology correction

The original 16-vertex arm-hole boundary was joined to an elliptical transition loop that could actually extend **inside** the chest cavity. This created inverted/overlapping patches and a visible armpit crease in neutral and raised-arm poses. The study now softly rounds the socket rim and enforces a lateral advance of 28, 16 and 8 mm (at reference height) across the first three connected rings. The mesh vertex/face counts are unchanged.

The neutral socket has a regression test that measures adjacent face-normal agreement, as well as checks enforcing outward ring progression.

### Superior shoulder revision

The remaining sharp upper crease was a **surface continuity** problem at the upper edge of the chest-to-arm patch. The current neutral model uses a controlled local quad-mesh fairing pass (four passes at 0.20 strength, maximum 16 mm displacement at reference height). A smaller fairing pass runs **after skinning for preview poses**, using the same neutral-space mask; it does not alter the exported neutral geometry or the lower armpit. The humeral-head joint is now seated approximately **23 mm more medially**, from 0.263 m to 0.240 m lateral of centre at default dimensions. Face counts and topology remain unchanged.

The independent neutral numerical reproduction measured the worst upper-shoulder adjacent face-normal dot product improving from about **0.35 to 0.69**. Sampled left-only and right-only raised/forward poses improved with the pose-space pass. This does not yet establish production-quality skinning or guarantee a crease-free appearance under every combination of poses.
 An independent numerical reproduction found the worst neutral shoulder-edge normal dot product improved from approximately **−0.95 to −0.20**; the neutral and sampled extreme poses retained finite nondegenerate triangles. **These figures are geometric diagnostics, not a visual acceptance test.**

**These are skeletal deformation tests, not a fully animation-ready character rig.** Recipe and OBJ/rig exports remain neutral, with inverse bind matrices and reference weights included in the rig metadata.

The mesh primarily contains quads and exports quad faces in OBJ. The preview triangulates those faces for drawing; end caps are triangles. A built-in check requires finite, nondegenerate, closed, consistently oriented geometry.

## Build and validation

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p modeldesigner --release
```

Once compiled, inspect the neutral study from front, profile and back, then vary shoulder width, chest width and upper-arm girth. This is only a **topological proof of concept**, not yet anatomically polished or animation-ready.

See [the milestone plan](docs/UPPER_BODY_MILESTONE.md).

## Morphology controls

See [Upper-body morphology controls](docs/UPPER_BODY_CONTROLS.md) for the full 29-parameter catalogue, numeric ranges, defaults and anatomy-specific behaviour.

## Pelvis and upper-leg milestone

The lower body is now constructed **leg-first**. Two anatomically tilted 12-vertex thigh root rings are placed at the hip landmarks (higher laterally, lower medially around the crotch). The intervening pelvis is solved as a three-boundary quad surface constrained by those two roots and the original 32-vertex waist ring. This replaces both the earlier flared skirt and the later flat pelvic underside. Four upper-thigh rings extend toward the temporary knee caps, with no separate attached tube shells. The crotch and hip contours emerge from the connected boundary geometry.

The current pelvic contour refinement reduces the abrupt outer-hip flare and eases the slope into the crotch. The same thigh loops now form anterior quadriceps, posterior hamstrings, lateral vastus and proximal medial adductor contours, tapering smoothly back to the existing knee boundary. A mild anterior pubic contour and posterior gluteal fullness are applied only to the free pelvic patch vertices; the waist and thigh seams remain fixed.

**Upper-leg proportions and anatomy:** The neutral thigh root and knee cross-sections have been enlarged to address the previously thin legs. The `Thigh girth` slider now expands both lateral width and anterior/posterior depth on **all five** root-to-knee cross-sections. Where the inner-thigh contour reaches its clearance limit, the cross-section shifts outward rather than discarding the requested growth. The groin gains a gradual pubic/adductor contour and subtle inguinal depressions; posterior gluteal mass is also increased. This remains a stylised anatomical approximation, not a clinical muscle model.

**Maximum thigh-girth correction:** The 12-vertex thigh sockets now use the corresponding rectangular patch perimeter directions rather than treating the vertices as evenly spaced in angle. Their left and right contours are exact mirrors. Two waist-pelvis corner quads use fixed alternative triangulation diagonals to remove sharply conflicting face normals, and a 25 mm per-side medial clearance at reference height keeps extreme thigh/hip/stance combinations from crossing the centreline. The authored 720-vertex/656-quad surface remains unchanged.

Six existing parameters are activated: hip width, pelvis tilt, glute volume, thigh girth, stance width and provisional upper-leg length. Bone chains now include hip and knee landmarks on each side. See [the pelvis and upper-leg implementation](docs/PELVIS_UPPER_LEGS.md) for details.

**CI is manual-only** (`workflow_dispatch`). No automatic GitHub Actions runs occur on commits. Local Rust checks may be run with `cargo check --workspace` and `cargo test --workspace`; the code is not considered runtime-verified until these succeed.

### Anatomical torso refinement

The target now uses shared-tangent cubic interpolation rather than per-band smoothstep. Upper chest breadth and shoulder placement are geometrically coupled, with deltoid shape integrated into a tapered upper arm rather than a separate sphere. Lower-abdominal tissue responds to waist width and depth, and the pelvis/iliac blend has been softened. No new sliders or topology changes were introduced; inspect the resulting appearance in **Anatomy target** mode.


## Experimental quad-cage fitting

The viewport now provides **Anatomy target**, **Fitted cage**, and **Topology** modes.
The fitter in `studio-core/src/cage.rs` starts from the existing indexed quad
study and moves only its vertex positions toward the continuous anatomical
field. It combines bounded signed-field projection with tangential quad-graph
fairing, adaptive backtracking and per-face inversion checks.

Fitting is deliberately restricted at the **crotch** and **axilla**, where the
seed topology is most vulnerable to folded loops. It also limits quad edge
stretch and compression relative to the unfitted cage, preserving the
original vertex and face indices. The UI exposes fit strength and numerical
surface-error diagnostics in the Fitted cage preview.

**This is an experimental first-stage cage fitter**, not an automatic
production retopology solution. Because the starting topology comes from the
older study, its edge layout may not yet correctly describe all anatomical
regions. The fitted output is unrigged and cannot be exported as a final
character. Inspect its wireframe and compare it against the anatomical
target before deciding where the template needs to be redesigned.

GitHub CI remains manual-only.
