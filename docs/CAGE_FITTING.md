# First-stage fixed-topology anatomical cage fitting

## Scope

A preliminary cage fitter already existed in `crates/studio-core/src/cage.rs`
when this pass began. It was wired to the **Fitted cage** editor view.
This implementation retains that separation and strengthens the existing
algorithm's anatomical and geometric constraints.

The starting cage uses the older quad study's connectivity. It is **not yet**
an independently authored ideal full-body topology template. Fitting can
change vertex positions only; face counts, vertex ordering, quads and
triangles remain stable across slider combinations.

## Method

For each iteration:

1. Evaluate signed distance and outward gradient of the anatomical target
   at the current cage vertex.
2. Propose a bounded displacement along the negative signed-distance gradient.
3. Compute a tangential quad-neighbourhood fairing correction.
4. Attenuate candidate movements around the **crotch bridge** and
   **axillary/shoulder sockets**, the least tolerant regions of the seed.
5. Use global backtracking to reject the step when a face collapses,
   flips relative to the original cage, or quad edges undergo excessive
   compression or extension.
6. Stop at the current valid fit if further improvements would break those
   constraints; do not silently remesh or apply fallback geometry.

The deformation is bounded relative to the original cage, and
`FitReport` provides initial and final mean absolute signed-field errors,
maximum error, movement and accepted iterations.

## Preview

The editor provides **Anatomy target**, **Fitted cage**, and **Topology** modes.
The Fitted cage mode exposes a `Surface fit` strength slider and supports
wireframe/landmark inspection. It does not carry validated skin weights,
UVs or an exportable rig. The legacy topology preview and export path remain
separate.

## Limitations and acceptance

A good distance-field fit alone does not guarantee correct groin, shoulder or
joint edge flow. The seed cage still contains legacy anatomical layout issues.
This stage should be assessed visually in wireframe at neutral proportions and
at extreme build settings. The next milestone is to improve correspondence
and vertex distribution in troublesome anatomical regions, not to hide
foldovers with smoothing or dynamic topology.

No new automated regression tests were added for this step at the user's
request. Rust build and runtime execution were not available here.
GitHub Actions remains **manual-only** and was not triggered.

Local commands:

```powershell
git pull --ff-only origin main
cargo check --workspace
cargo run -p modeldesigner --release
```
