# Fitted quad cage

The neutral humanoid anatomy is sampled to create a quad surface-net template. Its indices are cached, and the quad-flow stage moves neutral vertices towards upper-limb directions without rewriting connectivity. The previously flawed 656-quad study was deleted.

At the current partial-humanoid proportions the new template has approximately 4,000 quads. Surface nets do not automatically provide correct skeletal edge loops.

For player morphology, cage.rs first transports upper-arm/thigh regions by their shoulder-elbow and hip-knee landmarks. Vertices then move toward the target signed field with local face-inversion, triangle-collapse and edge-distortion checks. One problematic quad should not veto movement elsewhere.

The **Fitted cage** view displays surface errors, fit-strength, wireframe and pole diagnostics. No skin weights are authored by the cage fitter. The **Rigging** view uses exactly these fitted vertex positions and the same quad connectivity.

This is not production retopology or an exportable rig. Future passes must reorganize neutral connectivity around armpits, shoulders, crotch and hips, guided by the defects exposed under pose.

Manual validation: compare the fitted cage with the anatomy target under broad/narrow shoulder width, chest width, hip width and thigh girth. Then use the Rigging view to study posed deformation. Keep GitHub CI manual-only.