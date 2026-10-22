# Shoulder connectivity refinement

## Status

First neutral-only topology improvement, not a complete final shoulder patch.

## Purpose

The baseline surface-net mesh contains clusters of extraordinary-valence
poles around the shoulder and axilla. Earlier limb-direction smoothing
changed only vertex positions, not quad connectivity, so it could not
move those poles.

## Method

1. Use the neutral anatomy's shoulder and elbow landmarks to construct
   mirrored left and right shoulder regions, and approximate the axillary
   high-strain area.
2. Build a closed-manifold quad edge incidence map.
3. Inspect each local pair of quads sharing an edge. Their oriented
   six-vertex boundary admits two alternative quad-pair reconnections.
4. Score each candidate for reduction of exceptional pole valence
   near the axilla, alignment of the replacement edge along or around
   the upper arm, and edge-length quality.
5. Reject duplicate connections, nonmanifold region candidates,
   degenerate or inward-facing triangles, excessive edge distortion,
   and patches far from the signed anatomical field.
6. Commit only safe beneficial rewires; rebuild affected render triangles.
   Keep every surrounding boundary and vertex index unchanged.
7. Validate the resulting closed oriented quad surface, then run the
   neutral vertex-flow optimization and cache the template.

The editor reports exact numbers of rewires per shoulder and the
weighted shoulder hotspot pole cost before and after. The old vertex and
quad counts are deliberately unchanged, although adjacency is different.
Player build sliders and temporary rig poses never regenerate topology.

## Validation

The code contains one small direct test that both alternatives of a
six-sided two-quad patch keep the same boundary and outward winding.
A separate geometric check using a planar regular hexagon confirmed
that both alternative subdivisions preserve the boundary and triangle
orientation. This check is not a native Rust build.

Local build and visual validation remain necessary:

    git pull --ff-only origin main
    cargo fmt --all
    cargo check --workspace
    cargo run -p modeldesigner --release

In Fitted cage mode, inspect wireframe and pole overlays, then in Rigging
mode inspect 30° and 60° one-sided shoulder poses and red defect markers.
GitHub CI was not run.

## Limitations

This is topology surgery, but not yet global field-aligned quadrangulation.
It does not guarantee concentric deltoid rings, complete pole relocation,
animation-ready shoulder deformation or validated skinning. The next
topology stage should only replace a larger shoulder disk after stable,
compatible arm-side/torso-side boundary loops are extracted and verified.
