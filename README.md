# Model Designer

Native Rust procedural model authoring for Ninth Age. Humanoids are the first supported model type.

## Current humanoid

- **One editable humanoid generator** in `crates/studio-core/src/humanoid.rs`. No parallel or numbered generators.
- A connected neutral body reference with clavicular ridges, gently sloping pectoral surfaces, tapered deltoids and axillary transitions.
- A 21-joint skeleton including the left and right clavicles.
- 30 anatomical build controls and a shared exportable JSON control schema.
- A single **Default** preset. The Player and Advanced build panels edit the same morphology recipe.
- The 3D preview includes optional wireframe/skeleton overlays and top-right vertex, triangle, height and joint statistics.
- Mesh creation uses a 42 × 76 × 28 implicit sampling grid, targeting fewer than 26,000 triangles.
- JSON recipes, OBJ geometry and accompanying skinning/joint metadata are exportable.

## Limitations

The neutral form still requires anatomical inspection from multiple angles. Its triangulated field surface is **not animation-ready retopology**. Skinning weights, facial topology, joint articulation and equipment fitting are not yet production validated. Because the application is in prototyping, saved recipes from older generator implementations are **not migrated**.

## Run

Rust 1.95+:

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p modeldesigner --release
```

- `crates/studio-core` owns procedural mesh, rig, morphology, validation and export.
- `crates/studio-app` owns the desktop UI, viewport and file dialogs.
- `docs/HUMANOID_MORPHOLOGY.md` documents shared controls.
