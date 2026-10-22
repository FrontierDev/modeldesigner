# Anatomy-guided shoulder quad patch

## Status

The neutral cage now has a **structured annular shoulder patch** on both sides.
This replaces a connected band of surface-net quads with two new circumferential
support rings. It is a focused shoulder improvement, **not** complete
production-ready deltoid/chest/axilla retopology.

## Pipeline

1. Build the neutral anatomical signed field and surface-net quad cage.
2. Apply the existing small, field-constrained two-quad shoulder rewires.
3. Find a connected cylindrical shoulder band using the shoulder, elbow and
   estimated armpit landmarks. A candidate is eligible only when its removed
   faces have exactly **two simple, closed, oriented boundary loops**.
4. Classify the proximal loop as the **torso-side interface** and the distal
   loop as the **upper-arm interface**. Preserve all original seam vertices.
5. Build projected circumferential **deltoid** and **axillary support rings**.
   Zip the rings together with quads. When loop counts differ, place necessary
   count-transition poles preferentially away from the deep armpit.
6. Remove obsolete surface-net faces and orphaned interior vertices, rebuild
   render triangles and verify a closed, consistently oriented all-quad mesh.
7. Validate the **left shoulder first**, then apply the reflected anatomical
   construction to the **right shoulder**. Neither side is committed if the
   other side fails verification.
8. Run the neutral vertex-flow pass and cache the result. Morphology fitting,
   rig previews and pose sliders keep exactly those cached face indices.

There is no silent fallback to the automatic shoulder band if the boundaries
cannot be extracted or a candidate fails safety checks. The editor reports the
generation error. Geometry validation includes duplicate/repeated face-index
checks, minimum triangle area, quad aspect ratio, outward field normal
alignment, field proximity and oriented manifold edges.

## Diagnostics

The Fitted cage view reports removed/generated quad counts and torso/arm ring
sizes for each shoulder, in addition to the earlier two-quad rewire totals
and the final axillary pole cost. Rigging view continues to show deformation
defects, extraordinary poles and the unchanged fitted cage.

## Validation still required

This change has a quad-zipper seam test and a neutral-template test checking
that both patches exist and that the output is quad-only. The exact shoulder
deformation impact requires a native build and manual visual testing:

```sh
cargo fmt --all --check
cargo check --workspace
cargo test -p studio-core
cargo run -p modeldesigner --release
```

Check in Fitted cage mode that the rings wrap the shoulder/upper arm and that
the chest and back interfaces remain smooth. In Rigging mode compare:

- Neutral and asymmetric arms
- Left raised outwards by 30° and 60°
- Left raised forwards by 30° and 60°
- The mirrored right-side poses
- Wireframe + poles + defects, especially underarm collapse, red defects,
  upper-arm root volume and chest displacement

This execution environment has no local Rust compiler or GUI, so these native
build and pose checks have **not** been run. The GitHub Actions workflow
remains manual-only, and **GitHub CI was not triggered**.

## Limitations

The proximal boundary is the first safe closed arm/torso annulus, not a
hand-authored network extending into the sternum and scapula. These support
rings provide more intentional local circulation but cannot guarantee that
all axillary poles disappear, that 30°–60° motion improves, or that linear
blend skinning preserves volume. Finish visual assessment before expanding
this approach to a larger shoulder disk or the hip/groin.
