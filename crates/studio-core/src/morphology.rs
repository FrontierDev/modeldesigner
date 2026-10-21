//! Additional humanoid morphology controls. Every neutral default is a no-op.
//! The first six recipe fields remain unchanged for backwards compatibility.

use glam::Vec3;
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

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn band(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    smoothstep(a, b, x) * (1.0 - smoothstep(c, d, x))
}

/// Landmark-space corrections shared by the mesh and the rest-pose skeleton.
/// The reference coordinate must always be from the unchanged canonical body.
pub fn structural_offset(reference: Vec3, m: &Morphology, height_scale: f32) -> Vec3 {
    let p = reference;
    let side = p.x.signum();
    let torso = band(p.y, 0.38, 0.84, 1.30, 1.55);
    let shoulder = smoothstep(0.13, 0.27, p.x.abs()) * band(p.y, 1.14, 1.34, 1.49, 1.62);
    let neck = smoothstep(1.44, 1.64, p.y);
    let pelvis = band(p.y, 0.69, 0.84, 1.02, 1.30);
    let leg = (1.0 - smoothstep(0.76, 1.01, p.y)) * smoothstep(0.045, 0.115, p.x.abs());
    Vec3::new(
        side * (0.090 * (m.hip_width - 1.0) * (1.0 - smoothstep(1.00, 1.23, p.y))
              + 0.075 * (m.stance_width - 1.0) * leg),
        -0.14 * (m.torso_length - 1.0) * torso
            - 0.12 * (m.shoulder_slope - 0.5) * shoulder
            + 0.12 * (m.neck_length - 1.0) * neck,
        0.08 * (m.pelvis_tilt - 0.5) * pelvis
    ) * height_scale
}

/// Deform the fixed reference cage without remeshing. Parameters are region-local,
/// mirror-symmetric, and deliberately constrained; this is not final anatomical retopology.
pub fn deform_surface(reference: Vec3, base: Vec3, m: &Morphology, height_scale: f32) -> Vec3 {
    let p = reference;
    let mut out = base + structural_offset(p, m, height_scale);
    let abs_x = p.x.abs();
    let side = p.x.signum();
    let torso = (1.0 - smoothstep(0.22, 0.32, abs_x));
    let upper_chest = band(p.y, 1.16, 1.28, 1.39, 1.51) * torso;
    let lower_ribs = band(p.y, 0.99, 1.11, 1.22, 1.34) * torso;
    let waist = band(p.y, 0.93, 1.03, 1.14, 1.27) * torso;
    let abdomen = band(p.y, 0.90, 1.01, 1.17, 1.28) * torso;
    let upper_back = band(p.y, 1.00, 1.16, 1.35, 1.52) * torso;
    let hip = band(p.y, 0.76, 0.85, 0.95, 1.06) * (1.0 - smoothstep(0.24, 0.35, abs_x));
    let head = smoothstep(1.51, 1.63, p.y);
    let neck = band(p.y, 1.42, 1.48, 1.54, 1.62) * (1.0 - smoothstep(.11,.18,abs_x));

    // Width and depth are independent so broad builds are not necessarily thick.
    out.x += p.x * (0.55 * (m.chest_width - 1.0) * upper_chest
        + 0.42 * (m.ribcage_taper - 1.0) * lower_ribs
        + 0.68 * (m.waist_width - 1.0) * waist
        + 0.55 * (m.neck_width - 1.0) * neck) * height_scale;
    out.z += p.z * (0.88 * (m.chest_depth - 1.0) * upper_chest
        + 0.40 * (m.ribcage_taper - 1.0) * lower_ribs
        + 0.50 * (m.neck_width - 1.0) * neck) * height_scale;
    out.z += 0.125 * (m.abdomen_fullness - 0.5) * abdomen
        * smoothstep(-0.04,0.055,p.z) * height_scale;
    out.z -= 0.095 * (m.back_curvature - 0.5) * upper_back
        * (1.0 - smoothstep(-0.07, 0.06, p.z)) * height_scale;
    out.z -= 0.14 * (m.glute_volume - 0.5) * hip
        * (1.0 - smoothstep(-0.09, 0.02, p.z)) * height_scale;

    // Head size is relative to its central landmark, not a global scaling.
    out += (p - Vec3::new(0.0, 1.635, 0.0)) * (m.head_size - 1.0) * head * height_scale;

    // Arm girth changes transverse volume rather than shifting the shoulder joint.
    let arm = smoothstep(0.26, 0.37, abs_x);
    let upper_arm = band(p.y, 1.16, 1.24, 1.38, 1.46) * arm;
    let forearm = band(p.y, 1.04, 1.09, 1.19, 1.26) * arm;
    let wrist = band(p.y, 1.00, 1.04, 1.09, 1.14) * smoothstep(.48,.55,abs_x);
    let hand = smoothstep(0.55, 0.63, abs_x) * (1.0-smoothstep(1.01, 1.09, p.y));
    let arm_girth = 0.8*(m.upper_arm_girth-1.0)*upper_arm
        +0.8*(m.forearm_girth-1.0)*forearm
        +0.7*(m.wrist_width-1.0)*wrist;
    out.z += p.z*arm_girth*height_scale;
    out.x += side*0.038*arm_girth*height_scale;
    // Hands scale about their reference palm centres.
    let hand_centre = Vec3::new(side*0.63, 0.99, 0.0);
    out += (p-hand_centre)*(m.hand_size-1.0)*hand*height_scale;

    // Calibrated around the leg centre lines, preserving left/right symmetry.
    let leg = smoothstep(0.07, 0.12, abs_x);
    let thigh = band(p.y, .48,.60,.78,.94)*leg;
    let calf = band(p.y, .14,.25,.43,.57)*leg;
    let ankle = band(p.y, .065,.10,.16,.21)*leg;
    let leg_girth = 0.75*(m.thigh_girth-1.0)*thigh
        + 0.75*(m.calf_girth-1.0)*calf
        + 0.6*(m.ankle_width-1.0)*ankle;
    out.x += side*(abs_x-0.13)*leg_girth*height_scale;
    out.z += p.z*leg_girth*height_scale;
    let foot = (1.0 - smoothstep(.095,.17,p.y))*leg;
    out.z += (p.z-0.065)*(m.foot_length-1.0)*foot*height_scale;
    out.x += side*(abs_x-0.13)*(m.foot_width-1.0)*foot*height_scale;
    out
}
