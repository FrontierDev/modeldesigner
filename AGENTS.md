# Model Designer engineering guidance

- Use a standalone Rust workspace and native eframe/egui desktop app following `envmoddesigner` conventions. Do not couple the tool to Unity.
- `studio-core` owns all procedural geometry, versioned recipes, skeleton and influence calculations, validation and exports. The UI must never implement body generation.
- Preserve the **canonical vertex indexing and triangle connectivity across every recipe value**. Any topology migration must use an explicitly versioned generator, never silently change an existing recipe.
- Keep generator v1/v2 on their original 19-joint skeleton and generator v3 on its separate 21-joint hierarchy with clavicles. Never silently remap rig indices in saved recipes. A skeleton adjustment must not implicitly break recipe compatibility.
- Do not claim the generated implicit-field triangles are animation-ready retopology. Surface topology and influence quality still require real animation evaluation.
- Validate all recipe input, reject unsupported versions, and present explicit errors; do not silently substitute unrelated geometry.
- Save recipes atomically, protect unsaved changes, keep previews and exports consistent.
- Run local formatting/check/tests/clippy when toolchain is available. Do not trigger GitHub Actions without explicit authorization.
- Treat the authoring application as multi-model in design, while retaining humanoids as the only implemented model type until new generators exist.
- Keep user-facing build sliders minimal and aligned with the validated `studio-core::BUILD_CONTROLS` definitions. Do not duplicate slider bounds, change recipe units, or silently alter schema/generator versions.
- Keep mesh diagnostics in preview controls, not the player-style Build inspector. Avoid milestone copy and verbose explanatory labels in the UI.
- Define any new build parameter in the versioned recipe, shared `controls::BUILD_CONTROLS` contract and regional deformation engine. Keep IDs stable, validate bounds, and extend per-parameter topology regression tests.
- Keep legacy v1 recipes readable using neutral serde defaults for new optional morphology fields. Do not silently change geometry at all-neutral defaults.
- Use Player mode for `player_editable` controls, Advanced for all controls, and preserve consistent schema export via `controls::build_schema_json`.
- Generator v1 and v2 must keep separate canonical reference meshes. Never redirect an older saved recipe to the newer surface; preserve mesh vertex order, triangle indices and rig metadata version for legacy v1.
- Generator v2 adds simple facial features and improved body silhouette. Treat it as anatomical reference geometry, not a finished face or animation-ready retopology.
- Keep the top-right preview statistics panel consistent with envmoddesigner. The preview uses the raw triangle mesh path (not anti-aliased convex polygons) and smoothly interpolated per-vertex normals.

- Generator v3 revises only the upper torso/shoulder field and uses a 42×76×28 sampling grid with a regression budget below 26k triangles. Do not replace the v1/v2 fields, mesh arrays or rest skeletons.
- Chest shape should follow the sternum, clavicle, fan-shaped pectoralis and axilla, and shoulders should taper from acromion to humerus without detached spherical deltoids. See `docs/HUMANOID_SHOULDER_V3.md`.
