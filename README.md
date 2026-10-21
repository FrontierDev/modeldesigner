# Model Designer

Native Rust procedural model designer for Ninth Age. The architecture is designed to support multiple model generators; the first active generator is a **humanoid upper-body topology study**.

## Architecture

- `studio-core/src/mesh.rs`: model-independent indexed quad patches, loop stitching, end caps, face triangulation, normal generation and closed/oriented-manifold validation.
- `studio-core/src/study.rs`: the first anatomical generator, building a torso from parameterized cross-sections, cutting explicit shoulder openings and stitching upper-arm ring surfaces directly into those openings.
- `studio-core/src/controls.rs`: shared parameter bounds and a machine-readable subset of currently implemented controls for future client UI reuse.
- `studio-app`: native editor with preview, recipe controls and OBJ export.

The old SDF/marching-tetrahedra humanoid generator has been removed. **No reference model needs to be imported.**

## First milestone: upper-body study

The current output is intentionally **not a complete human**. It consists of a torso, neck termination and two upper arms ending at the elbows. It demonstrates connected chest/shoulder/arm surfaces without remeshing, and the same topology under supported parameter changes. The first study has 10 anatomical joints, including clavicles; skin weights remain provisional.

Ten working sliders: height, shoulder width, arm length, torso length, shoulder slope, chest width, chest depth, ribcage taper, waist width, upper-arm girth. Other planned controls are not presented; attempts to set them away from their neutral value in a recipe fail explicitly.

The updated shoulder connection adds intermediate **transition and deltoid-insertion loops**, allowing the torso-to-upper-arm cross-section to change more gradually. The first study now contains **580 vertices, 512 quads and 1,120 render triangles** in its neutral configuration. Wireframe mode shows **actual quad edges**, not rendering-only triangulation diagonals.

The preview toolbar includes non-persistent **Raise (0–70°)** and **Forward (0–55°)** controls. These evaluate a progressive shoulder-patch rotation and matching elbow landmarks to expose poor deformation. **These are topology stress tests, not final skeletal skinning.** Saving a recipe or exporting OBJ/rig metadata always uses the neutral reference regardless of the current preview pose.

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
