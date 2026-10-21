//! Model Designer desktop shell and inspectable, material-free humanoid preview.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};
use glam::Vec3;
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult};
use std::path::{Path, PathBuf};
use studio_core::{Model, Recipe, generate};

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
        Self { yaw: 0.48, pitch: 0.16, distance: height * 2.5, target: Vec3::Y * (height * 0.52) }
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

struct Face {
    points: [Pos2; 3],
    depth: f32,
    color: Color32,
}

fn preview(ui: &mut egui::Ui, model: &Model, camera: &mut Camera, wireframe: bool, skeleton: bool) {
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
        let illumination = 0.36 + 0.64 * normal.dot(light).max(0.0);
        let colour = Color32::from_rgb(
            (174.0 * illumination) as u8,
            (185.0 * illumination) as u8,
            (204.0 * illumination) as u8,
        );
        faces.push(Face { points: [pa, pb, pc], depth: (da + db + dc) / 3.0, color: colour });
    }
    faces.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    for face in faces {
        let outline = if wireframe { Stroke::new(0.35, Color32::from_rgba_unmultiplied(20, 28, 38, 190)) }
                      else { Stroke::NONE };
        painter.add(egui::Shape::convex_polygon(face.points.to_vec(), face.color, outline));
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
    painter.text(rect.left_top() + egui::vec2(14.0, 12.0), egui::Align2::LEFT_TOP,
        "LMB: orbit   RMB: pan   Wheel: zoom", egui::FontId::proportional(12.0), Color32::from_gray(155));
    painter.text(rect.left_bottom() + egui::vec2(14.0, -14.0), egui::Align2::LEFT_BOTTOM,
        format!("{} vertices  |  {} triangles  |  {} joints", model.vertices.len(), model.triangles.len(), model.joints.len()),
        egui::FontId::monospace(12.0), Color32::from_gray(172));
}

