//! Model Designer desktop shell and inspectable, material-free humanoid preview.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};
use glam::Vec3;
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult};
use std::path::{Path, PathBuf};
use studio_core::{BuildControl, BuildSection, BUILD_CONTROLS, Model, Recipe};
use studio_core::rig_preview::{self, DiagnosticPose, DeformationReport};
use studio_core::anatomy::{AnatomyTarget, Landmark, TargetSurface};
use studio_core::cage::{self, FitReport};
use studio_core::controls::build_schema_json;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1380.0, 840.0]),
        ..Default::default()
    };
    eframe::run_native("Model Designer — Ninth Age", options, Box::new(|cc| Ok(Box::new(StudioApp::new(cc)))))
}

#[derive(Clone)]
struct Camera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
}

impl Camera {
    fn framed(height: f32) -> Self {
        Self { yaw: 0.48, pitch: 0.12, distance: height * 1.45, target: Vec3::Y * (height * 0.57) }
    }

    fn for_anatomy(height: f32) -> Self {
        let scale = height / 1.75;
        // Frame neck through upper thighs rather than just the pelvic study.
        Self { yaw: 0.38, pitch: 0.10, distance: 2.72 * scale,
            target: Vec3::Y * (0.98 * scale) }
    }

    fn basis(&self) -> Basis {
        let eye = self.target + Vec3::new(
            self.yaw.sin() * self.pitch.cos() * self.distance,
            self.pitch.sin() * self.distance,
            self.yaw.cos() * self.pitch.cos() * self.distance,
        );
        let forward = (self.target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        Basis { eye, forward, right, up }
    }
}

struct Basis {
    eye: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
}

fn project(p: Vec3, basis: &Basis, rect: Rect) -> Option<(Pos2, f32)> {
    let relative = p - basis.eye;
    let depth = relative.dot(basis.forward);
    if depth <= 0.01 { return None; }
    let focal = rect.height() * 1.05;
    let x = relative.dot(basis.right) * focal / depth;
    let y = relative.dot(basis.up) * focal / depth;
    Some((rect.center() + egui::vec2(x, -y), depth))
}

#[derive(Clone, Copy)]
struct Face {
    points: [Pos2; 3],
    depth: f32,
    colors: [Color32; 3],
}

/// Render triangles without the anti-aliased convex-polygon tessellator.
/// Its miter expansion produces enormous vertices for narrow triangles,
/// creating the apparent spikes in otherwise valid humanoid geometry.
fn triangle_mesh(faces: &[Face]) -> egui::epaint::Mesh {
    let mut mesh = egui::epaint::Mesh::default();
    mesh.vertices.reserve(faces.len() * 3);
    mesh.indices.reserve(faces.len() * 3);
    for face in faces {
        let start = mesh.vertices.len() as u32;
        for (&pos, &color) in face.points.iter().zip(face.colors.iter()) {
            mesh.vertices.push(egui::epaint::Vertex {
                pos,
                uv: egui::epaint::WHITE_UV,
                color,
            });
        }
        mesh.indices.extend_from_slice(&[start, start + 1, start + 2]);
    }
    mesh
}

/// Same fixed top-right statistics panel pattern as envmoddesigner.
fn draw_model_statistics(
    painter: &egui::Painter, viewport: Rect, model: &Model, recipe: &Recipe,
    anatomy_target: bool, rigging: bool,
) {
    let panel = Rect::from_min_size(
        viewport.right_top() + egui::vec2(-194.0, 12.0),
        egui::vec2(182.0, 166.0),
    );
    painter.rect_filled(panel, 5.0, Color32::from_rgba_unmultiplied(28, 32, 38, 228));
    let entries = [
        ("MODEL", String::new()),
        ("Study", if anatomy_target {
            "Anatomy target".to_owned()
        } else if rigging {
            "Rigging preview".to_owned()
        } else {
            "Fitted cage".to_owned()
        }),
        ("Height", format!("{:.2} m", recipe.height)),
        ("Vertices", model.vertices.len().to_string()),
        ("Quads", if anatomy_target { "Not fitted".to_owned() } else { model.quads.len().to_string() }),
        ("Triangles", model.triangles.len().to_string()),
        ("Joints", if rigging {
            format!("{} provisional", model.joints.len())
        } else { "Not rigged".to_owned() }),
    ];
    for (index, (label, value)) in entries.iter().enumerate() {
        let y = panel.top() + 11.0 + index as f32 * 21.0;
        painter.text(
            egui::pos2(panel.left() + 11.0, y),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::proportional(11.0),
            Color32::from_gray(if index == 0 { 215 } else { 185 }),
        );
        if !value.is_empty() {
            painter.text(
                egui::pos2(panel.right() - 11.0, y),
                egui::Align2::RIGHT_TOP,
                value,
                egui::FontId::monospace(11.0),
                Color32::from_gray(215),
            );
        }
    }
}

fn shaded_vertex(normal: Vec3, light: Vec3) -> Color32 {
    let illumination = 0.39 + 0.61 * normal.dot(light).max(0.0);
    Color32::from_rgb(
        (174.0 * illumination) as u8,
        (185.0 * illumination) as u8,
        (204.0 * illumination) as u8,
    )
}

fn preview(
    ui: &mut egui::Ui, model: &Model, recipe: &Recipe,
    camera: &mut Camera, wireframe: bool, skeleton: bool, anatomy_target: bool,
    landmarks: &[Landmark], show_poles: bool, rigging: bool,
    defect_vertices: &[u32],
) {
    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        camera.yaw -= d.x * 0.008;
        camera.pitch = (camera.pitch + d.y * 0.006).clamp(-1.4, 1.4);
    }
    if response.dragged_by(egui::PointerButton::Secondary) {
        let d = response.drag_delta();
        let basis = camera.basis();
        camera.target += (-basis.right * d.x + basis.up * d.y) * (camera.distance * 0.0018);
    }
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        camera.distance = (camera.distance * (-scroll * 0.0015).exp()).clamp(1.1, 20.0);
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(24, 27, 32));
    let basis = camera.basis();

    // Subtle ground axes establish scale and stance.
    let grid_stroke = Stroke::new(0.7, Color32::from_rgb(53, 58, 66));
    for step in -5..=5 {
        let offset = step as f32 * 0.25;
        for (a, b) in [(Vec3::new(offset, 0.0, -1.25), Vec3::new(offset, 0.0, 1.25)),
                       (Vec3::new(-1.25, 0.0, offset), Vec3::new(1.25, 0.0, offset))] {
            if let (Some((a, _)), Some((b, _))) = (project(a, &basis, rect), project(b, &basis, rect)) {
                painter.line_segment([a, b], grid_stroke);
            }
        }
    }

    let light = Vec3::new(-0.55, 0.85, 0.35).normalize();
    let mut faces = Vec::with_capacity(model.triangles.len());
    for &[a, b, c] in &model.triangles {
        let va = Vec3::from_array(model.vertices[a as usize]);
        let vb = Vec3::from_array(model.vertices[b as usize]);
        let vc = Vec3::from_array(model.vertices[c as usize]);
        let normal = (vb - va).cross(vc - va).normalize_or_zero();
        if normal == Vec3::ZERO || normal.dot(basis.eye - va) <= 0.0 { continue; }
        let (Some((pa, da)), Some((pb, db)), Some((pc, dc))) =
            (project(va, &basis, rect), project(vb, &basis, rect), project(vc, &basis, rect))
        else { continue; };
        let colors = [a, b, c].map(|index| {
            shaded_vertex(Vec3::from_array(model.normals[index as usize]), light)
        });
        faces.push(Face { points: [pa, pb, pc], depth: (da + db + dc) / 3.0, colors });
    }
    faces.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    // Bypass convex-polygon anti-aliasing; it can stretch thin triangles
    // across the entire viewport. The GPU mesh retains the exact positions.
    painter.add(egui::Shape::mesh(triangle_mesh(&faces)));
    if wireframe {
        // The fixed-topology study draws quad edges; the anatomy study draws
        // provisional triangle edges (no claim of final topology).
        let stroke = Stroke::new(0.6, Color32::from_rgb(14, 20, 27));
        for &quad in &model.quads {
            let a = Vec3::from_array(model.vertices[quad[0] as usize]);
            let b = Vec3::from_array(model.vertices[quad[1] as usize]);
            let c = Vec3::from_array(model.vertices[quad[2] as usize]);
            let normal = (b - a).cross(c - a);
            if normal.dot(basis.eye - a) <= 0.0 { continue; }
            for edge in 0..4 {
                let v0 = Vec3::from_array(model.vertices[quad[edge] as usize]);
                let v1 = Vec3::from_array(model.vertices[quad[(edge + 1) % 4] as usize]);
                if let (Some((p0, _)), Some((p1, _))) =
                    (project(v0, &basis, rect), project(v1, &basis, rect))
                {
                    painter.line_segment([p0, p1], stroke);
                }
            }
        }
        if anatomy_target {
            use std::collections::HashSet;
            let mut edges = HashSet::<(u32,u32)>::new();
            for &[a, b, c] in &model.triangles {
                let va = Vec3::from_array(model.vertices[a as usize]);
                let vb = Vec3::from_array(model.vertices[b as usize]);
                let vc = Vec3::from_array(model.vertices[c as usize]);
                if (vb - va).cross(vc - va).dot(basis.eye - va) <= 0.0 {
                    continue;
                }
                for (u,v) in [(a,b),(b,c),(c,a)] {
                    let key = (u.min(v),u.max(v));
                    if !edges.insert(key) { continue; }
                    if let (Some((p0,_)),Some((p1,_))) = (
                        project(Vec3::from_array(model.vertices[u as usize]),&basis,rect),
                        project(Vec3::from_array(model.vertices[v as usize]),&basis,rect),
                    ) {
                        painter.line_segment([p0,p1],stroke);
                    }
                }
            }
        }
        // Only the temporary open-end study caps are genuinely triangulated.
        for &[a, b, c] in &model.caps {
            let va = Vec3::from_array(model.vertices[a as usize]);
            let vb = Vec3::from_array(model.vertices[b as usize]);
            let vc = Vec3::from_array(model.vertices[c as usize]);
            if (vb - va).cross(vc - va).dot(basis.eye - va) <= 0.0 { continue; }
            if let (Some((pa, _)), Some((pb, _)), Some((pc, _))) =
                (project(va, &basis, rect), project(vb, &basis, rect), project(vc, &basis, rect))
            {
                for edge in [(pa, pb), (pb, pc), (pc, pa)] {
                    painter.line_segment([edge.0, edge.1], stroke);
                }
            }
        }
    }
    if skeleton {
        for joint in &model.joints {
            let Some(parent_index) = joint.parent else { continue; };
            let pa = Vec3::from_array(model.joints[parent_index].position);
            let pb = Vec3::from_array(joint.position);
            if let (Some((a, _)), Some((b, _))) = (project(pa, &basis, rect), project(pb, &basis, rect)) {
                painter.line_segment([a, b], Stroke::new(2.3, Color32::from_rgb(234, 179, 87)));
                painter.circle_filled(b, 3.0, Color32::from_rgb(255, 209, 110));
            }
        }
    }
    if show_poles && !model.quads.is_empty() {
        // A valence-4 vertex is locally regular on an all-quad manifold.
        // Show irregular poles rather than hiding them behind shading.
        let mut adjacent = vec![Vec::<u32>::new(); model.vertices.len()];
        for &[a,b,c,d] in &model.quads {
            for (u,v) in [(a,b),(b,c),(c,d),(d,a)] {
                adjacent[u as usize].push(v);
                adjacent[v as usize].push(u);
            }
        }
        for (i, ring) in adjacent.iter_mut().enumerate() {
            ring.sort_unstable();
            ring.dedup();
            if ring.len() == 4 { continue; }
            let location = Vec3::from_array(model.vertices[i]);
            let normal = Vec3::from_array(model.normals[i]);
            if normal.dot(basis.eye - location) <= 0.0 { continue; }
            if let Some((p,_)) = project(location, &basis, rect) {
                let color = if ring.len() >= 6 {
                    Color32::from_rgb(245, 91, 81)
                } else { Color32::from_rgb(245, 194, 84) };
                painter.circle_filled(p, 2.7, color);
            }
        }
    }
    // Highlight high-strain and inverted-face vertices on the actual posed
    // cage. These are diagnostics, not simulated collision or corrective skinning.
    for &id in defect_vertices {
        let index = id as usize;
        let Some(&xyz) = model.vertices.get(index) else { continue; };
        let position = Vec3::from_array(xyz);
        if let Some(&normal) = model.normals.get(index) {
            if Vec3::from_array(normal).dot(basis.eye-position) <= 0.0 { continue; }
        }
        if let Some((screen,_)) = project(position, &basis, rect) {
            painter.circle_filled(screen, 3.3, Color32::from_rgb(255,75,75));
        }
    }
    for landmark in landmarks {
        if let Some((p,_)) = project(landmark.position,&basis,rect) {
            painter.circle_filled(p,3.2,Color32::from_rgb(230,175,90));
            painter.text(p+egui::vec2(5.0,-5.0),egui::Align2::LEFT_BOTTOM,
                landmark.name,egui::FontId::monospace(10.0),Color32::from_gray(230));
        }
    }
    draw_model_statistics(&painter, rect, model, recipe, anatomy_target, rigging);
}

