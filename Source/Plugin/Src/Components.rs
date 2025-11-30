#![allow(non_snake_case)]

use nih_plug_egui::egui::{
    Color32, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, StrokeKind, Align2, Shape, Pos2, emath::Rot2,
};
use std::f32::consts::PI;
use crate::scale::ScaleContext;

// --- Colors from React Design ---
pub const COLOR_BG_APP: Color32 = Color32::from_rgb(229, 231, 235); // Slate-200ish
pub const COLOR_BG_MAIN: Color32 = Color32::WHITE;
pub const COLOR_BG_SIDEBAR: Color32 = Color32::from_rgb(248, 250, 252); // Slate-50
pub const COLOR_BORDER_LIGHT: Color32 = Color32::from_rgb(226, 232, 240); // Slate-200
pub const COLOR_BORDER_MEDIUM: Color32 = Color32::from_rgb(203, 213, 225); // Slate-300
pub const COLOR_BORDER_DARK: Color32 = Color32::from_rgb(100, 116, 139); // Slate-500
pub const COLOR_TEXT_DARK: Color32 = Color32::from_rgb(15, 23, 42); // Slate-900
pub const COLOR_TEXT_MEDIUM: Color32 = Color32::from_rgb(71, 85, 105); // Slate-600
pub const COLOR_TEXT_LIGHT: Color32 = Color32::from_rgb(148, 163, 184); // Slate-400

pub const COLOR_ACTIVE_RED_BG: Color32 = Color32::from_rgb(220, 38, 38); // Red-600
pub const COLOR_ACTIVE_YELLOW_BG: Color32 = Color32::from_rgb(253, 224, 71); // Yellow-300
pub const COLOR_ACTIVE_SLATE_BG: Color32 = Color32::from_rgb(30, 41, 59); // Slate-800

// --- Helper: Draw Arc ---
fn shape_arc(center: Pos2, radius: f32, start_angle: f32, end_angle: f32, stroke: Stroke) -> Shape {
    let points: Vec<Pos2> = (0..=30) // 30 segments for smoothness
        .map(|i| {
            let t = i as f32 / 30.0;
            let angle = start_angle + (end_angle - start_angle) * t;
            center + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect();
    Shape::line(points, stroke)
}

// --- 1. Brutalist Button ---
pub struct BrutalistButton<'a> {
    label: &'a str,
    active: bool,
    danger: bool,
    width_mode: ButtonWidth, // <-- UPDATED: Replaced bool with an enum
    height: f32,
    scale: &'a ScaleContext,
}

// --- ADDED: Enum to control width logic ---
enum ButtonWidth {
    Full,          // Takes up all available width
    Fixed(f32),    // A specific, scaled width
    Default,       // The original default (80px scaled)
}

impl<'a> BrutalistButton<'a> {
    pub fn new(label: &'a str, scale: &'a ScaleContext) -> Self {
        Self {
            label,
            active: false,
            danger: false,
            width_mode: ButtonWidth::Default, // <-- Default behavior
            height: scale.s(40.0),
            scale,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    // --- UPDATED ---
    pub fn full_width(mut self, full: bool) -> Self {
        if full {
            self.width_mode = ButtonWidth::Full;
        }
        self
    }
    
    // --- ADDED: Method to set a specific width ---
    pub fn width(mut self, width_px: f32) -> Self {
        self.width_mode = ButtonWidth::Fixed(width_px);
        self
    }

    pub fn large(mut self) -> Self {
        self.height = self.scale.s(56.0);
        self
    }
}

impl<'a> Widget for BrutalistButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let s = self.scale;
        
        // --- UPDATED: Size calculation logic based on the enum ---
        let size = match self.width_mode {
            ButtonWidth::Full => Vec2::new(ui.available_width(), self.height),
            ButtonWidth::Fixed(w) => Vec2::new(w, self.height),
            ButtonWidth::Default => Vec2::new(s.s(80.0), self.height),
        };

        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter();

        let is_hovered = response.hovered();
        let is_active = self.active;
        let is_clicking = response.is_pointer_button_down_on();

        let (bg_color, text_color, border_color) = if is_active {
            if self.danger {
                (COLOR_ACTIVE_RED_BG, Color32::WHITE, Color32::from_rgb(185, 28, 28))
            } else {
                (COLOR_ACTIVE_YELLOW_BG, COLOR_TEXT_DARK, Color32::from_rgb(100, 116, 139))
            }
        } else if is_hovered {
            (COLOR_BG_SIDEBAR, COLOR_TEXT_DARK, COLOR_BORDER_DARK)
        } else {
            (Color32::WHITE, COLOR_TEXT_MEDIUM, COLOR_BORDER_MEDIUM)
        };

        let offset = if is_clicking { s.vec2(0.0, 1.0) } else { Vec2::ZERO };
        let draw_rect = rect.translate(offset);

        // Shadow effect (Hard shadow)
        if !is_active && !is_clicking {
            painter.rect_filled(
                rect.translate(s.vec2(1.0, 1.0)),
                0.0,
                Color32::from_black_alpha(20)
            );
        }

        painter.rect_filled(draw_rect, 0.0, bg_color);
        painter.rect_stroke(draw_rect, 0.0, Stroke::new(s.s(1.0), border_color), StrokeKind::Inside);

        painter.text(
            draw_rect.center(),
            Align2::CENTER_CENTER,
            self.label,
            if self.height > s.s(45.0) { s.font(14.0) } else { s.font(12.0) },
            text_color,
        );

        response
    }
}

// --- 2. Tech Volume Knob (Re-designed) ---
pub struct TechVolumeKnob<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
    scale: &'a ScaleContext,
}

