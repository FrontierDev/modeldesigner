//! Chest-and-shoulder anatomical reference surface (generator v3).
//!
//! This revises the sternum, pectoralis, clavicle, deltoid and axillary shapes while retaining one fixed
//! indexed mesh per generator version. It is a sculpting reference, not a
//! deformation-ready quad mesh or a substitute for anatomical retopology.

use glam::Vec3;
use crate::{ellipsoid, tapered_capsule, union_smooth};

/// Controlled surface indentation; preserve a closed mesh without cutting holes.
fn subtract_smooth(body: f32, cutter: f32, blend: f32) -> f32 {
    -union_smooth(-body, cutter, blend)
}

/// Box SDF with rounded edges. Allows broad, near-planar pectoral tissue instead
/// of balloon-like chest ellipsoids.
fn rounded_box(p: Vec3, centre: Vec3, half: Vec3, radius: f32) -> f32 {
    let q = (p - centre).abs() - half;
    q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - radius
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
        // A flattened pectoral fan spans sternum to the front of the axilla.
        // A slight lateral slope follows the clavicular/upper-pectoral line.
        let pec_point = Vec3::new(
            p.x.abs(),
            p.y + 0.14 * (p.x.abs() - 0.11),
            p.z,
        );
        d = union_smooth(
            d,
            rounded_box(
                pec_point,
                Vec3::new(0.108, 1.338, 0.090),
                Vec3::new(0.086, 0.047, 0.039),
                0.026,
            ),
            0.019,
        );
        // Clavicle shelf from manubrium to acromion: this controls shoulder
        // width without an oversized deltoid ball.
        d = union_smooth(
            d,
            tapered_capsule(
                p,
                Vec3::new(side * 0.050, 1.449, 0.080),
                Vec3::new(side * 0.220, 1.426, 0.035),
                0.025, 0.022,
            ),
            0.029,
        );
        // Posterior upper-trapezius slope anchors shoulder to the neck.
        d = union_smooth(
            d,
            tapered_capsule(
                p,
                Vec3::new(side * 0.065, 1.469, -0.045),
                Vec3::new(side * 0.218, 1.421, -0.044),
                0.037, 0.028,
            ),
            0.027,
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
                p, acromion, Vec3::new(side * 0.323, 1.296, -0.004),
                0.060, 0.048,
            ),
            0.024,
        );
        d = union_smooth(d, tapered_capsule(p, shoulder, elbow, 0.065, 0.052), 0.026);
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
                Vec3::new(side * 0.219, 1.313, 0.069),
                Vec3::new(0.044, 0.059, 0.052),
            ),
            0.011,
        );
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_mirrors_across_sagittal_plane() {
        for pos in [
            Vec3::new(0.17, 1.34, 0.14),
            Vec3::new(0.23, 1.42, 0.04),
            Vec3::new(0.24, 1.32, 0.08),
            Vec3::new(0.40, 1.19, 0.02),
        ] {
            assert!((field(pos) - field(Vec3::new(-pos.x, pos.y, pos.z))).abs() < 1e-5);
        }
    }

    #[test]
    fn deltoid_cap_does_not_project_as_an_oversized_sphere() {
        // The old generator encloses this exaggerated high lateral shoulder.
        let point = Vec3::new(0.29, 1.475, 0.0);
        assert!(field(point) > 0.0);
    }

    #[test]
    fn clavicle_ridge_is_present_anterior_to_acromion() {
        let anterior = Vec3::new(0.15, 1.446, 0.081);
        assert!(field(anterior) < 0.0);
    }
}