/// Adapt the disposable target triangles for the existing 3D viewport.
/// No rig, quad cage, UVs or exported mesh are created by this conversion.
fn anatomy_preview_model(surface: TargetSurface) -> Model {
    Model {
        vertices: surface.vertices,
        normals: surface.normals,
        triangles: surface.triangles,
        quads: Vec::new(),
        caps: Vec::new(),
        joints: Vec::new(),
        influences: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewKind { Anatomy, FittedCage, Rigging }

struct StudioApp {
    recipe: Recipe,
    saved: Recipe,
    recipe_path: Option<PathBuf>,
    anatomy_preview: Option<Model>,
    fitted_cage: Option<Model>,
    rigged_preview: Option<Model>,
    fit_report: Option<FitReport>,
    deformation: Option<DeformationReport>,
    defect_vertices: Vec<u32>,
    fit_strength: f32,
    anatomy_landmarks: Vec<Landmark>,
    show_landmarks: bool,
    show_poles: bool,
    show_defects: bool,
    preview_kind: PreviewKind,
    camera: Camera,
    wireframe: bool,
    show_skeleton: bool,
    pose: DiagnosticPose,
    show_advanced: bool,
    notice: Option<(String, bool)>,
}

impl StudioApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(31, 35, 41);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(43, 49, 56);
        visuals.selection.bg_fill = Color32::from_rgb(55, 107, 134);
        cc.egui_ctx.set_visuals(visuals);
        let recipe = Recipe::default();
        let initial = AnatomyTarget::from_recipe(&recipe)
            .and_then(|target| target.sample_preview().map(|surface| (target.landmarks, surface)));
        let (anatomy_preview, anatomy_landmarks, notice) = match initial {
            Ok((landmarks,surface)) =>
                (Some(anatomy_preview_model(surface)),landmarks,None),
            Err(error) => (None,Vec::new(),
                Some((format!("Anatomical target failed: {error}"),true))),
        };
        Self { saved: recipe.clone(), recipe_path: None,
            camera: Camera::for_anatomy(recipe.height), recipe,
            anatomy_preview, anatomy_landmarks,
            fitted_cage: None, rigged_preview: None, fit_report: None,
            deformation: None, defect_vertices: Vec::new(), fit_strength: 1.0,
            show_landmarks: false, show_poles: false, show_defects: true,
            preview_kind: PreviewKind::Anatomy,
            wireframe: false, show_skeleton: true, pose: DiagnosticPose::default(),
            show_advanced: false, notice }
    }

    fn dirty(&self) -> bool { self.recipe != self.saved }

    fn can_replace(&self) -> bool {
        !self.dirty() || MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description("Discard the changes to this character recipe?")
            .set_buttons(MessageButtons::YesNo)
            .show() == MessageDialogResult::Yes
    }

    fn set_recipe(&mut self, recipe: Recipe) {
        if !self.can_replace() { return; }
        self.recipe = recipe;
        self.saved = self.recipe.clone();
        self.recipe_path = None;
        self.camera = Camera::for_anatomy(self.recipe.height);
        self.pose = DiagnosticPose::default();
        self.notice = None;
        self.regenerate();
    }

    fn regenerate(&mut self) {
        match self.preview_kind {
            PreviewKind::Anatomy => {
                match AnatomyTarget::from_recipe(&self.recipe)
                    .and_then(|target| target.sample_preview()
                        .map(|surface| (target.landmarks, surface)))
                {
                    Ok((landmarks, surface)) => {
                        self.anatomy_landmarks = landmarks;
                        self.anatomy_preview = Some(anatomy_preview_model(surface));
                        self.notice = None;
                    }
                    Err(error) => {
                        self.anatomy_preview = None;
                        self.notice = Some((format!("Anatomical target failed: {error}"), true));
                    }
                }
            }
            PreviewKind::FittedCage | PreviewKind::Rigging => {
                match cage::fit(&self.recipe, self.fit_strength) {
                    Ok(fit) => {
                        self.fitted_cage = Some(fit.model);
                        self.fit_report = Some(fit.report);
                        self.anatomy_landmarks = AnatomyTarget::from_recipe(&self.recipe)
                            .map(|target| target.landmarks).unwrap_or_default();
                        self.notice = None;
                        if self.preview_kind == PreviewKind::Rigging {
                            self.update_rig_pose();
                        }
                    }
                    Err(error) => {
                        self.fitted_cage = None;
                        self.rigged_preview = None;
                        self.fit_report = None;
                        self.deformation = None;
                        self.defect_vertices.clear();
                        self.notice = Some((format!("Cage fitting failed: {error}"), true));
                    }
                }
            }
        }
    }

    // Reuse the already fitted cage on every pose-slider change; only linear
    // blend skinning and deformation diagnostics need to be recalculated.
    fn update_rig_pose(&mut self) {
        let Some(cage) = self.fitted_cage.as_ref() else {
            self.rigged_preview = None;
            self.notice = Some(("Rigging requires a fitted cage".into(), true));
            return;
        };
        match rig_preview::pose_fitted(cage, &self.recipe, self.pose) {
            Ok(result) => {
                self.rigged_preview = Some(result.model);
                self.deformation = Some(result.report);
                self.defect_vertices = result.defect_vertices;
                self.notice = None;
            }
            Err(error) => {
                self.rigged_preview = None;
                self.deformation = None;
                self.defect_vertices.clear();
                self.notice = Some((format!("Rigging preview failed: {error}"), true));
            }
        }
    }

    fn open_recipe(&mut self) {
        if !self.can_replace() { return; }
        let Some(path) = FileDialog::new().add_filter("Model recipe", &["json"]).pick_file() else { return; };
        let result = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string()).and_then(|data| Recipe::from_json(&data));
        match result {
            Ok(recipe) => {
                self.recipe = recipe;
                self.saved = self.recipe.clone();
                self.recipe_path = Some(path);
                self.camera = Camera::for_anatomy(self.recipe.height);
                self.pose = DiagnosticPose::default();
                self.notice = None;
                self.regenerate();
            }
            Err(error) => self.notice = Some((format!("Open failed: {error}"), true)),
        }
    }

    fn save_recipe(&mut self, save_as: bool) {
        let path = if save_as { None } else { self.recipe_path.clone() }
            .or_else(|| FileDialog::new().add_filter("Model recipe", &["json"])
                .set_file_name("humanoid.json").save_file());
        let Some(mut path) = path else { return; };
        if path.extension().is_none() { path.set_extension("json"); }
        let result = self.recipe.to_json().and_then(|json| write_atomic(&path, &json));
        match result {
            Ok(()) => {
                self.saved = self.recipe.clone();
                self.recipe_path = Some(path);
                self.notice = Some(("Recipe saved".into(), false));
            }
            Err(e) => self.notice = Some((format!("Save failed: {e}"), true)),
        }
    }

    fn export_build_schema(&mut self) {
        let Some(mut path) = FileDialog::new()
            .add_filter("Build schema", &["json"])
            .set_file_name("humanoid-build-schema.json")
            .save_file()
        else { return; };
        if path.extension().is_none() { path.set_extension("json"); }
        let result = build_schema_json()
            .and_then(|json| write_atomic(&path, &json));
        self.notice = Some(match result {
            Ok(()) => (format!("Build schema exported to {}", path.display()), false),
            Err(error) => (format!("Build schema export failed: {error}"), true),
        });
    }

    fn draw_models(&mut self, ui: &mut egui::Ui) {
        ui.heading("Models");
        ui.separator();
        // Model types will grow beyond humanoids; only the humanoid generator exists today.
        ui.label("Humanoid / torso + upper thighs");

        ui.add_space(16.0);
        ui.label("Model name");
        ui.text_edit_singleline(&mut self.recipe.name);

        ui.add_space(16.0);
        ui.heading("Presets");
        ui.separator();
        if ui.button("Default").clicked() { self.set_recipe(Recipe::default()); }
    }

    fn draw_build(&mut self, ui: &mut egui::Ui) {
        ui.heading("Humanoid parameters");
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.show_advanced, false, "Player");
            ui.selectable_value(&mut self.show_advanced, true, "Advanced");
            if ui.small_button("Reset all").clicked() {
                for control in BUILD_CONTROLS.iter().copied().filter(|c| c.implemented()) {
                    control.set(&mut self.recipe, control.default);
                }
                self.regenerate();
            }
        });
        ui.add_space(8.0);

        let slider_width = (ui.available_width() - 70.0).max(130.0);
        ui.spacing_mut().slider_width = slider_width;
        let show_advanced = self.show_advanced;
        let mut changed = false;
        for (section, label) in [
            (BuildSection::Proportions, "Proportions"),
            (BuildSection::Physique, "Physique"),
            (BuildSection::Neck, "Neck"),
            (BuildSection::Torso, "Torso & back"),
            (BuildSection::Chest, "Chest & pectorals"),
            (BuildSection::Shoulders, "Shoulders & deltoids"),
            (BuildSection::Arms, "Upper arms"),
            (BuildSection::Pelvis, "Pelvis"),
            (BuildSection::Legs, "Upper legs"),
        ] {
            let visible: Vec<BuildControl> = BUILD_CONTROLS.iter().copied()
                .filter(|control| control.section == section
                    && control.implemented()
                    && (show_advanced || control.player_editable))
                .collect();
            if visible.is_empty() { continue; }
            egui::CollapsingHeader::new(format!("{} ({})", label, visible.len()))
                .default_open(matches!(
                    section, BuildSection::Proportions | BuildSection::Chest
                    | BuildSection::Shoulders
                ))
                .show(ui, |ui| {
                    if ui.small_button("Reset group").clicked() {
                        for control in &visible {
                            control.set(&mut self.recipe, control.default);
                        }
                        changed = true;
                    }
                    for control in visible {
                        changed |= build_slider(ui, &mut self.recipe, control);
                    }
                });
            ui.add_space(3.0);
        }
        if changed { self.regenerate(); }
    }
}

