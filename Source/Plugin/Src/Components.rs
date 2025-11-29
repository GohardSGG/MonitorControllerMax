#![allow(non_snake_case)]

use nih_plug_egui::egui::{
    self, Color32, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, StrokeKind, Align2, FontId,
};
use nih_plug_egui::egui::emath::Rot2;

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

// --- 1. Brutalist Button ---
pub struct BrutalistButton<'a> {
    label: &'a str,
    active: bool,
    danger: bool,
    full_width: bool,
    height: f32,
    scale: f32,
}

impl<'a> BrutalistButton<'a> {
    pub fn new(label: &'a str, scale: f32) -> Self {
        Self {
            label,
            active: false,
            danger: false,
            full_width: false,
            height: 40.0 * scale,
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

    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }
    
    pub fn large(mut self) -> Self {
        self.height = 56.0 * self.scale;
        self
    }
}

impl<'a> Widget for BrutalistButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = if self.full_width {
            Vec2::new(ui.available_width(), self.height)
        } else {
            Vec2::new(80.0 * self.scale, self.height)
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
            (Color32::WHITE, COLOR_TEXT_MEDIUM, COLOR_BORDER_MEDIUM) // Milder border for inactive
        };

        let offset = if is_clicking { Vec2::new(0.0, 1.0 * self.scale) } else { Vec2::ZERO };
        let draw_rect = rect.translate(offset);

        // Shadow effect
        if !is_active && !is_clicking {
            painter.rect_filled(
                rect.translate(Vec2::new(1.0 * self.scale, 1.0 * self.scale)), 
                0.0, 
                Color32::from_black_alpha(20) // Lighter shadow
            );
        }
        
        painter.rect_filled(draw_rect, 0.0, bg_color);
        painter.rect_stroke(draw_rect, 0.0, Stroke::new(1.0 * self.scale, border_color), StrokeKind::Inside);

        painter.text(
            draw_rect.center(),
            Align2::CENTER_CENTER,
            self.label,
            if self.height > 45.0 * self.scale { FontId::proportional(14.0 * self.scale) } else { FontId::proportional(12.0 * self.scale) },
            text_color,
        );

        response
    }
}

// --- 2. Volume Knob (Simplified for now, styled slider) ---
pub struct TechVolumeKnob<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
    scale: f32,
}

impl<'a> TechVolumeKnob<'a> {
    pub fn new(value: &'a mut f32, scale: f32) -> Self {
        Self { value, min: 0.0, max: 12.0, scale } // Default to typical monitor range
    }
}

impl<'a> Widget for TechVolumeKnob<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Vertical slider for now to fit sidebar
        let slider_height = 200.0 * self.scale;
        let slider_width = 40.0 * self.scale;
        
        ui.vertical_centered(|ui| {
             let resp = ui.add(egui::Slider::new(self.value, self.min..=self.max)
                .vertical()
                .text("")
                .show_value(false)
                .trailing_fill(true)
            );
            ui.label(format!("{:.1} dB", self.value));
            resp
        }).inner
    }
}

// --- 3. Speaker Box ---
pub struct SpeakerBox<'a> {
    name: &'a str,
    active: bool,
    is_sub: bool,
    scale: f32,
    label: Option<&'a str>, // For "CH 7", "AUX" labels below
}

impl<'a> SpeakerBox<'a> {
    pub fn new(name: &'a str, active: bool, scale: f32) -> Self {
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
        let box_size = if self.is_sub { Vec2::new(80.0 * self.scale, 80.0 * self.scale) } else { Vec2::new(96.0 * self.scale, 96.0 * self.scale) };
        
        // Container for Box + Label
        let (rect, response) = ui.allocate_exact_size(
            if self.label.is_some() { box_size + Vec2::new(0.0, 20.0 * self.scale) } else { box_size }, 
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
        painter.rect_stroke(box_rect, 0.0, Stroke::new(1.0 * self.scale, border_color), StrokeKind::Inside);

        // 2. Corner Accents (Tech Feel)
        let corner_len = 4.0 * self.scale;
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
            FontId::monospace(14.0 * self.scale),
            text_color,
        );
        
        // 4. External Label (if any)
        if let Some(label_text) = self.label {
            let label_rect = Rect::from_min_size(
                box_rect.left_bottom() + Vec2::new(0.0, 4.0 * self.scale),
                Vec2::new(box_size.x, 14.0 * self.scale)
            );
            // Label background tag
            let tag_width = 30.0 * self.scale;
            let tag_rect = Rect::from_center_size(label_rect.center(), Vec2::new(tag_width, 12.0 * self.scale));
            painter.rect_filled(tag_rect, 0.0, Color32::WHITE);
            
            painter.text(
                tag_rect.center(),
                Align2::CENTER_CENTER,
                label_text,
                FontId::proportional(9.0 * self.scale),
                COLOR_TEXT_LIGHT,
            );
        }

        response
    }
}
