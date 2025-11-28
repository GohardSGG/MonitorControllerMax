#![allow(non_snake_case)]

use nih_plug_egui::egui::{
    self, Color32, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, StrokeKind,
};
// 移除 eframe，直接使用 egui 的 emath
use nih_plug_egui::egui::emath::Rot2;

// --- Color Palette (Based on React Reference) ---
pub const COLOR_BG_APP: Color32 = Color32::from_rgb(229, 231, 235); // #e5e7eb (gray-200)
pub const COLOR_BG_MAIN: Color32 = Color32::WHITE;
pub const COLOR_BG_SIDEBAR: Color32 = Color32::from_rgb(248, 250, 252); // #f8fafc (slate-50)
pub const COLOR_BORDER_LIGHT: Color32 = Color32::from_rgb(226, 232, 240); // #e2e8f0 (slate-200)
pub const COLOR_BORDER_MEDIUM: Color32 = Color32::from_rgb(203, 213, 225); // #cbd5e1 (slate-300)
pub const COLOR_BORDER_DARK: Color32 = Color32::from_rgb(148, 163, 184); // #94a3b8 (slate-400)
pub const COLOR_TEXT_DARK: Color32 = Color32::from_rgb(15, 23, 42); // #0f172a (slate-900)
pub const COLOR_TEXT_MEDIUM: Color32 = Color32::from_rgb(71, 85, 105); // #475569 (slate-600)
pub const COLOR_TEXT_LIGHT: Color32 = Color32::from_rgb(148, 163, 184); // #94a3b8 (slate-400)

// Active Colors
pub const COLOR_ACTIVE_RED_BG: Color32 = Color32::from_rgb(220, 38, 38); // #dc2626 (red-600)
pub const COLOR_ACTIVE_YELLOW_BG: Color32 = Color32::from_rgb(253, 224, 71); // #fde047 (yellow-300)
pub const COLOR_ACTIVE_SLATE_BG: Color32 = Color32::from_rgb(30, 41, 59); // #1e293b (slate-800)

// --- 1. Brutalist Button ---

pub struct BrutalistButton<'a> {
    label: &'a str,
    active: bool,
    danger: bool,
    full_width: bool,
    height: f32,
}

impl<'a> BrutalistButton<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            active: false,
            danger: false,
            full_width: false,
            height: 40.0, // Default size "md"
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

    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }
    
    pub fn large(mut self) -> Self {
        self.height = 56.0; // size "lg" -> h-14 (14 * 4 = 56px)
        self
    }
}

impl<'a> Widget for BrutalistButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = if self.full_width {
            Vec2::new(ui.available_width(), self.height)
        } else {
            Vec2::new(80.0, self.height) // w-20
        };

        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter();

        // Visual State Logic
        let is_hovered = response.hovered();
        let is_active = self.active;
        let is_clicking = response.is_pointer_button_down_on();

        // Determine Colors
        let (bg_color, text_color, border_color) = if is_active {
            if self.danger {
                (COLOR_ACTIVE_RED_BG, Color32::WHITE, Color32::from_rgb(185, 28, 28)) // red-700
            } else {
                (COLOR_ACTIVE_YELLOW_BG, COLOR_TEXT_DARK, Color32::from_rgb(100, 116, 139)) // slate-500
            }
        } else if is_hovered {
            (Color32::from_rgb(248, 250, 252), COLOR_TEXT_DARK, COLOR_BORDER_DARK) // slate-50 hover
        } else {
            (Color32::WHITE, COLOR_TEXT_MEDIUM, COLOR_BORDER_DARK)
        };

        // Draw Logic
        // Brutalist style: No rounded corners, clear borders
        // Active/Click effect: translate 1px down
        
        let offset = if is_clicking { Vec2::new(0.0, 1.0) } else { Vec2::ZERO };
        let draw_rect = rect.translate(offset);

        // 1. Shadow (only for inactive state to give depth)
        if !is_active && !is_clicking {
            painter.rect_filled(
                rect.translate(Vec2::new(1.0, 1.0)), 
                0.0, 
                Color32::from_black_alpha(25) // 10% opacity shadow
            );
        }
        
        // 2. Background
        painter.rect_filled(draw_rect, 0.0, bg_color);
        
        // 3. Border
        // egui 0.31+ rect_stroke 需要 StrokeKind (Inside/Middle/Outside)
        painter.rect_stroke(draw_rect, 0.0, Stroke::new(1.0, border_color), StrokeKind::Inside);

        // 4. Inner Shadow for Active State
        if is_active {
             // Simulate inset shadow by drawing lines
             // Top inner
             painter.line_segment(
                [draw_rect.left_top() + Vec2::new(0.0, 1.0), draw_rect.right_top() + Vec2::new(0.0, 1.0)],
                Stroke::new(1.0, Color32::from_black_alpha(30))
             );
        }

        // 5. Text
        painter.text(
            draw_rect.center(),
            egui::Align2::CENTER_CENTER,
            self.label,
            if self.height > 45.0 { egui::FontId::proportional(14.0) } else { egui::FontId::proportional(12.0) },
            text_color,
        );

        response
    }
}

// --- 2. Tech Volume Knob ---

pub struct TechVolumeKnob<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
}

impl<'a> TechVolumeKnob<'a> {
    pub fn new(value: &'a mut f32) -> Self {
        Self { value, min: 0.0, max: 1.0 }
    }
}

