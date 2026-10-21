//! Model Designer desktop shell and inspectable, material-free humanoid preview.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};
use glam::Vec3;
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult};
use std::path::{Path, PathBuf};
use studio_core::{BuildControl, BuildSection, BUILD_CONTROLS, Model, Recipe, generate};
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
fn draw_model_statistics(painter: &egui::Painter, viewport: Rect, model: &Model, recipe: &Recipe) {
    let panel = Rect::from_min_size(
        viewport.right_top() + egui::vec2(-194.0, 12.0),
        egui::vec2(182.0, 145.0),
    );
    painter.rect_filled(panel, 5.0, Color32::from_rgba_unmultiplied(28, 32, 38, 228));
    let entries = [
        ("MODEL", String::new()),
        ("Type", "Humanoid".to_owned()),
        ("Height", format!("{:.2} m", recipe.height)),
        ("Vertices", model.vertices.len().to_string()),
        ("Triangles", model.triangles.len().to_string()),
        ("Joints", model.joints.len().to_string()),
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
    camera: &mut Camera, wireframe: bool, skeleton: bool,
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
        let stroke = Stroke::new(0.35, Color32::from_rgb(20, 28, 38));
        for face in &faces {
            for edge in 0..3 {
                painter.line_segment(
                    [face.points[edge], face.points[(edge + 1) % 3]],
                    stroke,
                );
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
    draw_model_statistics(&painter, rect, model, recipe);
}

struct StudioApp {
    recipe: Recipe,
    saved: Recipe,
    recipe_path: Option<PathBuf>,
    model: Option<Model>,
    camera: Camera,
    wireframe: bool,
    show_skeleton: bool,
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
        let model = generate(&recipe).ok();
        Self { saved: recipe.clone(), recipe_path: None,
            camera: Camera::framed(recipe.height), recipe, model,
            wireframe: false, show_skeleton: false, show_advanced: false, notice: None }
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
                self.notice = None;
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
        ui.selectable_label(true, "Humanoid");

        ui.add_space(16.0);
        ui.label("Model name");
        ui.text_edit_singleline(&mut self.recipe.name);

        ui.add_space(16.0);
        ui.heading("Presets");
        ui.separator();
        if ui.button("Default").clicked() { self.set_recipe(Recipe::default()); }
    }

    fn draw_build(&mut self, ui: &mut egui::Ui) {
        ui.heading("Build");
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.show_advanced, false, "Player");
            ui.selectable_value(&mut self.show_advanced, true, "Advanced");
        });
        ui.add_space(12.0);

        let slider_width = (ui.available_width() - 70.0).max(130.0);
        ui.spacing_mut().slider_width = slider_width;
        let show_advanced = self.show_advanced;
        let mut changed = false;
        for (section, label) in [
            (BuildSection::Proportions, "Proportions"),
            (BuildSection::Physique, "Physique"),
            (BuildSection::Torso, "Torso"),
            (BuildSection::Arms, "Arms & hands"),
            (BuildSection::Legs, "Pelvis & legs"),
            (BuildSection::HeadNeck, "Head & neck"),
        ] {
            if !BUILD_CONTROLS.iter().any(|control| {
                control.section == section && (show_advanced || control.player_editable)
            }) {
                continue;
            }
            egui::CollapsingHeader::new(label)
                .default_open(matches!(section, BuildSection::Proportions | BuildSection::Physique))
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    for control in BUILD_CONTROLS.iter().copied().filter(|control| {
                        control.section == section && (show_advanced || control.player_editable)
                    }) {
                        changed |= build_slider(ui, &mut self.recipe, control);
                    }
                });
            ui.add_space(4.0);
        }
        if changed { self.regenerate(); }
    }
}

/// Keep the editor controls aligned with the future in-game build sliders.
fn build_slider(ui: &mut egui::Ui, recipe: &mut Recipe, control: BuildControl) -> bool {
    ui.label(control.label);
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
                if ui.button("Export OBJ + Rig").clicked() { self.export(); }
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
                ui.checkbox(&mut self.wireframe, "Wireframe");
                ui.checkbox(&mut self.show_skeleton, "Skeleton");
                if ui.button("Reset view").clicked() {
                    self.camera = Camera::framed(self.recipe.height);
                }
            });
            ui.add_space(6.0);
            if let Some(model) = self.model.as_ref() {
                preview(ui, model, &self.recipe, &mut self.camera, self.wireframe, self.show_skeleton);
            } else {
                ui.centered_and_justified(|ui| { ui.label("No valid mesh to preview"); });
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