/// Keep the editor controls aligned with the future in-game build sliders.
fn build_slider(ui: &mut egui::Ui, recipe: &mut Recipe, control: BuildControl) -> bool {
    ui.label(control.label).on_hover_text(control.description());
    let mut display_value = control.get(recipe) * control.display_scale;
    let range = control.min * control.display_scale..=control.max * control.display_scale;
    let changed = ui.add(
        egui::Slider::new(&mut display_value, range)
            .suffix(control.suffix)
            .fixed_decimals(0)
            .show_value(true)
    ).changed();
    if changed {
        control.set(recipe, display_value / control.display_scale);
    }
    ui.add_space(8.0);
    changed
}

/// Ensure an interrupted write never truncates the original recipe.
fn write_atomic(path: &Path, data: &str) -> Result<(), String> {
    let temporary = path.with_extension(format!("{}.tmp", path.extension().and_then(|ext| ext.to_str()).unwrap_or("data")));
    std::fs::write(&temporary, data).map_err(|e| e.to_string())?;
    match std::fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            Err(error.to_string())
        }
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.heading("MODEL DESIGNER");
                ui.separator();
                if ui.button("New").clicked() { self.set_recipe(Recipe::default()); }
                if ui.button("Open").clicked() { self.open_recipe(); }
                if ui.button("Save").clicked() { self.save_recipe(false); }
                if ui.button("Save As").clicked() { self.save_recipe(true); }
                ui.separator();
                if ui.button("Export Build Schema").clicked() { self.export_build_schema(); }
                if self.dirty() { ui.label("• Unsaved"); }
            });
            ui.add_space(5.0);
        });

        if let Some((message, is_error)) = &self.notice {
            egui::Panel::bottom("status").exact_size(27.0).show(ui, |ui| {
                ui.colored_label(
                    if *is_error { Color32::LIGHT_RED } else { Color32::LIGHT_GREEN },
                    message
                );
            });
        }

        egui::Panel::left("models").exact_size(210.0).show(ui, |ui| {
            ui.add_space(10.0);
            egui::ScrollArea::vertical().show(ui, |ui| self.draw_models(ui));
        });
        egui::Panel::right("build").exact_size(300.0).show(ui, |ui| {
            ui.add_space(10.0);
            egui::ScrollArea::vertical().show(ui, |ui| self.draw_build(ui));
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Preview");
                ui.separator();
                let changed = ui.selectable_value(&mut self.preview_kind, PreviewKind::Anatomy, "Anatomy target").changed()
                    | ui.selectable_value(&mut self.preview_kind, PreviewKind::FittedCage, "Auto quad cage").changed()
                    | ui.selectable_value(&mut self.preview_kind, PreviewKind::Topology, "Topology").changed();
                if changed {
                    self.camera = if self.preview_kind != PreviewKind::Topology {
                        Camera::for_anatomy(self.recipe.height)
                    } else { Camera::framed(self.recipe.height) };
                    self.regenerate();
                }
                ui.checkbox(&mut self.wireframe, "Wireframe");
                if self.preview_kind != PreviewKind::Topology {
                    ui.checkbox(&mut self.show_landmarks, "Landmarks");
                }
                if self.preview_kind == PreviewKind::Topology {
                    ui.checkbox(&mut self.show_skeleton, "Skeleton");
                }
                if self.preview_kind == PreviewKind::FittedCage {
                    ui.checkbox(&mut self.show_poles, "Poles");
                }
                if ui.button("Reset view").clicked() {
                    self.camera = if self.preview_kind != PreviewKind::Topology {
                        Camera::for_anatomy(self.recipe.height)
                    } else { Camera::framed(self.recipe.height) };
                }
            });
            if self.preview_kind == PreviewKind::Topology {
            let mut pose_changed = false;
            ui.horizontal(|ui| {
                ui.label("Left arm");
                pose_changed |= ui.add(
                    egui::Slider::new(&mut self.pose.left.raise_deg, 0.0..=70.0)
                        .text("Raise").suffix("°").fixed_decimals(0)
                ).changed();
                pose_changed |= ui.add(
                    egui::Slider::new(&mut self.pose.left.forward_deg, 0.0..=55.0)
                        .text("Forward").suffix("°").fixed_decimals(0)
                ).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Right arm");
                pose_changed |= ui.add(
                    egui::Slider::new(&mut self.pose.right.raise_deg, 0.0..=70.0)
                        .text("Raise").suffix("°").fixed_decimals(0)
                ).changed();
                pose_changed |= ui.add(
                    egui::Slider::new(&mut self.pose.right.forward_deg, 0.0..=55.0)
                        .text("Forward").suffix("°").fixed_decimals(0)
                ).changed();
                if ui.button("Reset pose").clicked() {
                    self.pose = PreviewPose::default();
                    pose_changed = true;
                }
            });
            if pose_changed { self.regenerate(); }
            } else if self.preview_kind == PreviewKind::FittedCage {
                ui.horizontal(|ui| {
                    ui.label("Surface fit");
                    if ui.add(egui::Slider::new(&mut self.fit_strength,0.0..=1.0)).changed() { self.regenerate(); }
                    if let Some(report) = &self.fit_report {
                        ui.label(format!("Vertex {:.1} → {:.1} mm | surface {:.1} mm (max {:.1}) | {} passes | moved {}",
                            report.initial_mean_error * 1000.0, report.mean_error * 1000.0,
                            report.surface_mean_error * 1000.0,
                            report.surface_maximum_error * 1000.0, report.iterations,
                            report.moved_vertices));
                    }
                });
                if let Some(report) = &self.fit_report {
                    let neutral = report.neutral_flow;
                    ui.label(format!(
                        "Neutral edge alignment {:.3} → {:.3} ({} steps) | irregular poles {} | max edge aspect {:.2} | min corner {:.0}°",
                        neutral.directional_error_before, neutral.directional_error_after,
                        neutral.accepted_passes, neutral.extraordinary_vertices,
                        neutral.maximum_quad_aspect, neutral.minimum_corner_angle_degrees
                    ));
                }
                ui.label("Anatomical landmark morph + local shrink-wrap • fixed topology • not rigged");
            } else {
                ui.label("Torso + pelvis + upper arms/thighs • anatomical surface target • not rigged");
            }
            ui.add_space(6.0);
            let selected = match self.preview_kind {
                PreviewKind::Anatomy => self.anatomy_preview.as_ref(),
                PreviewKind::FittedCage => self.fitted_cage.as_ref(),
                PreviewKind::Topology => self.model.as_ref(),
            };
            if let Some(model) = selected {
                preview(ui, model, &self.recipe, &mut self.camera, self.wireframe,
                    self.show_skeleton && self.preview_kind == PreviewKind::Topology,
                    self.preview_kind == PreviewKind::Anatomy,
                    if self.preview_kind != PreviewKind::Topology && self.show_landmarks {
                        &self.anatomy_landmarks
                    } else { &[] },
                    self.preview_kind == PreviewKind::FittedCage && self.show_poles);
            } else {
                ui.centered_and_justified(|ui| { ui.label("No valid surface to preview"); });
            }
        });
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn thin_triangles_preserve_exact_projected_vertices() {
        let points = [
            egui::pos2(100.0, 100.0),
            egui::pos2(100.0001, 100.0),
            egui::pos2(100.0, 150.0),
        ];
        let face = Face { points, depth: 2.0, colors: [Color32::GRAY; 3] };
        let mesh = triangle_mesh(&[face]);
        assert!(mesh.is_valid());
        assert_eq!(mesh.indices.as_slice(), &[0, 1, 2]);
        assert_eq!(mesh.vertices.len(), 3);
        for (actual, expected) in mesh.vertices.iter().zip(points.iter()) {
            assert_eq!(actual.pos, *expected);
            assert_eq!(actual.uv, egui::epaint::WHITE_UV);
            assert_eq!(actual.color, Color32::GRAY);
        }
    }

    #[test]
    fn triangle_mesh_preserves_per_vertex_shading() {
        let face = Face {
            points: [egui::pos2(0.0, 0.0), egui::pos2(1.0, 0.0), egui::pos2(0.0, 1.0)],
            depth: 1.0,
            colors: [Color32::RED, Color32::GREEN, Color32::BLUE],
        };
        let mesh = triangle_mesh(&[face]);
        assert_eq!(
            mesh.vertices.iter().map(|v| v.color).collect::<Vec<_>>(),
            vec![Color32::RED, Color32::GREEN, Color32::BLUE]
        );
    }

    #[test]
    fn large_triangle_batches_use_untruncated_indices() {
        let face = Face {
            points: [egui::pos2(10.0, 10.0), egui::pos2(20.0, 10.0), egui::pos2(10.0, 20.0)],
            depth: 2.0,
            colors: [Color32::WHITE; 3],
        };
        let faces = vec![face; 35_000];
        let mesh = triangle_mesh(&faces);
        assert!(mesh.is_valid());
        assert_eq!(mesh.vertices.len(), 105_000);
        assert_eq!(mesh.indices.len(), 105_000);
        assert_eq!(mesh.indices[104_999], 104_999);
    }
}
