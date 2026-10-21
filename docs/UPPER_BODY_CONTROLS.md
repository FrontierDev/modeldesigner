# Upper-body morphology controls

This is the first complete **upper-body** parameter catalogue. Sliders operate on one procedural torso/neck/shoulder/upper-arm mesh with stable indexed topology, not on a pre-imported character. The head, forearms and lower body are deliberately excluded.

**Player** exposes the primary shape controls. **Advanced** also exposes anatomical refinement parameters. Every visible control is implemented in `study.rs`; hidden future-anatomy controls are invalid unless they retain their neutral values. All stored values are dimensionless ratios or normalized weights except height and shoulder width, which are metres internally.

All groups provide Reset group; Reset all returns every implemented parameter to its neutral default. Recipe and exported UI-schema JSON preserve these same identifiers and bounds.

## Global proportions

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Height (`height`) | 1.5–2.1 cm | 1.75 | Yes | Overall reference stature; scales torso, shoulders, arm length and joints. |
| Shoulder width (`shoulder_width`) | 0.34–0.62 cm | 0.46 | Yes | Lateral shoulder-girdle span; moves both shoulder landmarks and chest contour. |
| Arm length (`arm_length`) | 0.82–1.18 % | 1.0 | Yes | Upper-arm length from humeral head to elbow (forearm is not modelled). |

## Body composition

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Musculature (`musculature`) | 0.0–1.0 % | 0.5 | Yes | Overall upper-body muscle definition, strongest around pectorals and deltoids. |
| Body mass (`body_mass`) | 0.0–1.0 % | 0.5 | Yes | Soft-tissue fullness through the waist, abdomen and upper arms. |

## Neck termination

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Neck length (`neck_length`) | 0.85–1.15 % | 1.0 | Advanced | Length of the temporary neck termination and neck joint. |
| Neck width (`neck_width`) | 0.75–1.3 % | 1.0 | Advanced | Width and depth of the neck termination. |

## Torso and back

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Torso length (`torso_length`) | 0.85–1.15 % | 1.0 | Yes | Torso length between lower trunk and neck. |
| Ribcage taper (`ribcage_taper`) | 0.8–1.2 % | 1.0 | Advanced | Relative widening or narrowing of the lower ribcage. |
| Waist width (`waist_width`) | 0.75–1.3 % | 1.0 | Yes | Width of the lower torso and waist. |
| Abdomen fullness (`abdomen_fullness`) | 0.0–1.0 % | 0.5 | Yes | Forward projection of the abdominal surface. |
| Back curvature (`back_curvature`) | 0.0–1.0 % | 0.5 | Advanced | Front-to-back bending of the spine and torso profile. |
| Ribcage height (`ribcage_height`) | 0.85–1.15 % | 1.0 | Yes | Relative vertical span of the thoracic region. |
| Waist depth (`waist_depth`) | 0.75–1.3 % | 1.0 | Yes | Front-to-back thickness around the waist. |
| Latissimus spread (`lat_spread`) | 0.75–1.3 % | 1.0 | Advanced | Width of the posterior-lateral back under the arms. |
| Upper back fullness (`upper_back_fullness`) | 0.0–1.0 % | 0.5 | Advanced | Rearward surface volume of the upper back. |

## Chest and pectorals

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Chest width (`chest_width`) | 0.8–1.25 % | 1.0 | Yes | Side-to-side width of the ribcage and upper chest. |
| Chest depth (`chest_depth`) | 0.75–1.3 % | 1.0 | Yes | Front-to-back depth of the ribcage. |
| Pectoral fullness (`pectoral_fullness`) | 0.0–1.0 % | 0.5 | Yes | Anterior pectoral projection, distinct from overall chest depth. |
| Sternum definition (`sternum_definition`) | 0.0–1.0 % | 0.5 | Advanced | Depth of the narrow groove at the chest centreline. |

## Shoulder girdle and deltoids

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Shoulder slope (`shoulder_slope`) | 0.0–1.0 % | 0.5 | Yes | Inclination of upper torso and shoulder line. |
| Shoulder forward position (`shoulder_forward`) | 0.0–1.0 % | 0.5 | Advanced | Moves the shoulder girdle and humeral heads forward or backward. |
| Deltoid volume (`deltoid_volume`) | 0.0–1.0 % | 0.5 | Yes | Overall thickness of the deltoid cap near the shoulder. |
| Deltoid roundness (`deltoid_roundness`) | 0.0–1.0 % | 0.5 | Advanced | Fullness of the upper deltoid contour rather than arm girth. |

## Upper arms

| Control (ID) | Range | Neutral | Player | Effect |
| --- | --- | --- | --- | --- |
| Upper arm girth (`upper_arm_girth`) | 0.7–1.4 % | 1.0 | Yes | Overall thickness of each upper arm. |
| Biceps volume (`biceps_volume`) | 0.0–1.0 % | 0.5 | Yes | Anterior muscle bulge along the upper-arm surface. |
| Triceps volume (`triceps_volume`) | 0.0–1.0 % | 0.5 | Yes | Posterior muscle bulge along the upper-arm surface. |
| Elbow width (`elbow_width`) | 0.75–1.25 % | 1.0 | Yes | Width of the temporary elbow-end loop. |
| Upper-arm taper (`arm_taper`) | 0.8–1.2 % | 1.0 | Advanced | How rapidly the arm narrows toward the elbow. |

## Behavioural constraints

- Height and torso length reshape mesh and rest skeletal landmarks coherently; arm length measures only shoulder-to-elbow in this partial-body prototype.
- Pectoral fullness and sternum definition adjust the anterior chest surface, not independent floating pectoral meshes. Upper-back fullness and latissimus spread affect posterior contours.
- Deltoid volume/roundness and biceps/triceps bulges are longitudinal and angular contour controls on the same five arm rings. Elbow width and arm taper are separate from total arm girth.
- Shoulder forward position updates socket geometry and clavicle/humeral landmark positions; shoulder slope updates upper torso and bone placement.
- Body mass and musculature are simplified geometric morphology proxies, not a simulated fat/muscle anatomy system.
- All 29 controls must alter vertex positions, and bounds should produce non-degenerate geometry without changing quads, triangle connectivity, joint count or influence-array length.
- The neutral model remains the baseline for rig export; Raise/Forward controls are **temporary posing**, not morphology sliders or saved recipe parameters.

## Verification

Run `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`. The test suite checks supported controls, extreme slider values, topology invariance, recipe round-tripping, schema descriptions, regional bulges and joint updates. Visual review must still check combinations (e.g. high muscularity with narrow shoulders).
