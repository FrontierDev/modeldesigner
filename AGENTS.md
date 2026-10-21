# Model Designer engineering guidance

- Use a standalone Rust workspace and native eframe/egui desktop app following `envmoddesigner` conventions. Do not couple the tool to Unity.
- `studio-core` owns all procedural geometry, versioned recipes, skeleton and influence calculations, validation and exports. The UI must never implement body generation.
- Preserve the **canonical vertex indexing and triangle connectivity across every recipe value**. Any topology migration must use an explicitly versioned generator, never silently change an existing recipe.
- Keep the 19-joint named skeleton and parent indices stable for this generator. A skeleton adjustment must not implicitly break recipe compatibility.
- Do not claim the generated implicit-field triangles are animation-ready retopology. Surface topology and influence quality still require real animation evaluation.
- Validate all recipe input, reject unsupported versions, and present explicit errors; do not silently substitute unrelated geometry.
- Save recipes atomically, protect unsaved changes, keep previews and exports consistent.
- Run local formatting/check/tests/clippy when toolchain is available. Do not trigger GitHub Actions without explicit authorization.