impl<'a> TechVolumeKnob<'a> {
    pub fn new(value: &'a mut f32, scale: &'a ScaleContext) -> Self {
        // We'll keep the internal value as dB, but display as percentage
        Self { value, min: -60.0, max: 12.0, scale } 
    }
}

impl<'a> Widget for TechVolumeKnob<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let s = self.scale;
        let desired_size = Vec2::splat(s.s(96.0)); // 24rem -> 96px base
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::drag());

        if response.dragged() {
            let drag_delta = response.drag_delta().y;
            let range = self.max - self.min;
            // Sensitivity: Full height drag = full range change
            let delta_val = (drag_delta / 200.0) * range;
            *self.value = (*self.value - delta_val).clamp(self.min, self.max);
            response.mark_changed();
        }

        let painter = ui.painter();
        let center = rect.center();
        let radius = (rect.width() / 2.0) - s.s(4.0); // Padding

        // Angles: -135deg to +135deg (in radians)
        // -135 deg = -2.356 rad
        // +135 deg = +2.356 rad
        let start_angle = -135.0f32.to_radians(); 
        let end_angle = 135.0f32.to_radians(); 
        
        // Background Track (Thin, dark)
        // We draw the full 270 degree arc
        painter.add(shape_arc(
            center, 
            radius, 
            start_angle - PI / 2.0, // egui 0 is X axis, we want 0 to be UP? No, egui Y is down.
            // Let's use standard unit circle: 0 is Right (3 o'clock). 
            // -135 is bottom-left. +135 is bottom-right.
            // Wait, usually knobs are: 
            // Min: Bottom-Left (approx 135 deg in normal math, or 225 deg)
            // Max: Bottom-Right (approx 45 deg or 315 deg)
            // In egui:
            // 0 = Right, PI/2 = Bottom, PI = Left, 3PI/2 = Top
            // Let's map Min to 3/4 PI + something
            
            // Standard knob: 7 o'clock to 5 o'clock
            // 7 o'clock = 135 deg from Bottom (90) = 225 deg total?
            // Egui coordinates:
            // 0 is Right. 
            // Min (-135 deg from Top):
            // Top is -PI/2. 
            // Min = -PI/2 - 135deg = -90 - 135 = -225 deg?
            
            // Let's stick to the React logic: rotate(135deg)
            // React: 0 value = -135deg. Max value = +135deg. 0deg is TOP (12 o'clock).
            // Egui: 0 rad is Right (3 o'clock).
            // So Top is -PI/2.
            // Min = -PI/2 - (135 * PI/180)
            // Max = -PI/2 + (135 * PI/180)
            
            end_angle - PI / 2.0,
            Stroke::new(s.s(4.0), COLOR_BORDER_LIGHT)
        ));

        let min_angle_rad = -PI / 2.0 - (135.0f32.to_radians());
        let max_angle_rad = -PI / 2.0 + (135.0f32.to_radians());
        
        let t = (*self.value - self.min) / (self.max - self.min);
        let current_angle_rad = min_angle_rad + (max_angle_rad - min_angle_rad) * t;

        // Draw Background Arc (Full Range)
        painter.add(shape_arc(center, radius, min_angle_rad, max_angle_rad, Stroke::new(s.s(4.0), COLOR_BORDER_LIGHT)));

        // Draw Active Arc
        let active_color = if t > 0.9 { COLOR_ACTIVE_RED_BG } else { COLOR_TEXT_DARK };
        painter.add(shape_arc(center, radius, min_angle_rad, current_angle_rad, Stroke::new(s.s(4.0), active_color)));

        // Draw Knob Handle (The big rotating box)
        let knob_size = s.s(64.0);
        let knob_rect = Rect::from_center_size(center, Vec2::splat(knob_size));
        
        // Rotate the knob rect
        // We can't rotate a rect easily, but we can draw a rotated shape or use transform
        // Egui doesn't have a simple "draw rotated rect" without Mesh.
        // Let's verify if we can rotate the painter transform?
        
        // Easier: Calculate corners manually
        let rotation = Rot2::from_angle(current_angle_rad + PI / 2.0); // Adjust because 0 angle for rect is usually Up?
        
        // Actually, let's just draw the square unrotated, but with an indicator line that rotates?
        // React design: The whole square rotates.
        // <div style={{ transform: `rotate(${angle}deg)` }}>
        
        let half_size = knob_size / 2.0;
        let corners = [
            Vec2::new(-half_size, -half_size),
            Vec2::new(half_size, -half_size),
            Vec2::new(half_size, half_size),
            Vec2::new(-half_size, half_size),
        ];
        
        let rotated_corners: Vec<Pos2> = corners.iter().map(|p| {
            center + rotation * *p
        }).collect();
        
        // Draw the Knob Body (White Box)
        // Shadow first
        let shadow_offset = s.vec2(0.0, 2.0);
        let shadow_corners: Vec<Pos2> = rotated_corners.iter().map(|p| *p + shadow_offset).collect();
        painter.add(Shape::convex_polygon(shadow_corners, Color32::from_black_alpha(20), Stroke::NONE));

        // Main Box
        painter.add(Shape::convex_polygon(rotated_corners.clone(), Color32::WHITE, Stroke::new(s.s(2.0), COLOR_BORDER_DARK)));

        // Indicator Line (Black strip at top)
        // Relative to center, Top is (0, -half_size)
        // Line from (0, -half_size + padding) to (0, -half_size + length)
        let indicator_top = Vec2::new(0.0, -half_size + s.s(2.0));
        let indicator_bottom = Vec2::new(0.0, -half_size + s.s(20.0));

        let p1 = center + rotation * indicator_top;
        let p2 = center + rotation * indicator_bottom;

        painter.line_segment([p1, p2], Stroke::new(s.s(4.0), COLOR_TEXT_DARK));

        // --- FIX: Restore the percentage display ---
        let t = (*self.value - self.min) / (self.max - self.min);
        let percentage = t * 100.0;

        // The text is drawn on top of the rhombus, so it's placed here at the end.
        painter.text(
            center,
            Align2::CENTER_CENTER,
            format!("{:.0}%", percentage),
            s.mono_font(12.0),
            COLOR_TEXT_DARK
        );
        
        response
    }
}

