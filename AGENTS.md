# Model Designer engineering guidance

- Work in the Rust workspace and follow existing native eframe/egui desktop conventions.
- Keep procedural geometry, recipes, skeleton and export logic in `studio-core`. UI logic belongs in `studio-app`.
- Maintain **one working humanoid generator** in `crates/studio-core/src/humanoid.rs`. Do not add numbered generator versions or backward-compatibility branches during prototyping.
- Keep only **Default** in the presets panel.
- Maintain the 21-joint skeleton, including both clavicles between chest and shoulder joints.
- Keep the pectoral region integrated with the ribcage and the deltoid continuous with the humerus. Avoid rectangular pectoral shelves, spherical shoulder blobs and hard axillary gouges.
- Validate triangle indices, normals, finite vertex values, skinning influences and preserved topology under build controls.
- Use `BUILD_CONTROLS` as the single source for the desktop slider bounds, units, defaults and JSON build schema.
- Keep the raw indexed triangle preview to avoid anti-aliased polygon-spike artifacts, plus top-right model statistics.
- Prototype meshes may change in place; do not claim backward compatibility or animation-ready topology.
- Run `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` when the Rust toolchain is available.
- Avoid unnecessary fallbacks and unnecessary text in the authoring UI.
