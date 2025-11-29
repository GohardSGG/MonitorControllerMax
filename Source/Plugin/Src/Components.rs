#![allow(non_snake_case)]

use nih_plug_egui::egui::{
    self, Color32, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, StrokeKind,
};
use nih_plug_egui::egui::emath::Rot2;

pub const COLOR_BG_APP: Color32 = Color32::from_rgb(229, 231, 235); 
pub const COLOR_BG_MAIN: Color32 = Color32::WHITE;
pub const COLOR_BG_SIDEBAR: Color32 = Color32::from_rgb(248, 250, 252); 
pub const COLOR_BORDER_LIGHT: Color32 = Color32::from_rgb(226, 232, 240); 
pub const COLOR_BORDER_MEDIUM: Color32 = Color32::from_rgb(203, 213, 225); 
pub const COLOR_BORDER_DARK: Color32 = Color32::from_rgb(148, 163, 184); 
pub const COLOR_TEXT_DARK: Color32 = Color32::from_rgb(15, 23, 42); 
pub const COLOR_TEXT_MEDIUM: Color32 = Color32::from_rgb(71, 85, 105); 
pub const COLOR_TEXT_LIGHT: Color32 = Color32::from_rgb(148, 163, 184); 

pub const COLOR_ACTIVE_RED_BG: Color32 = Color32::from_rgb(220, 38, 38); 
pub const COLOR_ACTIVE_YELLOW_BG: Color32 = Color32::from_rgb(253, 224, 71); 
pub const COLOR_ACTIVE_SLATE_BG: Color32 = Color32::from_rgb(30, 41, 59); 

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
            height: 40.0,
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
        self.height = 56.0;
        self
    }
}

impl<'a> Widget for BrutalistButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = if self.full_width {
            Vec2::new(ui.available_width(), self.height)
        } else {
            Vec2::new(80.0, self.height)
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
            (Color32::from_rgb(248, 250, 252), COLOR_TEXT_DARK, COLOR_BORDER_DARK)
        } else {
            (Color32::WHITE, COLOR_TEXT_MEDIUM, COLOR_BORDER_DARK)
        };

        let offset = if is_clicking { Vec2::new(0.0, 1.0) } else { Vec2::ZERO };
        let draw_rect = rect.translate(offset);

        if !is_active && !is_clicking {
            painter.rect_filled(
                rect.translate(Vec2::new(1.0, 1.0)), 
                0.0, 
                Color32::from_black_alpha(25)
            );
        }
        
        painter.rect_filled(draw_rect, 0.0, bg_color);
        painter.rect_stroke(draw_rect, 0.0, Stroke::new(1.0, border_color), StrokeKind::Inside);

        if is_active {
             painter.line_segment(
                [draw_rect.left_top() + Vec2::new(0.0, 1.0), draw_rect.right_top() + Vec2::new(0.0, 1.0)],
                Stroke::new(1.0, Color32::from_black_alpha(30))
             );
        }

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

// --- 2. Tech Volume Knob (DISABLED / PLACEHOLDER) ---
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
        // SAFE FALLBACK: Just use a Slider for now
        ui.add(egui::Slider::new(self.value, self.min..=self.max).text("Volume"))
    }
}

// --- 3. Speaker Box (SIMPLIFIED) ---
// 移除复杂的角落装饰，只画最基本的矩形，排除崩溃源
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
        
        let (bg_color, text_color, border_color) = if self.active {
            (COLOR_ACTIVE_SLATE_BG, Color32::WHITE, COLOR_ACTIVE_SLATE_BG)
        } else if is_hovered {
            (Color32::WHITE, COLOR_TEXT_MEDIUM, COLOR_TEXT_MEDIUM)
        } else {
            (Color32::WHITE, COLOR_BORDER_DARK, COLOR_BORDER_MEDIUM)
        };

        // 1. Basic Background & Border
        painter.rect_filled(rect, 0.0, bg_color);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, border_color), StrokeKind::Inside);

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