// --- 3. Speaker Box ---
pub struct SpeakerBox<'a> {
    name: &'a str,
    active: bool,
    is_sub: bool,
    scale: &'a ScaleContext,
    label: Option<&'a str>, // For "CH 7", "AUX" labels below
}

impl<'a> SpeakerBox<'a> {
    pub fn new(name: &'a str, active: bool, scale: &'a ScaleContext) -> Self {
        Self {
            name,
            active,
            is_sub: name.contains("SUB") || name == "LFE",
            scale,
            label: None,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl<'a> Widget for SpeakerBox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let s = self.scale;
        let box_size = if self.is_sub { s.vec2(80.0, 80.0) } else { s.vec2(96.0, 96.0) };

        // Container for Box + Label
        let (rect, response) = ui.allocate_exact_size(
            if self.label.is_some() { box_size + Vec2::new(0.0, s.s(20.0)) } else { box_size },
            Sense::click()
        );

        let box_rect = Rect::from_min_size(rect.min, box_size);
        let painter = ui.painter();

        let is_hovered = response.hovered();

        let (bg_color, text_color, border_color) = if self.active {
            (COLOR_ACTIVE_SLATE_BG, Color32::WHITE, COLOR_ACTIVE_SLATE_BG)
        } else if is_hovered {
            (Color32::WHITE, COLOR_TEXT_MEDIUM, COLOR_TEXT_MEDIUM)
        } else {
            (Color32::WHITE, COLOR_BORDER_LIGHT, COLOR_BORDER_MEDIUM)
        };

        // 1. Box Background
        painter.rect_filled(box_rect, 0.0, bg_color);
        painter.rect_stroke(box_rect, 0.0, Stroke::new(s.s(1.0), border_color), StrokeKind::Inside);

        // 2. Corner Accents (Tech Feel)
        let corner_len = s.s(4.0);
        let corner_color = if self.active { Color32::WHITE } else { COLOR_BORDER_MEDIUM };

        // TL
        painter.rect_filled(Rect::from_min_size(box_rect.min, Vec2::splat(corner_len)), 0.0, corner_color);
        // TR
        painter.rect_filled(Rect::from_min_size(box_rect.right_top() - Vec2::new(corner_len, 0.0), Vec2::splat(corner_len)), 0.0, corner_color);
        // BL
        painter.rect_filled(Rect::from_min_size(box_rect.left_bottom() - Vec2::new(0.0, corner_len), Vec2::splat(corner_len)), 0.0, corner_color);
        // BR
        painter.rect_filled(Rect::from_min_size(box_rect.right_bottom() - Vec2::splat(corner_len), Vec2::splat(corner_len)), 0.0, corner_color);

        // 3. Text
        painter.text(
            box_rect.center(),
            Align2::CENTER_CENTER,
            self.name,
            s.mono_font(14.0),
            text_color,
        );

        // 4. External Label (if any)
        if let Some(label_text) = self.label {
            let label_rect = Rect::from_min_size(
                box_rect.left_bottom() + Vec2::new(0.0, s.s(4.0)),
                Vec2::new(box_size.x, s.s(14.0))
            );
            // Label background tag
            let tag_width = s.s(30.0);
            let tag_rect = Rect::from_center_size(label_rect.center(), s.vec2(tag_width, 12.0));
            painter.rect_filled(tag_rect, 0.0, Color32::WHITE);

            painter.text(
                tag_rect.center(),
                Align2::CENTER_CENTER,
                label_text,
                s.font(9.0),
                COLOR_TEXT_LIGHT,
            );
        }

        response
    }
}
