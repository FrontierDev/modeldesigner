# Model Designer engineering guidance

- Use a standalone Rust workspace and native eframe/egui desktop app following `envmoddesigner` conventions. Do not couple the tool to Unity.
- `studio-core` owns all procedural geometry, versioned recipes, skeleton and influence calculations, validation and exports. The UI must never implement body generation.
- Preserve the **canonical vertex indexing and triangle connectivity across every recipe value**. Any topology migration must use an explicitly versioned generator, never silently change an existing recipe.
- Keep the 19-joint named skeleton and parent indices stable for this generator. A skeleton adjustment must not implicitly break recipe compatibility.
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
