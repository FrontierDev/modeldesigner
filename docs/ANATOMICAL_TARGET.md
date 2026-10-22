# Anatomical target

Purpose: define the desired anatomy independently of final quad connectivity.

The anatomy field combines pelvis, abdominal, gluteal, quadriceps, hamstring and adductor volumes with a tapered ten-section torso, back, chest and neck profile. Tapered upper arms connect at the deltoids. The field is smoothed and sampled into temporary triangles for the Anatomy target preview.

Named landmarks include neck, sternum, waist, sacrum, pubic symphysis, hip centres, greater trochanters, shoulders, elbows and knees. These anchors are shared by morphology fitting and provisional rigging.

The model currently stops at the neck, elbows and knees. Forearms, lower legs, head, hands and feet have not yet been implemented.

The neutral target is quadrangulated once into the archetype's cached quad mesh. Build-slider changes adjust anatomy and fitted vertex positions while retaining that quad connectivity. The Rigging view uses the fitted cage, never temporary anatomy triangles.

No imported base mesh is required. Surface detail is an anatomical approximation, not biomedical muscle simulation or validated animation data.

Related: docs/CAGE_FITTING.md and docs/RIGGING_PREVIEW.md.