//! Continuous anatomical upper-body target. These are shape constraints,
//! not the indexed quad mesh, skinning or an animation rig.
use crate::Recipe;
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
struct Section {
    y: f32,
    width: f32,
    front: f32,
    back: f32,
    z_centre: f32,
}

#[derive(Clone, Copy, Debug)]
struct Arm {
    shoulder: Vec3,
    elbow: Vec3,
    proximal: f32,
    distal: f32,
    front: f32,
    back: f32,
    deltoid_radius: Vec3,
}

impl Arm {
    fn field(&self, p: Vec3) -> f32 {
        let axis = self.elbow - self.shoulder;
        let t = ((p - self.shoulder).dot(axis) / axis.length_squared())
            .clamp(0.0, 1.0);
        let centre = self.shoulder.lerp(self.elbow, t);
        let direction = axis.normalize();
        let forward = (Vec3::Z - direction * direction.z).normalize();
        let vertical = direction.cross(forward).normalize();
        let difference = p - centre;
        let radius = self.proximal + (self.distal - self.proximal) * t;
        // Anterior biceps and posterior triceps change thickness independently.
        let sagittal = if difference.dot(forward) >= 0.0 {
            self.front
        } else {
            self.back
        };
        let scaled = Vec3::new(
            difference.dot(vertical) / radius,
            difference.dot(forward) / sagittal,
            difference.dot(direction) / radius,
        );
        let shaft = (scaled.length() - 1.0) * radius;
        let cap = (p - self.shoulder) / self.deltoid_radius;
        let shoulder = (cap.length() - 1.0) * self.deltoid_radius.min_element();
        smooth_union(shaft, shoulder, 0.038)
    }
}

