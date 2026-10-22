# Fitted quad cage

The neutral humanoid anatomy is sampled to create a quad surface-net template. Its indices are cached, and the quad-flow stage moves neutral vertices towards upper-limb directions without rewriting connectivity. The previously flawed 656-quad study was deleted.

At the current partial-humanoid proportions the new template has approximately 4,000 quads. Surface nets do not automatically provide correct skeletal edge loops.

For player morphology, cage.rs first transports upper-arm/thigh regions by their shoulder-elbow and hip-knee landmarks. Vertices then move toward the target signed field with local face-inversion, triangle-collapse and edge-distortion checks. One problematic quad should not veto movement elsewhere.

The **Fitted cage** view displays surface errors, fit-strength, wireframe and pole diagnostics. No skin weights are authored by the cage fitter. The **Rigging** view uses exactly these fitted vertex positions and the same quad connectivity.

This is not production retopology or an exportable rig. Future passes must reorganize neutral connectivity around armpits, shoulders, crotch and hips, guided by the defects exposed under pose.

Manual validation: compare the fitted cage with the anatomy target under broad/narrow shoulder width, chest width, hip width and thigh girth. Then use the Rigging view to study posed deformation. Keep GitHub CI manual-only.

## Neutral shoulder topology surgery (first connectivity pass)

`studio-core/src/shoulder_patch.rs` runs after neutral surface-net extraction
and before the neutral vertex-flow optimizer. For each shoulder, anatomical
shoulder/elbow landmarks define a local region and arm direction; an axillary
hotspot emphasizes moving extraordinary-valence poles away from the likely
high-strain junction.

The local surgery examines pairs of adjacent quads sharing an edge. Their
combined boundary is an oriented hexagon with **two alternative quad-pair
splits**. An accepted exchange substitutes their shared edge, changes the
valence at its four endpoints, and leaves every outer boundary edge exactly
unchanged. It therefore changes **real connectivity** without adding
vertices, touching another region's boundary, or modifying the total
number of quads.

The operation rejects candidates that introduce a duplicate edge, repeated
vertices, too-short or overlong edges, stretched/aspect-distorted quads,
degenerate/inward-facing triangles or faces too far from the underlying
anatomical field. After each accepted exchange the corresponding render
triangles are rewritten to match the new quad indices. The complete
neutral surface is checked for closed, consistently oriented topology
before caching.

This is performed once per neutral humanoid archetype; fitting and rigging
use the resulting fixed connectivity for every player build. The UI
shows accepted left/right rewires and regional weighted pole cost.
A nonzero number of rewires indicates genuine local connectivity changes;
zero means the current neutral geometry had no safe improvement under
these constraints, not that a patch was silently invented.

**Limitation:** local two-quad reconfiguration does **not** automatically
produce regular circular shoulder loops or strategically relocate all
poles away from deformation hotspots. A future stronger
anatomy-guided patch will need explicit, dependable torso-side and
arm-side boundary loops, compatible loop counts and a watertight
structured quad grid between them.

Manual visual review: inspect shoulders in Fitted cage with Wireframe and
Poles enabled, then raise one arm 30° and 60° in Rigging with Defects
enabled. Watch for a lower concentration of exceptional poles in the
armpit, and distinguish connectivity defects from provisional
skin-weight distortion. GitHub CI remains manual-only.