struct StudioApp {
    recipe: Recipe,
    saved: Recipe,
    recipe_path: Option<PathBuf>,
    model: Option<Model>,
    camera: Camera,
    wireframe: bool,
    show_skeleton: bool,
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
        let model = generate(&recipe).ok();
        Self { saved: recipe.clone(), recipe_path: None,
            camera: Camera::framed(recipe.height), recipe, model,
            wireframe: false, show_skeleton: true, notice: None }
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
        self.camera = Camera::framed(self.recipe.height);
        self.notice = None;
        self.regenerate();
    }

    fn regenerate(&mut self) {
        match generate(&self.recipe) {
            Ok(model) => {
                self.model = Some(model);
                if self.notice.as_ref().is_some_and(|(_, error)| *error) { self.notice = None; }
            }
            Err(error) => {
                self.model = None;
                self.notice = Some((format!("Generation failed: {error}"), true));
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
                self.camera = Camera::framed(self.recipe.height);
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

    fn export(&mut self) {
        let Some(model) = self.model.as_ref() else {
            self.notice = Some(("Export failed: no validated model".into(), true)); return;
        };
        let Some(mut path) = FileDialog::new().add_filter("Wavefront OBJ", &["obj"])
            .set_file_name("humanoid.obj").save_file() else { return; };
        if path.extension().is_none() { path.set_extension("obj"); }
        let companion = path.with_extension("rig.json");
        let result = model.to_rig_json().and_then(|rig| {
            write_atomic(&path, &model.to_obj())?;
            write_atomic(&companion, &rig)
        });
        self.notice = Some(match result {
            Ok(()) => (format!("Exported {} and {}", path.display(), companion.display()), false),
            Err(e) => (format!("Export failed: {e}"), true),
        });
    }

    fn draw_library(&mut self, ui: &mut egui::Ui) {
        ui.heading("Body presets");
        ui.label("Each preset uses the same canonical surface and 19-joint skeleton.");
        ui.add_space(8.0);
        if ui.button("Neutral humanoid").clicked() { self.set_recipe(Recipe::default()); }
        if ui.button("Tall / broad").clicked() {
            self.set_recipe(Recipe { name: "Tall / Broad".into(), height: 1.99,
                shoulder_width: 0.55, musculature: 0.76, body_mass: 0.55, ..Recipe::default() });
        }
        if ui.button("Short / stocky").clicked() {
            self.set_recipe(Recipe { name: "Short / Stocky".into(), height: 1.58,
                shoulder_width: 0.56, leg_length: 0.88, musculature: 0.62,
                body_mass: 0.87, ..Recipe::default() });
        }
        if ui.button("Lean / long-limbed").clicked() {
            self.set_recipe(Recipe { name: "Lean / Long-limbed".into(), height: 1.89,
                arm_length: 1.15, leg_length: 1.13, musculature: 0.23,
                body_mass: 0.17, ..Recipe::default() });
        }
        ui.add_space(18.0);
        ui.separator();
        ui.label("Milestone 1");
        ui.small("Reference topology • editable morphology • skeletal metadata");
        ui.add_space(8.0);
        ui.small("Skin weights are experimental. The preview does not animate the character.");
    }

    fn draw_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Humanoid parameters");
        ui.separator();
        ui.label("Name");
        let mut changed = ui.text_edit_singleline(&mut self.recipe.name).changed();
        ui.add_space(10.0);
        ui.label("Overall proportions");
        changed |= slider(ui, "Height (m)", &mut self.recipe.height, 1.50..=2.10);
        changed |= slider(ui, "Shoulders (m)", &mut self.recipe.shoulder_width, 0.34..=0.62);
        changed |= slider(ui, "Arm length", &mut self.recipe.arm_length, 0.82..=1.18);
        changed |= slider(ui, "Leg length", &mut self.recipe.leg_length, 0.82..=1.18);
        ui.add_space(10.0);
        ui.label("Body shape");
        changed |= slider(ui, "Musculature", &mut self.recipe.musculature, 0.0..=1.0);
        changed |= slider(ui, "Body mass", &mut self.recipe.body_mass, 0.0..=1.0);
        if changed { self.regenerate(); }
        ui.add_space(14.0);
        ui.separator();
        ui.label("Viewport overlays");
        ui.checkbox(&mut self.wireframe, "Wireframe");
        ui.checkbox(&mut self.show_skeleton, "Skeleton");
        if ui.button("Reset camera").clicked() { self.camera = Camera::framed(self.recipe.height); }
        ui.add_space(10.0);
        ui.separator();
        ui.small("Topology stays fixed across all parameter combinations. Surface edge flow and animation deformation are not yet production-validated.");
    }
}

fn slider(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    ui.add(egui::Slider::new(value, range).text(label).fixed_decimals(2)).changed()
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.heading("MODEL DESIGNER");
                ui.separator();
                if ui.button("New").clicked() { self.set_recipe(Recipe::default()); }
                if ui.button("Open").clicked() { self.open_recipe(); }
                if ui.button("Save").clicked() { self.save_recipe(false); }
                if ui.button("Save As").clicked() { self.save_recipe(true); }
                ui.separator();
                if ui.button("Export OBJ + Rig").clicked() { self.export(); }
                if self.dirty() { ui.label("• Unsaved changes"); }
            });
            ui.add_space(5.0);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((message, is_error)) = &self.notice {
                    ui.colored_label(if *is_error { Color32::LIGHT_RED } else { Color32::LIGHT_GREEN }, message);
                } else { ui.small("Y-up • Metres • Canonical topology v1 • No materials"); }
            });
        });
        egui::SidePanel::left("presets").resizable(true).default_width(212.0)
            .min_width(190.0).show(ctx, |ui| { ui.add_space(10.0); self.draw_library(ui); });
        egui::SidePanel::right("inspector").resizable(true).default_width(310.0)
            .min_width(265.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| { ui.add_space(10.0); self.draw_inspector(ui); });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(model) = self.model.as_ref() {
                preview(ui, model, &mut self.camera, self.wireframe, self.show_skeleton);
            } else { ui.centered_and_justified(|ui| { ui.label("No valid mesh to preview"); }); }
        });
    }
}