fn smooth_union(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

#[derive(Clone, Debug)]
pub(crate) struct TorsoTarget {
    sections: [Section; 10],
    arms: [Arm; 2],
    pub(crate) shoulders: [Vec3; 2],
    pub(crate) elbows: [Vec3; 2],
    pub(crate) neck_top: Vec3,
}

impl TorsoTarget {
    pub(crate) fn from_recipe(recipe: &Recipe) -> Self {
        let m = &recipe.morphology;
        let scale = recipe.height / 1.75;
        let muscle = recipe.musculature - 0.5;
        let mass = recipe.body_mass - 0.5;
        // Values specify Y, half-width, anterior depth and posterior depth,
        // in reference-height metres. The chest narrows into a neck stump.
        const BASE: [(f32, f32, f32, f32); 10] = [
            (0.72, 0.151, 0.119, 0.123),
            (0.83, 0.137, 0.106, 0.115),
            (0.97, 0.126, 0.104, 0.108),
            (1.10, 0.151, 0.119, 0.116),
            (1.21, 0.173, 0.127, 0.120),
            (1.32, 0.188, 0.134, 0.123),
            (1.39, 0.169, 0.120, 0.119),
            (1.45, 0.108, 0.086, 0.090),
            (1.51, 0.070, 0.071, 0.070),
            (1.58, 0.065, 0.068, 0.066),
        ];
        let sections = std::array::from_fn(|i| {
            let (base_y, base_w, front, back) = BASE[i];
            let chest = ((base_y - 1.02) / 0.22).clamp(0.0, 1.0)
                * ((1.47 - base_y) / 0.14).clamp(0.0, 1.0);
            let waist = ((1.19 - base_y) / 0.35).clamp(0.0, 1.0);
            let neck = ((base_y - 1.43) / 0.10).clamp(0.0, 1.0);
            let ribs = ((base_y - 1.02) / 0.19).clamp(0.0, 1.0)
                * ((1.36 - base_y) / 0.15).clamp(0.0, 1.0);
            let abdomen = ((1.20 - base_y) / 0.35).clamp(0.0, 1.0);
            let width = base_w
                * (1.0 + waist * (m.waist_width - 1.0))
                * (1.0 + chest * (m.chest_width - 1.0))
                * (1.0 + ribs * (m.ribcage_taper - 1.0) * 0.45)
                * (1.0 + chest * (recipe.shoulder_width / 0.46 - 1.0) * 0.40)
                * (1.0 + neck * (m.neck_width - 1.0))
                * (1.0 + mass * waist * 0.14 + muscle * chest * 0.07);
            let front = front
                * (1.0 + chest * (m.chest_depth - 1.0))
                * (1.0 + waist * (m.waist_depth - 1.0))
                * (1.0 + neck * (m.neck_width - 1.0) * 0.6)
                + abdomen * (0.024 * (m.abdomen_fullness - 0.5)
                    + 0.013 * mass)
                + chest * (0.027 * (m.pectoral_fullness - 0.5)
                    + 0.010 * muscle);
            let back = back
                * (1.0 + chest * (m.chest_depth - 1.0))
                * (1.0 + waist * (m.waist_depth - 1.0))
                * (1.0 + neck * (m.neck_width - 1.0) * 0.6)
                + chest * (0.032 * (m.upper_back_fullness - 0.5));
            Section {
                y: (0.78 + (base_y - 0.78) * m.torso_length
                    + (m.ribcage_height - 1.0) * ribs * 0.05
                    + (m.neck_length - 1.0) * neck * 0.09) * scale,
                width: width * scale,
                front: front * scale,
                back: back * scale,
                z_centre: (0.033 * (m.back_curvature - 0.5)
                    * ((base_y - 0.93) / 0.39).clamp(0.0, 1.0)
                    + 0.035 * (m.shoulder_forward - 0.5) * chest) * scale,
            }
        });
        let arms = std::array::from_fn(|i| {
            let side = if i == 0 { -1.0 } else { 1.0 };
            let shoulder = Vec3::new(
                side * recipe.shoulder_width * 0.50,
                1.352 - (m.shoulder_slope - 0.5) * 0.085,
                0.020 + 0.095 * (m.shoulder_forward - 0.5),
            ) * scale;
            let elbow = shoulder + Vec3::new(
                side * 0.285 * recipe.arm_length,
                -0.215 * recipe.arm_length,
                0.0,
            ) * scale;
            let shoulder_volume = m.deltoid_volume - 0.5;
            let roundness = m.deltoid_roundness - 0.5;
            let radius = m.upper_arm_girth * (1.0 + 0.13 * mass);
            Arm {
                shoulder,
                elbow,
                proximal: (0.067 + 0.028 * shoulder_volume) * radius * scale,
                distal: (0.052 + 0.012 * (m.arm_taper - 1.0)
                    + 0.018 * (m.elbow_width - 1.0)) * radius * scale,
                front: (0.064 + 0.036 * (m.biceps_volume - 0.5)) * radius * scale,
                back: (0.065 + 0.037 * (m.triceps_volume - 0.5)) * radius * scale,
                deltoid_radius: Vec3::new(
                    (0.079 + 0.026 * shoulder_volume) * scale,
                    (0.079 + 0.024 * roundness) * scale,
                    (0.075 + 0.014 * shoulder_volume) * scale,
                ),
            }
        });
        Self {
            shoulders: arms.map(|a| a.shoulder),
            elbows: arms.map(|a| a.elbow),
            neck_top: Vec3::new(0.0, sections[9].y, sections[9].z_centre),
            sections,
            arms,
        }
    }

    /// Approximate signed distance to the variable elliptical torso sections,
    /// with rounded implicit junctions at each shoulder and upper arm.
    pub(crate) fn field(&self, point: Vec3) -> f32 {
        let first = self.sections[0];
        let last = self.sections[9];
        let y = point.y.clamp(first.y, last.y);
        let band = self.sections.windows(2)
            .position(|w| y <= w[1].y).unwrap_or(8);
        let a = self.sections[band];
        let b = self.sections[band + 1];
        let t = ((y - a.y) / (b.y - a.y)).clamp(0.0, 1.0);
        // Cubic Hermite interpolation prevents visible hard steps at sections.
        let t = t * t * (3.0 - 2.0 * t);
        let width = a.width + (b.width - a.width) * t;
        let front = a.front + (b.front - a.front) * t;
        let back = a.back + (b.back - a.back) * t;
        let z_centre = a.z_centre + (b.z_centre - a.z_centre) * t;
        let z = point.z - z_centre;
        let depth = if z >= 0.0 { front } else { back };
        let cross_section = ((point.x / width).powi(2) + (z / depth).powi(2)).sqrt();
        let radial = (cross_section - 1.0) * width.min(depth);
        let axial = (first.y - point.y).max(point.y - last.y);
        // Rounded intersection with the finite vertical interval.
        let outside = Vec3::new(radial.max(0.0), axial.max(0.0), 0.0).length();
        let inside = radial.max(axial).min(0.0);
        let mut result = outside + inside;
        for arm in self.arms {
            result = smooth_union(result, arm.field(point), 0.038 * (last.y / 1.58));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torso_has_distinct_waist_chest_and_neck() {
        let torso = TorsoTarget::from_recipe(&Recipe::default());
        assert!(torso.sections[5].width > torso.sections[2].width * 1.35);
        assert!(torso.sections[9].width < torso.sections[5].width * 0.5);
        assert!(torso.field(Vec3::new(0.0, 1.13, 0.0)) < 0.0);
        assert!(torso.field(Vec3::new(0.40, 1.13, 0.0)) > 0.0);
        assert!(torso.field(Vec3::new(0.0, 1.70, 0.0)) > 0.0);
    }

    #[test]
    fn shoulder_and_upper_arm_are_continuous_with_torso() {
        let torso = TorsoTarget::from_recipe(&Recipe::default());
        for i in 0..2 {
            assert!(torso.field(torso.shoulders[i]) < 0.0);
            let midpoint = torso.shoulders[i].lerp(torso.elbows[i], 0.5);
            assert!(torso.field(midpoint) < 0.0);
        }
    }

    #[test]
    fn upper_body_sliders_have_distinct_effects() {
        let original = TorsoTarget::from_recipe(&Recipe::default());
        let mut wide = Recipe::default();
        wide.morphology.chest_width = 1.25;
        let wider = TorsoTarget::from_recipe(&wide);
        assert!(wider.sections[5].width > original.sections[5].width);
        let mut arms = Recipe::default();
        arms.arm_length = 1.18;
        let longer = TorsoTarget::from_recipe(&arms);
        assert!(longer.elbows[1].x > original.elbows[1].x);
        let mut neck = Recipe::default();
        neck.morphology.neck_length = 1.15;
        let taller = TorsoTarget::from_recipe(&neck);
        assert!(taller.neck_top.y > original.neck_top.y);
    }
}
