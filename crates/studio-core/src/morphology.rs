//! Parameter data reserved for current and future anatomical patches.
//! Unsupported controls are explicitly rejected until implemented.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Morphology {
    pub torso_length: f32,
    pub head_size: f32,
    pub neck_length: f32,
    pub neck_width: f32,
    pub shoulder_slope: f32,
    pub chest_width: f32,
    pub chest_depth: f32,
    pub ribcage_taper: f32,
    pub waist_width: f32,
    pub abdomen_fullness: f32,
    pub back_curvature: f32,
    pub hip_width: f32,
    pub pelvis_tilt: f32,
    pub glute_volume: f32,
    pub upper_arm_girth: f32,
    pub forearm_girth: f32,
    pub wrist_width: f32,
    pub hand_size: f32,
    pub thigh_girth: f32,
    pub calf_girth: f32,
    pub ankle_width: f32,
    pub foot_length: f32,
    pub foot_width: f32,
    pub stance_width: f32,
}
impl Default for Morphology {
    fn default() -> Self {
        Self {
            torso_length: 1.0,
            head_size: 1.0,
            neck_length: 1.0,
            neck_width: 1.0,
            shoulder_slope: 0.5,
            chest_width: 1.0,
            chest_depth: 1.0,
            ribcage_taper: 1.0,
            waist_width: 1.0,
            abdomen_fullness: 0.5,
            back_curvature: 0.5,
            hip_width: 1.0,
            pelvis_tilt: 0.5,
            glute_volume: 0.5,
            upper_arm_girth: 1.0,
            forearm_girth: 1.0,
            wrist_width: 1.0,
            hand_size: 1.0,
            thigh_girth: 1.0,
            calf_girth: 1.0,
            ankle_width: 1.0,
            foot_length: 1.0,
            foot_width: 1.0,
            stance_width: 1.0,
        }
    }
}

