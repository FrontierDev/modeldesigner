# Anatomical target — first implementation

## Objective

Define the **desired neutral anatomical shape first**, then later fit a fixed-topology quad cage onto it. The old pelvis and thigh construction had stable face indices but unacceptable proportions, a planar crotch and persistent hip/groin folding. Further smoothing and manual offsets were not converging on realistic anatomy.

This milestone introduces a new core system in `crates/studio-core/src/anatomy.rs` and a separate **Anatomy target** mode in the native editor. It does *not* replace the existing quad generator or export it as a finished humanoid.

## Two deliberately separate outputs

| Output | Geometry | Intended use | Skin/UV/export |
| --- | --- | --- | --- |
| Anatomy target | Continuous implicit field; disposable triangulated isosurface | Review neutral anatomical silhouette and volume response | None; explicitly not exportable as a game mesh |
| Topology study | Existing fixed indexed quads and shoulder/hip skeleton | Regression comparison, existing recipe/OBJ/rig pathway | Existing provisional weights and rig remain unchanged |

Default editor mode is **Anatomy target**. Select **Topology** in the viewport for the old quad model. Anatomy mode shows optional named landmark markers and a wireframe of its provisional *triangles*, not quad topology. The top-right statistics distinguish the two modes.

## Data definitions

`AnatomyTarget::from_recipe` creates:
- Landmarks: waist, sacrum, pubic symphysis, left/right hip centres, greater trochanters and provisional knee endpoints.
- A pelvic core and lower abdominal volume.
- A pubic/adductor bridge representing anterior-midline soft-tissue continuity.
- Bilateral iliac, gluteal, thigh core, quadriceps, hamstring and medial adductor volumes.

Volumes are parameterized ellipsoidal primitives. Their field is approximately `(length((p-centre)/radii)-1)*min(radii)`. Tissue volumes are combined using a continuous polynomial smooth union with a finite blending width. The field is negative inside and positive outside.

This is a **procedural anatomical approximation**, not a biomedical simulator, CT reconstruction or detailed muscular attachment model. Muscles are simplified volume groups.

Implemented morphology dependencies include overall height, hip width, thigh girth, stance width, provisional leg length, pelvic tilt, glute volume, body mass, musculature and torso length. Parameter updates reconstruct the anatomical volumes and resample the surface.

## Preview extraction

The target is sampled in a bounded three-dimensional volume at approximately 26 mm intervals at reference height. Each cubic cell is split into six tetrahedra. Surface intersections are cached by **shared grid-edge identity** so neighboring cells reuse the same vertex. A signed-field gradient defines outward normals and fixes triangle winding.

A sampled target is a **temporary triangle mesh**. It has no permanent vertex indices or topology correspondence across recipe changes. It must *never* be implicitly converted into a rigged quad mesh or offered as a production OBJ.

Because surfaces sometimes pass extremely close to grid nodes, discarding all very small preview triangles introduced open cracks. A strict near-zero-area threshold preserves connectivity. Replacing this temporary visualization with a robust fixed-topology fitter is a later milestone.

## Validation

The Rust tests in `anatomy.rs` cover:
- bilateral symmetry of the neutral field;
- inside/outside sign checks;
- increasing thigh fullness with thigh-girth changes;
- responsiveness of every enabled pelvis and upper-leg morphology control;
- finite preview vertices and normals;
- two-face pairing of isosurface edges;
- named landmark and tissue-region construction.

An independent local Python/numerical reproduction of the corresponding ellipsoid field and marching-tetrahedra sampling at reference dimensions found a closed mesh with **6,138 shared vertices and 12,272 preview triangles**, with every edge paired. Independent sampled fields for narrow hips and high girth also produced closed surfaces using a separate isosurface extractor. These calculations are **not Rust compilation or native app visual acceptance**.

No GitHub CI was triggered. The workflow remains manual-only.

## Local test commands

```powershell
git pull --ff-only origin main
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo run -p modeldesigner --release
```

On launch, examine the anatomical target's front, profile and rear silhouettes. Toggle **Landmarks** to inspect the femoral-head/greater-trochanter/pubic regions. Change hip width, glute volume, thigh girth and stance width independently, then in combination. Compare against the original **Topology** preview, particularly at maximum thigh girth.

## Next milestone — not implemented

1. Accept or revise the neutral anatomical target surface based on visual references.
2. Author a low-density, anatomically distributed **fixed quad cage** for the lower torso/pelvis and both thighs. No dynamic remeshing under character sliders.
3. Fit cage vertices to the continuous field with constrained landmark correspondences, boundary and symmetry conditions; explicitly avoid foldovers and self-intersections.
4. Produce a stable topology and parameter-varying positions. Only then implement UVs, skin weights and hip articulation.

The next code step must **not** promote the temporary isosurface triangulation to final production topology.
