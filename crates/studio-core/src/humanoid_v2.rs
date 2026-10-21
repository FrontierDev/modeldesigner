//! Human-anatomy-oriented implicit reference surface (generator v2).
//!
//! This supplies a more recognizable neutral anatomy while retaining one fixed
//! indexed mesh per generator version. It is a sculpting reference, not a
//! deformation-ready quad mesh or a substitute for anatomical retopology.

use glam::Vec3;
use crate::{ellipsoid, tapered_capsule, union_smooth};

/// Controlled surface indentation; preserve a closed mesh without cutting holes.
fn subtract_smooth(body: f32, cutter: f32, blend: f32) -> f32 {
    -union_smooth(-body, cutter, blend)
}

pub(crate) fn field(p: Vec3) -> f32 {
    // Ribcage flows into a comparatively narrow waist and broad pelvis.
    let mut d = ellipsoid(
        p, Vec3::new(0.0, 1.215, -0.008), Vec3::new(0.193, 0.259, 0.135),
    );
    d = union_smooth(
        d, ellipsoid(p, Vec3::new(0.0, 1.355, -0.004), Vec3::new(0.219, 0.128, 0.128)), 0.067,
    );
    d = union_smooth(
        d, ellipsoid(p, Vec3::new(0.0, 1.058, 0.002), Vec3::new(0.167, 0.154, 0.121)), 0.063,
    );
    d = union_smooth(
        d, ellipsoid(p, Vec3::new(0.0, 0.918, -0.006), Vec3::new(0.186, 0.119, 0.137)), 0.055,
    );
    // Subtle chest planes rather than a single inflated spherical upper torso.
    for side in [-1.0_f32, 1.0] {
        d = union_smooth(
            d,
            ellipsoid(
                p,
                Vec3::new(side * 0.099, 1.324, 0.051),
                Vec3::new(0.100, 0.077, 0.087),
            ),
            0.027,
        );
        d = union_smooth(
            d,
            ellipsoid(
                p,
                Vec3::new(side * 0.102, 0.909, -0.050),
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
        // Anatomical shoulders: flatter deltoid silhouette, tapered biceps,
        // elbow, forearm and a distinct flattened palm.
        let shoulder = Vec3::new(side * 0.224, 1.421, -0.003);
        let elbow = Vec3::new(side * 0.408, 1.191, -0.002);
        let wrist = Vec3::new(side * 0.572, 1.043, 0.002);
        d = union_smooth(
            d,
            ellipsoid(p, Vec3::new(side * 0.244, 1.399, -0.001),
                Vec3::new(0.078, 0.098, 0.083)),
            0.044,
        );
        d = union_smooth(d, tapered_capsule(p, shoulder, elbow, 0.080, 0.055), 0.047);
        d = union_smooth(d, tapered_capsule(p, elbow, wrist, 0.060, 0.036), 0.028);
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
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference_field;

    #[test]
    fn facial_silhouette_includes_a_nose_beyond_legacy_head() {
        let tip = Vec3::new(0.0, 1.592, 0.153);
        assert!(field(tip) < 0.0);
        assert!(reference_field(tip) > 0.0);
    }

    #[test]
    fn face_and_limbs_are_symmetric_left_to_right() {
        for point in [
            Vec3::new(0.060, 1.625, 0.079),
            Vec3::new(0.113, 1.619, 0.0),
            Vec3::new(0.51, 1.10, 0.03),
            Vec3::new(0.13, 0.45, 0.06),
        ] {
            let other = Vec3::new(-point.x, point.y, point.z);
            assert!((field(point) - field(other)).abs() < 1e-5);
        }
    }
}