impl<'a> Widget for TechVolumeKnob<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(96.0, 96.0); // w-24 h-24
        // CRITICAL FIX: make response mutable
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::drag());

        if response.dragged() {
            let delta = response.drag_delta().y;
            let sensitivity = 0.005;
            *self.value = (*self.value - delta * sensitivity).clamp(self.min, self.max);
            response.mark_changed();
        }

        let painter = ui.painter();
        let center = rect.center();
        let radius = 38.0;
        
        // Angle calculation: 0% = -135deg, 100% = +135deg (Total 270deg)
        let start_angle = -135.0_f32.to_radians();
        let total_angle = 270.0_f32.to_radians();
        let current_angle = start_angle + (*self.value) * total_angle;

        // 1. Background Track (Thin, dark)
        painter.circle_stroke(center, radius, Stroke::new(4.0, COLOR_BORDER_LIGHT));

        // 2. Knob Handle (Square rotated)
        let knob_size = 64.0; // w-16
        let knob_rect = Rect::from_center_size(center, Vec2::splat(knob_size));
        
        // Rotate the square container
        let mut square_points = vec![
            knob_rect.left_top(),
            knob_rect.right_top(),
            knob_rect.right_bottom(),
            knob_rect.left_bottom(),
        ];
        
        // Rotate points
        let rot = Rot2::from_angle(current_angle);
        for p in &mut square_points {
            *p = center + rot * (*p - center);
        }

        // Draw Knob Body
        painter.add(egui::Shape::convex_polygon(
            square_points.clone(), 
            Color32::WHITE, 
            Stroke::new(2.0, if response.hovered() { COLOR_TEXT_MEDIUM } else { COLOR_BORDER_DARK })
        ));

        // Draw Indicator Line
        let indicator_top = center + rot * Vec2::new(0.0, -knob_size/2.0 + 4.0);
        let indicator_bottom = center + rot * Vec2::new(0.0, -knob_size/2.0 + 20.0);
        
        painter.line_segment([indicator_top, indicator_bottom], Stroke::new(4.0, COLOR_ACTIVE_SLATE_BG));

        // 3. Digital Display
        let text_rect = Rect::from_center_size(
            center + Vec2::new(0.0, 60.0), 
            Vec2::new(64.0, 20.0)
        );
        painter.rect_filled(text_rect, 0.0, Color32::WHITE);
        painter.rect_stroke(text_rect, 0.0, Stroke::new(1.0, COLOR_BORDER_DARK), StrokeKind::Inside);
        
        let percentage = (*self.value * 100.0).round() as i32;
        painter.text(
            text_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}%", percentage),
            egui::FontId::monospace(12.0),
            COLOR_TEXT_DARK
        );
        
        // Little shadow for box
        painter.rect_stroke(text_rect.translate(Vec2::new(1.0, 1.0)), 0.0, Stroke::new(1.0, Color32::from_black_alpha(25)), StrokeKind::Inside);

        response
    }
}

// --- 3. Speaker Box ---

pub struct SpeakerBox<'a> {
    name: &'a str,
    active: bool,
    is_sub: bool,
}

impl<'a> SpeakerBox<'a> {
    pub fn new(name: &'a str, active: bool) -> Self {
        Self { 
            name, 
            active,
            is_sub: name.contains("SUB") || name == "LFE",
        }
    }
}

impl<'a> Widget for SpeakerBox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = if self.is_sub { Vec2::new(80.0, 80.0) } else { Vec2::new(96.0, 96.0) };
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter();

        let is_hovered = response.hovered();
        let border_color = if is_hovered { COLOR_TEXT_MEDIUM } else { COLOR_BORDER_MEDIUM };
        
        let (bg_color, text_color, accent_color) = if self.active {
            (COLOR_ACTIVE_SLATE_BG, Color32::WHITE, Color32::WHITE)
        } else {
            (Color32::WHITE, COLOR_BORDER_DARK, COLOR_BORDER_MEDIUM)
        };

        // 1. Background & Border
        painter.rect_filled(rect, 0.0, bg_color);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, if self.active { COLOR_ACTIVE_SLATE_BG } else { border_color }), StrokeKind::Inside);

        if self.active {
             // Active Shadow
             painter.rect_filled(
                rect.translate(Vec2::new(2.0, 2.0)), 
                0.0, 
                Color32::from_black_alpha(50)
            );
             // Redraw box on top to cover shadow
             painter.rect_filled(rect, 0.0, bg_color);
             painter.rect_stroke(rect, 0.0, Stroke::new(1.0, COLOR_TEXT_DARK), StrokeKind::Inside);
        } else if is_hovered {
             // Hover Shadow
             painter.rect_filled(
                rect.translate(Vec2::new(2.0, 2.0)), 
                0.0, 
                Color32::from_black_alpha(25)
            );
             painter.rect_filled(rect, 0.0, bg_color);
             painter.rect_stroke(rect, 0.0, Stroke::new(1.0, COLOR_TEXT_MEDIUM), StrokeKind::Inside);
        }

        // 2. Corner Accents (The 4 little squares)
        let accent_size = 4.0;
        // Top-Left
        painter.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(accent_size)), 0.0, accent_color);
        // Top-Right
        painter.rect_filled(Rect::from_min_size(rect.right_top() - Vec2::new(accent_size, 0.0), Vec2::splat(accent_size)), 0.0, accent_color);
        // Bottom-Left
        painter.rect_filled(Rect::from_min_size(rect.left_bottom() - Vec2::new(0.0, accent_size), Vec2::splat(accent_size)), 0.0, accent_color);
        // Bottom-Right
        painter.rect_filled(Rect::from_min_size(rect.right_bottom() - Vec2::splat(accent_size), Vec2::splat(accent_size)), 0.0, accent_color);

        // 3. Text
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.name,
            egui::FontId::monospace(14.0),
            text_color,
        );

        response
    }
}
