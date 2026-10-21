//! Shared slider contract for authored models and player-facing build customization.

use crate::Recipe;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum BuildParameter {
    Height,
    ShoulderWidth,
    ArmLength,
    LegLength,
    Musculature,
    BodyMass,
    TorsoLength,
    HeadSize,
    NeckLength,
    NeckWidth,
    ShoulderSlope,
    ChestWidth,
    ChestDepth,
    RibcageTaper,
    WaistWidth,
    AbdomenFullness,
    BackCurvature,
    HipWidth,
    PelvisTilt,
    GluteVolume,
    UpperArmGirth,
    ForearmGirth,
    WristWidth,
    HandSize,
    ThighGirth,
    CalfGirth,
    AnkleWidth,
    FootLength,
    FootWidth,
    StanceWidth,
    RibcageHeight,
    WaistDepth,
    LatSpread,
    UpperBackFullness,
    PectoralFullness,
    SternumDefinition,
    ShoulderForward,
    DeltoidVolume,
    DeltoidRoundness,
    BicepsVolume,
    TricepsVolume,
    ElbowWidth,
    ArmTaper,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildSection {
    Proportions,
    Physique,
    HeadNeck,
    Torso,
    Pelvis,
    Chest,
    Shoulders,
    Neck,
    Arms,
    Legs,
}
#[derive(Clone, Copy, Debug)]
pub struct BuildControl {
    pub parameter: BuildParameter,
    pub section: BuildSection,
    /// Stable recipe field ID; new morphology controls reside in "morphology".
    pub id: &'static str,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub display_scale: f32,
    pub suffix: &'static str,
    /// True when this control is suitable for the initial in-game build UI.
    pub player_editable: bool,
}
impl BuildControl {
    /// Only controls that affect the current upper-body study are exposed.
    /// Additional humanoid controls will be enabled as corresponding patches arrive.
    pub fn implemented(self) -> bool {
        matches!(self.parameter,
            BuildParameter::Height
            | BuildParameter::ShoulderWidth
            | BuildParameter::ArmLength
            | BuildParameter::Musculature
            | BuildParameter::BodyMass
            | BuildParameter::TorsoLength
            | BuildParameter::NeckLength
            | BuildParameter::NeckWidth
            | BuildParameter::ShoulderSlope
            | BuildParameter::ChestWidth
            | BuildParameter::ChestDepth
            | BuildParameter::RibcageTaper
            | BuildParameter::WaistWidth
            | BuildParameter::AbdomenFullness
            | BuildParameter::BackCurvature
            | BuildParameter::UpperArmGirth
            | BuildParameter::RibcageHeight
            | BuildParameter::WaistDepth
            | BuildParameter::LatSpread
            | BuildParameter::UpperBackFullness
            | BuildParameter::PectoralFullness
            | BuildParameter::SternumDefinition
            | BuildParameter::ShoulderForward
            | BuildParameter::DeltoidVolume
            | BuildParameter::DeltoidRoundness
            | BuildParameter::BicepsVolume
            | BuildParameter::TricepsVolume
            | BuildParameter::ElbowWidth
            | BuildParameter::ArmTaper
            | BuildParameter::LegLength
            | BuildParameter::HipWidth
            | BuildParameter::PelvisTilt
            | BuildParameter::GluteVolume
            | BuildParameter::ThighGirth
            | BuildParameter::StanceWidth
        )
    }

    /// Anatomical intent and precise effect; exported with the game UI schema.
    pub fn description(self) -> &'static str {
        match self.parameter {
            BuildParameter::Height => "Overall reference stature; scales torso, shoulders, arm length and joints.",
            BuildParameter::ShoulderWidth => "Lateral shoulder-girdle span; moves both shoulder landmarks and chest contour.",
            BuildParameter::ArmLength => "Upper-arm length from humeral head to elbow (forearm is not modelled).",
            BuildParameter::Musculature => "Overall upper-body muscle definition, strongest around pectorals and deltoids.",
            BuildParameter::BodyMass => "Soft-tissue fullness through the waist, abdomen and upper arms.",
            BuildParameter::TorsoLength => "Torso length between lower trunk and neck.",
            BuildParameter::NeckLength => "Length of the temporary neck termination and neck joint.",
            BuildParameter::NeckWidth => "Width and depth of the neck termination.",
            BuildParameter::ShoulderSlope => "Inclination of upper torso and shoulder line.",
            BuildParameter::ChestWidth => "Side-to-side width of the ribcage and upper chest.",
            BuildParameter::ChestDepth => "Front-to-back depth of the ribcage.",
            BuildParameter::RibcageTaper => "Relative widening or narrowing of the lower ribcage.",
            BuildParameter::WaistWidth => "Width of the lower torso and waist.",
            BuildParameter::AbdomenFullness => "Forward projection of the abdominal surface.",
            BuildParameter::BackCurvature => "Front-to-back bending of the spine and torso profile.",
            BuildParameter::UpperArmGirth => "Overall thickness of each upper arm.",
            BuildParameter::RibcageHeight => "Relative vertical span of the thoracic region.",
            BuildParameter::WaistDepth => "Front-to-back thickness around the waist.",
            BuildParameter::LatSpread => "Width of the posterior-lateral back under the arms.",
            BuildParameter::UpperBackFullness => "Rearward surface volume of the upper back.",
            BuildParameter::PectoralFullness => "Anterior pectoral projection, distinct from overall chest depth.",
            BuildParameter::SternumDefinition => "Depth of the narrow groove at the chest centreline.",
            BuildParameter::ShoulderForward => "Moves the shoulder girdle and humeral heads forward or backward.",
            BuildParameter::DeltoidVolume => "Overall thickness of the deltoid cap near the shoulder.",
            BuildParameter::DeltoidRoundness => "Fullness of the upper deltoid contour rather than arm girth.",
            BuildParameter::BicepsVolume => "Anterior muscle bulge along the upper-arm surface.",
            BuildParameter::TricepsVolume => "Posterior muscle bulge along the upper-arm surface.",
            BuildParameter::ElbowWidth => "Width of the temporary elbow-end loop.",
            BuildParameter::ArmTaper => "How rapidly the arm narrows toward the elbow.",
            BuildParameter::LegLength => "Upper-leg length from the hip to the provisional knee boundary.",
            BuildParameter::HipWidth => "Lateral width of the pelvic girdle and hip socket spacing.",
            BuildParameter::PelvisTilt => "Forward/backward pelvic inclination around the hip girdle.",
            BuildParameter::GluteVolume => "Posterior volume around the buttocks and lower pelvis.",
            BuildParameter::ThighGirth => "Overall upper-leg circumference.",
            BuildParameter::StanceWidth => "Lateral separation of the knees relative to the hip joints.",
            _ => "Not available until the corresponding anatomy is implemented.",
        }
    }

    pub fn get(self, recipe: &Recipe) -> f32 {
        match self.parameter {
            BuildParameter::Height => recipe.height,
            BuildParameter::ShoulderWidth => recipe.shoulder_width,
            BuildParameter::ArmLength => recipe.arm_length,
            BuildParameter::LegLength => recipe.leg_length,
            BuildParameter::Musculature => recipe.musculature,
            BuildParameter::BodyMass => recipe.body_mass,
            BuildParameter::TorsoLength => recipe.morphology.torso_length,
            BuildParameter::HeadSize => recipe.morphology.head_size,
            BuildParameter::NeckLength => recipe.morphology.neck_length,
            BuildParameter::NeckWidth => recipe.morphology.neck_width,
            BuildParameter::ShoulderSlope => recipe.morphology.shoulder_slope,
            BuildParameter::ChestWidth => recipe.morphology.chest_width,
            BuildParameter::ChestDepth => recipe.morphology.chest_depth,
            BuildParameter::RibcageTaper => recipe.morphology.ribcage_taper,
            BuildParameter::WaistWidth => recipe.morphology.waist_width,
            BuildParameter::AbdomenFullness => recipe.morphology.abdomen_fullness,
            BuildParameter::BackCurvature => recipe.morphology.back_curvature,
            BuildParameter::HipWidth => recipe.morphology.hip_width,
            BuildParameter::PelvisTilt => recipe.morphology.pelvis_tilt,
            BuildParameter::GluteVolume => recipe.morphology.glute_volume,
            BuildParameter::UpperArmGirth => recipe.morphology.upper_arm_girth,
            BuildParameter::ForearmGirth => recipe.morphology.forearm_girth,
            BuildParameter::WristWidth => recipe.morphology.wrist_width,
            BuildParameter::HandSize => recipe.morphology.hand_size,
            BuildParameter::ThighGirth => recipe.morphology.thigh_girth,
            BuildParameter::CalfGirth => recipe.morphology.calf_girth,
            BuildParameter::AnkleWidth => recipe.morphology.ankle_width,
            BuildParameter::FootLength => recipe.morphology.foot_length,
            BuildParameter::FootWidth => recipe.morphology.foot_width,
            BuildParameter::StanceWidth => recipe.morphology.stance_width,
            BuildParameter::RibcageHeight => recipe.morphology.ribcage_height,
            BuildParameter::WaistDepth => recipe.morphology.waist_depth,
            BuildParameter::LatSpread => recipe.morphology.lat_spread,
            BuildParameter::UpperBackFullness => recipe.morphology.upper_back_fullness,
            BuildParameter::PectoralFullness => recipe.morphology.pectoral_fullness,
            BuildParameter::SternumDefinition => recipe.morphology.sternum_definition,
            BuildParameter::ShoulderForward => recipe.morphology.shoulder_forward,
            BuildParameter::DeltoidVolume => recipe.morphology.deltoid_volume,
            BuildParameter::DeltoidRoundness => recipe.morphology.deltoid_roundness,
            BuildParameter::BicepsVolume => recipe.morphology.biceps_volume,
            BuildParameter::TricepsVolume => recipe.morphology.triceps_volume,
            BuildParameter::ElbowWidth => recipe.morphology.elbow_width,
            BuildParameter::ArmTaper => recipe.morphology.arm_taper,
        }
    }
    pub fn set(self, recipe: &mut Recipe, value: f32) {
        match self.parameter {
            BuildParameter::Height => recipe.height = value,
            BuildParameter::ShoulderWidth => recipe.shoulder_width = value,
            BuildParameter::ArmLength => recipe.arm_length = value,
            BuildParameter::LegLength => recipe.leg_length = value,
            BuildParameter::Musculature => recipe.musculature = value,
            BuildParameter::BodyMass => recipe.body_mass = value,
            BuildParameter::TorsoLength => recipe.morphology.torso_length = value,
            BuildParameter::HeadSize => recipe.morphology.head_size = value,
            BuildParameter::NeckLength => recipe.morphology.neck_length = value,
            BuildParameter::NeckWidth => recipe.morphology.neck_width = value,
            BuildParameter::ShoulderSlope => recipe.morphology.shoulder_slope = value,
            BuildParameter::ChestWidth => recipe.morphology.chest_width = value,
            BuildParameter::ChestDepth => recipe.morphology.chest_depth = value,
            BuildParameter::RibcageTaper => recipe.morphology.ribcage_taper = value,
            BuildParameter::WaistWidth => recipe.morphology.waist_width = value,
            BuildParameter::AbdomenFullness => recipe.morphology.abdomen_fullness = value,
            BuildParameter::BackCurvature => recipe.morphology.back_curvature = value,
            BuildParameter::HipWidth => recipe.morphology.hip_width = value,
            BuildParameter::PelvisTilt => recipe.morphology.pelvis_tilt = value,
            BuildParameter::GluteVolume => recipe.morphology.glute_volume = value,
            BuildParameter::UpperArmGirth => recipe.morphology.upper_arm_girth = value,
            BuildParameter::ForearmGirth => recipe.morphology.forearm_girth = value,
            BuildParameter::WristWidth => recipe.morphology.wrist_width = value,
            BuildParameter::HandSize => recipe.morphology.hand_size = value,
            BuildParameter::ThighGirth => recipe.morphology.thigh_girth = value,
            BuildParameter::CalfGirth => recipe.morphology.calf_girth = value,
            BuildParameter::AnkleWidth => recipe.morphology.ankle_width = value,
            BuildParameter::FootLength => recipe.morphology.foot_length = value,
            BuildParameter::FootWidth => recipe.morphology.foot_width = value,
            BuildParameter::StanceWidth => recipe.morphology.stance_width = value,
            BuildParameter::RibcageHeight => recipe.morphology.ribcage_height = value,
            BuildParameter::WaistDepth => recipe.morphology.waist_depth = value,
            BuildParameter::LatSpread => recipe.morphology.lat_spread = value,
            BuildParameter::UpperBackFullness => recipe.morphology.upper_back_fullness = value,
            BuildParameter::PectoralFullness => recipe.morphology.pectoral_fullness = value,
            BuildParameter::SternumDefinition => recipe.morphology.sternum_definition = value,
            BuildParameter::ShoulderForward => recipe.morphology.shoulder_forward = value,
            BuildParameter::DeltoidVolume => recipe.morphology.deltoid_volume = value,
            BuildParameter::DeltoidRoundness => recipe.morphology.deltoid_roundness = value,
            BuildParameter::BicepsVolume => recipe.morphology.biceps_volume = value,
            BuildParameter::TricepsVolume => recipe.morphology.triceps_volume = value,
            BuildParameter::ElbowWidth => recipe.morphology.elbow_width = value,
            BuildParameter::ArmTaper => recipe.morphology.arm_taper = value,
        }
    }
}
impl BuildSection {
    pub fn id(self) -> &'static str {
        match self {
            Self::Proportions => "proportions",
            Self::Physique => "physique",
            Self::Torso => "torso",
            Self::Pelvis => "pelvis",
            Self::Chest => "chest",
            Self::Shoulders => "shoulders",
            Self::Neck => "neck",
            Self::Arms => "arms",
            Self::Legs => "legs",
            Self::HeadNeck => "head_neck",
        }
    }
}

