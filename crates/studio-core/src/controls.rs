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
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildSection {
    Proportions,
    Physique,
    HeadNeck,
    Torso,
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
        }
    }
}
impl BuildSection {
    pub fn id(self) -> &'static str {
        match self {
            Self::Proportions => "proportions",
            Self::Physique => "physique",
            Self::Torso => "torso",
            Self::Arms => "arms",
            Self::Legs => "legs",
            Self::HeadNeck => "head_neck",
        }
    }
}

/// Portable JSON contract for a future Unity/game character build UI.
/// Parameter IDs and stored numeric values match the versioned humanoid recipe.
pub fn build_schema_json() -> Result<String, String> {
    let controls: Vec<_> = BUILD_CONTROLS.iter().map(|control| {
        serde_json::json!({
            "id": control.id,
            "section": control.section.id(),
            "label": control.label,
            "min": control.min,
            "max": control.max,
            "default": control.default,
            "display_scale": control.display_scale,
            "suffix": control.suffix,
            "player_editable": control.player_editable,
        })
    }).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": crate::SCHEMA_VERSION,
        "generator_version": crate::GENERATOR_VERSION,
        "model_type": "humanoid",
        "controls": controls
    })).map_err(|error| error.to_string())
}

pub const BUILD_CONTROLS: [BuildControl; 30] = [
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
        parameter: BuildParameter::LegLength, section: BuildSection::Proportions,
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
        parameter: BuildParameter::NeckLength, section: BuildSection::HeadNeck,
        id: "neck_length", label: "Neck length",
        min: 0.85, max: 1.15, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::NeckWidth, section: BuildSection::HeadNeck,
        id: "neck_width", label: "Neck width",
        min: 0.75, max: 1.3, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::ShoulderSlope, section: BuildSection::Torso,
        id: "shoulder_slope", label: "Shoulder slope",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ChestWidth, section: BuildSection::Torso,
        id: "chest_width", label: "Chest width",
        min: 0.8, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::ChestDepth, section: BuildSection::Torso,
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
        parameter: BuildParameter::HipWidth, section: BuildSection::Legs,
        id: "hip_width", label: "Hip width",
        min: 0.8, max: 1.25, default: 1.0,
        display_scale: 100.0, suffix: "%", player_editable: true
    },
    BuildControl {
        parameter: BuildParameter::PelvisTilt, section: BuildSection::Legs,
        id: "pelvis_tilt", label: "Pelvis tilt",
        min: 0.0, max: 1.0, default: 0.5,
        display_scale: 100.0, suffix: "%", player_editable: false
    },
    BuildControl {
        parameter: BuildParameter::GluteVolume, section: BuildSection::Legs,
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
];
