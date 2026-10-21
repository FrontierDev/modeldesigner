//! Humanoid reference geometry: torso, shoulder girdle and limbs.
//!
//! The single active surface may change in place during prototyping.
//! It is not animation-ready retopology.

use glam::Vec3;
use crate::{ellipsoid, tapered_capsule, union_smooth};

/// Controlled surface indentation; preserve a closed mesh without cutting holes.
fn subtract_smooth(body: f32, cutter: f32, blend: f32) -> f32 {
    -union_smooth(-body, cutter, blend)
}

/// Thin elliptical pectoral fan with a gentle lateral slope, not a rectangular ledge.
fn pectoral_fan(p: Vec3) -> f32 {
    let x = p.x.abs();
    ellipsoid(
        Vec3::new(x, p.y + 0.13 * (x - 0.10), p.z),
        Vec3::new(0.108, 1.330, 0.090),
        Vec3::new(0.102, 0.059, 0.049),
    )
}

pub(crate) fn field(p: Vec3) -> f32 {
    // Anatomical ribcage: a broad thoracic volume narrows to the waist.
    // There is no separate inflated sphere at the upper chest.
    let mut d = ellipsoid(
        p, Vec3::new(0.0, 1.215, -0.012), Vec3::new(0.196, 0.258, 0.135),
    );
    d = union_smooth(
        d, ellipsoid(p, Vec3::new(0.0, 1.344, -0.013), Vec3::new(0.200, 0.127, 0.126)), 0.043,
    );
    d = union_smooth(
        d, ellipsoid(p, Vec3::new(0.0, 1.058, 0.002), Vec3::new(0.167, 0.154, 0.121)), 0.052,
    );
    d = union_smooth(
        d, ellipsoid(p, Vec3::new(0.0, 0.918, -0.006), Vec3::new(0.186, 0.119, 0.137)), 0.048,
    );
    for side in [-1.0_f32, 1.0] {
        d = union_smooth(d, pectoral_fan(p), 0.040);
        // Clavicle shelf from manubrium to acromion: this controls shoulder
        // width without an oversized deltoid ball.
        d = union_smooth(
            d,
            tapered_capsule(
                p,
                Vec3::new(side * 0.050, 1.449, 0.080),
                Vec3::new(side * 0.220, 1.426, 0.035),
                0.017, 0.015,
            ),
            0.019,
        );
        // Posterior upper-trapezius slope anchors shoulder to the neck.
        d = union_smooth(
            d,
            tapered_capsule(
                p,
                Vec3::new(side * 0.065, 1.469, -0.045),
                Vec3::new(side * 0.218, 1.421, -0.044),
                0.030, 0.023,
            ),
            0.023,
        );
        d = union_smooth(
            d,
            ellipsoid(
                p, Vec3::new(side * 0.102, 0.909, -0.050),
                Vec3::new(0.099, 0.090, 0.119),
            ),
            0.044,
        );
    }

    // Neck and head: separate cranium, tapered lower face, chin and cheeks.
    d = union_smooth(
        d,
        tapered_capsule(
            p, Vec3::new(0.0, 1.427, -0.008),
            Vec3::new(0.0, 1.566, -0.008), 0.069, 0.055,
        ),
        0.045,
    );
    d = union_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.656, -0.008), Vec3::new(0.095, 0.117, 0.092)),
        0.032,
    );
    d = union_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.583, 0.024), Vec3::new(0.077, 0.067, 0.083)),
        0.034,
    );
    d = union_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.542, 0.058), Vec3::new(0.047, 0.025, 0.042)),
        0.019,
    );
    for side in [-1.0_f32, 1.0] {
        // Cheeks, brow ridges and ears; positions mirror left/right.
        d = union_smooth(
            d,
            ellipsoid(
                p, Vec3::new(side * 0.059, 1.615, 0.060),
                Vec3::new(0.040, 0.035, 0.039),
            ),
            0.020,
        );
        d = union_smooth(
            d,
            ellipsoid(
                p, Vec3::new(side * 0.045, 1.672, 0.070),
                Vec3::new(0.043, 0.019, 0.025),
            ),
            0.013,
        );
        d = union_smooth(
            d,
            ellipsoid(
                p, Vec3::new(side * 0.101, 1.619, -0.007),
                Vec3::new(0.021, 0.039, 0.024),
            ),
            0.010,
        );
    }
    // Nose bridge, nose tip and lip silhouette face forward (+Z).
    d = union_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.627, 0.104), Vec3::new(0.022, 0.043, 0.032)),
        0.018,
    );
    d = union_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.592, 0.133), Vec3::new(0.027, 0.019, 0.037)),
        0.012,
    );
    d = union_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.558, 0.100), Vec3::new(0.041, 0.013, 0.018)),
        0.009,
    );
    // Shallow orbital and lip recesses add recognisable light/shadow features.
    for side in [-1.0_f32, 1.0] {
        d = subtract_smooth(
            d,
            ellipsoid(
                p, Vec3::new(side * 0.043, 1.652, 0.094),
                Vec3::new(0.026, 0.013, 0.017),
            ),
            0.004,
        );
    }
    d = subtract_smooth(
        d,
        ellipsoid(p, Vec3::new(0.0, 1.553, 0.122), Vec3::new(0.035, 0.004, 0.012)),
        0.002,
    );

    for side in [-1.0_f32, 1.0] {
        // The deltoid drapes down around the humeral head: no shoulder sphere.
        // The lateral contour begins at the acromion and narrows into biceps.
        let acromion = Vec3::new(side * 0.229, 1.419, 0.003);
        let shoulder = Vec3::new(side * 0.253, 1.395, -0.003);
        let elbow = Vec3::new(side * 0.408, 1.191, -0.002);
        let wrist = Vec3::new(side * 0.572, 1.043, 0.002);
        d = union_smooth(
            d,
            tapered_capsule(
                p, acromion, Vec3::new(side * 0.310, 1.300, -0.004),
                0.044, 0.048,
            ),
            0.028,
        );
        d = union_smooth(d, tapered_capsule(p, shoulder, elbow, 0.058, 0.052), 0.028);
        d = union_smooth(d, tapered_capsule(p, elbow, wrist, 0.058, 0.036), 0.025);
        d = union_smooth(
            d,
            ellipsoid(p, Vec3::new(side * 0.613, 1.012, 0.003),
                Vec3::new(0.078, 0.040, 0.052)),
            0.018,
        );
        // Thumb projects toward the front/torso side of the hand.
        d = union_smooth(
            d,
            tapered_capsule(
                p, Vec3::new(side * 0.591, 1.015, 0.025),
                Vec3::new(side * 0.612, 0.956, 0.064), 0.024, 0.015,
            ),
            0.013,
        );

        // Thigh transitions through a defined knee to calf, narrow ankle,
        // heel and long forward-pointing foot.
        let hip = Vec3::new(side * 0.115, 0.888, -0.003);
        let knee = Vec3::new(side * 0.126, 0.478, 0.004);
        let ankle = Vec3::new(side * 0.129, 0.105, -0.011);
        d = union_smooth(d, tapered_capsule(p, hip, knee, 0.109, 0.064), 0.045);
        d = union_smooth(
            d,
            ellipsoid(p, Vec3::new(side * 0.126, 0.472, 0.015),
                Vec3::new(0.066, 0.060, 0.067)),
            0.018,
        );
        d = union_smooth(d, tapered_capsule(p, knee, ankle, 0.064, 0.039), 0.030);
        d = union_smooth(
            d,
            ellipsoid(p, Vec3::new(side * 0.126, 0.338, -0.038),
                Vec3::new(0.067, 0.133, 0.063)),
            0.027,
        );
        d = union_smooth(
            d,
            ellipsoid(p, Vec3::new(side * 0.129, 0.064, 0.084),
                Vec3::new(0.062, 0.052, 0.153)),
            0.028,
        );
        d = union_smooth(
            d,
            ellipsoid(p, Vec3::new(side * 0.129, 0.048, 0.186),
                Vec3::new(0.074, 0.037, 0.060)),
            0.020,
        );
    }
    // Axilla: distinguish anterior pectoral wall from upper-arm contour.
    // Use a shallow recess, not an open hole or an unrelated limb shell.
    for side in [-1.0_f32, 1.0] {
        d = subtract_smooth(
            d,
            ellipsoid(
                p,
                Vec3::new(side * 0.211, 1.293, 0.055),
                Vec3::new(0.030, 0.041, 0.040),
            ),
            0.009,
        );
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_body_mirrors_across_center() {
        for p in [
            Vec3::new(0.10, 1.33, 0.12),
            Vec3::new(0.22, 1.42, 0.03),
            Vec3::new(0.25, 1.31, 0.04),
        ] {
            assert!((field(p) - field(Vec3::new(-p.x, p.y, p.z))).abs() < 1e-5);
        }
    }

    #[test]
    fn upper_body_values_remain_finite() {
        for x in [-0.3_f32, -0.1, 0.0, 0.1, 0.3] {
            for y in [1.25_f32, 1.35, 1.45] {
                for z in [-0.15_f32, 0.0, 0.15] {
                    assert!(field(Vec3::new(x, y, z)).is_finite());
                }
            }
        }
    }
}