/// Portable JSON contract for a future Unity/game character build UI.
/// Parameter IDs and stored numeric values match the versioned humanoid recipe.
pub fn build_schema_json() -> Result<String, String> {
    let controls: Vec<_> = BUILD_CONTROLS.iter().copied().filter(|control| control.implemented()).map(|control| {
        serde_json::json!({
            "id": control.id,
            "section": control.section.id(),
            "label": control.label,
            "description": control.description(),
            "min": control.min,
            "max": control.max,
            "default": control.default,
            "display_scale": control.display_scale,
            "suffix": control.suffix,
            "player_editable": control.player_editable,
            "implemented": control.implemented(),
        })
    }).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": crate::SCHEMA_VERSION,
        "model_type": "upper_body_study",
        "controls": controls
    })).map_err(|error| error.to_string())
}

pub const BUILD_CONTROLS: [BuildControl; 43] = [
    BuildControl {
        parameter: BuildParameter::Height, section: BuildSection::Proportions,
        id: "height", label: "Height",
        min: 1.5, max: 2.1, default: 1.75,
        display_scale: 100.0, suffix: " cm", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ShoulderWidth, section: BuildSection::Proportions,
        id: "shoulder_width", label: "Shoulder width",
        min: 0.34, max: 0.62, default: 0.46,
        display_scale: 100.0, suffix: " cm", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ArmLength, section: BuildSection::Proportions,
        id: "arm_length", label: "Arm length",
        min: 0.82, max: 1.18, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::LegLength, section: BuildSection::Legs,
        id: "leg_length", label: "Leg length",
        min: 0.82, max: 1.18, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::Musculature, section: BuildSection::Physique,
        id: "musculature", label: "Musculature",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::BodyMass, section: BuildSection::Physique,
        id: "body_mass", label: "Body mass",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::TorsoLength, section: BuildSection::Torso,
        id: "torso_length", label: "Torso length",
        min: 0.85, max: 1.15, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::HeadSize, section: BuildSection::HeadNeck,
        id: "head_size", label: "Head size",
        min: 0.8, max: 1.2, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::NeckLength, section: BuildSection::Neck,
        id: "neck_length", label: "Neck length",
        min: 0.85, max: 1.15, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::NeckWidth, section: BuildSection::Neck,
        id: "neck_width", label: "Neck width",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::ShoulderSlope, section: BuildSection::Shoulders,
        id: "shoulder_slope", label: "Shoulder slope",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ChestWidth, section: BuildSection::Chest,
        id: "chest_width", label: "Chest width",
        min: 0.8, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ChestDepth, section: BuildSection::Chest,
        id: "chest_depth", label: "Chest depth",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::RibcageTaper, section: BuildSection::Torso,
        id: "ribcage_taper", label: "Ribcage taper",
        min: 0.8, max: 1.2, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::WaistWidth, section: BuildSection::Torso,
        id: "waist_width", label: "Waist width",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::AbdomenFullness, section: BuildSection::Torso,
        id: "abdomen_fullness", label: "Abdomen fullness",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::BackCurvature, section: BuildSection::Torso,
        id: "back_curvature", label: "Back curvature",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::HipWidth, section: BuildSection::Pelvis,
        id: "hip_width", label: "Hip width",
        min: 0.8, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::PelvisTilt, section: BuildSection::Pelvis,
        id: "pelvis_tilt", label: "Pelvis tilt",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::GluteVolume, section: BuildSection::Pelvis,
        id: "glute_volume", label: "Glute volume",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::UpperArmGirth, section: BuildSection::Arms,
        id: "upper_arm_girth", label: "Upper arm girth",
        min: 0.7, max: 1.4, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ForearmGirth, section: BuildSection::Arms,
        id: "forearm_girth", label: "Forearm girth",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::WristWidth, section: BuildSection::Arms,
        id: "wrist_width", label: "Wrist width",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::HandSize, section: BuildSection::Arms,
        id: "hand_size", label: "Hand size",
        min: 0.8, max: 1.2, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ThighGirth, section: BuildSection::Legs,
        id: "thigh_girth", label: "Thigh girth",
        min: 0.7, max: 1.4, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::CalfGirth, section: BuildSection::Legs,
        id: "calf_girth", label: "Calf girth",
        min: 0.7, max: 1.4, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::AnkleWidth, section: BuildSection::Legs,
        id: "ankle_width", label: "Ankle width",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::FootLength, section: BuildSection::Legs,
        id: "foot_length", label: "Foot length",
        min: 0.8, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::FootWidth, section: BuildSection::Legs,
        id: "foot_width", label: "Foot width",
        min: 0.8, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::StanceWidth, section: BuildSection::Legs,
        id: "stance_width", label: "Stance width",
        min: 0.8, max: 1.2, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::RibcageHeight, section: BuildSection::Torso,
        id: "ribcage_height", label: "Ribcage height",
        min: 0.85, max: 1.15, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::WaistDepth, section: BuildSection::Torso,
        id: "waist_depth", label: "Waist depth",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::LatSpread, section: BuildSection::Torso,
        id: "lat_spread", label: "Latissimus spread",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::UpperBackFullness, section: BuildSection::Torso,
        id: "upper_back_fullness", label: "Upper back fullness",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::PectoralFullness, section: BuildSection::Chest,
        id: "pectoral_fullness", label: "Pectoral fullness",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::SternumDefinition, section: BuildSection::Chest,
        id: "sternum_definition", label: "Sternum definition",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::ShoulderForward, section: BuildSection::Shoulders,
        id: "shoulder_forward", label: "Shoulder forward position",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::DeltoidVolume, section: BuildSection::Shoulders,
        id: "deltoid_volume", label: "Deltoid volume",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::DeltoidRoundness, section: BuildSection::Shoulders,
        id: "deltoid_roundness", label: "Deltoid roundness",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::BicepsVolume, section: BuildSection::Arms,
        id: "biceps_volume", label: "Biceps volume",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::TricepsVolume, section: BuildSection::Arms,
        id: "triceps_volume", label: "Triceps volume",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ElbowWidth, section: BuildSection::Arms,
        id: "elbow_width", label: "Elbow width",
        min: 0.75, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ArmTaper, section: BuildSection::Arms,
        id: "arm_taper", label: "Upper-arm taper",
        min: 0.8, max: 1.2, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
];
