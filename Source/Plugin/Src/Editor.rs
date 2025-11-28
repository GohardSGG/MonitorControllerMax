#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{self, Color32, Layout, Rect, RichText, Stroke, Vec2, Visuals, UiBuilder, Pos2, Sense, CursorIcon};
use std::sync::Arc;
use std::collections::HashSet;
use crate::Params::MonitorParams;

use crate::Components::*;

// 基准设计尺寸
const BASE_WIDTH: f32 = 1024.0;
const BASE_HEIGHT: f32 = 768.0;

// GUI 专属的临时状态
struct GuiState {
    role: String,
    active_speakers: HashSet<String>,
    logs: Vec<String>,
    
    solo_active: bool,
    mute_active: bool,
    dim_active: bool,
    master_mute_active: bool,
    effect_active: bool,
    
    volume_val: f32,
}

impl Default for GuiState {
    fn default() -> Self {
        let mut s = Self {
            role: "Standalone".to_string(),
            active_speakers: HashSet::new(),
            logs: Vec::new(),
            solo_active: false,
            mute_active: false,
            dim_active: false,
            master_mute_active: false,
            effect_active: false,
            volume_val: 0.08,
        };
        s.active_speakers.insert("L".into());
        s.active_speakers.insert("C".into());
        s.active_speakers.insert("R".into());
        s.active_speakers.insert("LFE".into());
        s.active_speakers.insert("LR".into());
        s.active_speakers.insert("RR".into());
        s
    }
}

impl GuiState {
    fn add_log(&mut self, msg: &str) {
        let time = "12:00:00"; 
        self.logs.insert(0, format!("[{}] {}", time, msg));
        if self.logs.len() > 10 {
            self.logs.pop();
        }
    }
}

pub fn create_editor(_params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    let egui_state_clone = egui_state.clone();
    
    create_egui_editor(
        egui_state,
        GuiState::default(), 
        |_, _| {}, 
        move |ctx, _setter, state| {
            // --- Global Style ---
            let mut visuals = Visuals::light();
            visuals.window_fill = COLOR_BG_APP;
            visuals.panel_fill = COLOR_BG_MAIN;
            ctx.set_visuals(visuals);

            // 使用 ResizableWindow
            ResizableWindow::new("main_window_resize")
                .min_size(Vec2::new(800.0, 600.0))
                .show(ctx, &egui_state_clone, |ui| {
                    
                    // 强制保持纵横比逻辑 (Visual Only)
                    // 我们不能强制改变物理窗口，但我们可以让内容区域保持比例
                    // 并在多余区域填充背景色
                    
                    // --- Layout Start ---
                    
                    ui.horizontal(|ui| {
                        // 1. Sidebar (Left) - Fixed Width
                        let sidebar_width = 176.0;
                        // 使用 available_height 确保它填满当前高度
                        let sidebar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(sidebar_width, ui.available_height()));
                        
                        ui.allocate_new_ui(UiBuilder::new().max_rect(sidebar_rect), |ui| {
                            // Background
                            ui.painter().rect_filled(ui.max_rect(), 0.0, COLOR_BG_SIDEBAR);
                            // Border Right
                            ui.painter().line_segment(
                                [ui.max_rect().right_top(), ui.max_rect().right_bottom()], 
                                Stroke::new(2.0, COLOR_BORDER_LIGHT)
                            );

                            ui.vertical_centered(|ui| {
                                ui.add_space(10.0);
                                // Branding
                                ui.label(RichText::new("Monitor").size(16.0).family(egui::FontFamily::Proportional));
                                ui.strong("Controller");
                                ui.add_space(4.0);
                                ui.label(RichText::new("v5.0").size(10.0).monospace().color(COLOR_TEXT_MEDIUM));
                                ui.add_space(20.0);

                                // Controls
                                if ui.add(BrutalistButton::new("SOLO").active(state.solo_active).large().full_width(true)).clicked() {
                                    state.solo_active = !state.solo_active;
                                    state.add_log(&format!("SOLO turned {}", if state.solo_active {"ON"} else {"OFF"}));
                                }
                                ui.add_space(12.0);
                                if ui.add(BrutalistButton::new("MUTE").active(state.mute_active).danger(true).large().full_width(true)).clicked() {
                                    state.mute_active = !state.mute_active;
                                    state.add_log(&format!("MUTE turned {}", if state.mute_active {"ON"} else {"OFF"}));
                                }

                                ui.add_space(24.0);
                                ui.painter().line_segment(
                                    [ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)], 
                                    Stroke::new(1.0, COLOR_BORDER_LIGHT)
                                );
                                ui.add_space(24.0);

                                // Volume
                                ui.add(TechVolumeKnob::new(&mut state.volume_val));

                                ui.add_space(20.0);
                                if ui.add(BrutalistButton::new("DIM").active(state.dim_active).full_width(true)).clicked() {
                                    state.dim_active = !state.dim_active;
                                    state.add_log(&format!("DIM turned {}", if state.dim_active {"ON"} else {"OFF"}));
                                }

                                ui.add_space(24.0);
                                ui.painter().line_segment(
                                    [ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)], 
                                    Stroke::new(1.0, COLOR_BORDER_LIGHT)
                                );
                                ui.add_space(24.0);

                                if ui.add(BrutalistButton::new("M. MUTE").active(state.master_mute_active).danger(true).full_width(true)).clicked() {
                                    state.master_mute_active = !state.master_mute_active;
                                    state.add_log("Master MUTE toggled");
                                }
                                ui.add_space(12.0);
                                if ui.add(BrutalistButton::new("EFFECT").active(state.effect_active).full_width(true)).clicked() {
                                    state.effect_active = !state.effect_active;
                                    state.add_log("EFFECT toggled");
                                }
                            });
                        });

                        // 2. Right Content Area (Fill remaining)
                        let content_width = ui.available_width();
                        let content_height = ui.available_height();
                        
                        ui.allocate_ui(Vec2::new(content_width, content_height), |ui| {
                            ui.painter().rect_filled(ui.max_rect(), 0.0, COLOR_BG_MAIN);
                            
                            // Top Bar
                            ui.allocate_new_ui(UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 56.0))), |ui| {
                                let rect = ui.max_rect();
                                ui.painter().line_segment(
                                    [rect.left_bottom(), rect.right_bottom()], 
                                    Stroke::new(1.0, COLOR_BG_APP)
                                );

                                ui.horizontal(|ui| {
                                    ui.add_space(24.0);
                                    ui.label(RichText::new("OUTPUT ROUTING MATRIX").monospace().size(12.0).color(COLOR_TEXT_LIGHT));
                                    
                                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add_space(24.0);
                                        
                                        // Role/Format Selectors (Simplified Visuals)
                                        let _ = ui.selectable_label(false, "5.1"); 
                                        ui.add_space(8.0);
                                        ui.label(RichText::new("ROLE").size(10.0).strong().color(COLOR_TEXT_LIGHT));
                                        let _ = ui.selectable_label(false, format!("{}", state.role));
                                    });
                                });
                            });

                            // Main Grid - Centered
                            ui.vertical(|ui| {
                                // 计算剩余高度用于垂直居中
                                let remaining_h = ui.available_height() - 160.0; // Minus log panel
                                let grid_h = 400.0; // Approx grid height
                                let top_pad = (remaining_h - grid_h) / 2.0;
                                if top_pad > 0.0 { ui.add_space(top_pad); }
                                
                                ui.horizontal(|ui| {
                                    // 水平居中
                                    let grid_w = 500.0;
                                    let left_pad = (ui.available_width() - grid_w) / 2.0;
                                    if left_pad > 0.0 { ui.add_space(left_pad); }
                                    
                                    ui.vertical(|ui| {
                                        // Row 1
                                        ui.horizontal(|ui| {
                                            let gap = 48.0;
                                            if ui.add(SpeakerBox::new("L", state.active_speakers.contains("L"))).clicked() { toggle_speaker(state, "L"); }
                                            ui.add_space(gap);
                                            if ui.add(SpeakerBox::new("C", state.active_speakers.contains("C"))).clicked() { toggle_speaker(state, "C"); }
                                            ui.add_space(gap);
                                            if ui.add(SpeakerBox::new("R", state.active_speakers.contains("R"))).clicked() { toggle_speaker(state, "R"); }
                                        });
                                        ui.add_space(64.0);
                                        // Row 2
                                        ui.horizontal(|ui| {
                                            let gap = 48.0;
                                            ui.add_space(16.0);
                                            if ui.add(SpeakerBox::new("SUB L", state.active_speakers.contains("SUB L"))).clicked() { toggle_speaker(state, "SUB L"); }
                                            ui.add_space(gap + 16.0); 
                                            if ui.add(SpeakerBox::new("LFE", state.active_speakers.contains("LFE"))).clicked() { toggle_speaker(state, "LFE"); }
                                            ui.add_space(gap + 16.0);
                                            if ui.add(SpeakerBox::new("SUB R", state.active_speakers.contains("SUB R"))).clicked() { toggle_speaker(state, "SUB R"); }
                                        });
                                        ui.add_space(64.0);
                                        // Row 3
                                        ui.horizontal(|ui| {
                                            let gap = 48.0;
                                            if ui.add(SpeakerBox::new("LR", state.active_speakers.contains("LR"))).clicked() { toggle_speaker(state, "LR"); }
                                            ui.add_space(gap);
                                            ui.add_space(16.0);
                                            if ui.add(SpeakerBox::new("SUB", state.active_speakers.contains("SUB"))).clicked() { toggle_speaker(state, "SUB"); }
                                            ui.add_space(gap + 16.0);
                                            if ui.add(SpeakerBox::new("RR", state.active_speakers.contains("RR"))).clicked() { toggle_speaker(state, "RR"); }
                                        });
                                    });
                                });
                            });

                            // Log Panel (Pinned Bottom)
                            let log_height = 160.0;
                            // Ensure it sticks to bottom even if window is tall
                            let log_y = ui.max_rect().bottom() - log_height;
                            let log_rect = Rect::from_min_size(
                                Pos2::new(ui.max_rect().left(), log_y),
                                Vec2::new(ui.available_width(), log_height)
                            );
                            
                            ui.allocate_new_ui(UiBuilder::new().max_rect(log_rect), |ui| {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(248, 250, 252));
                                ui.painter().line_segment(
                                    [ui.max_rect().left_top(), ui.max_rect().right_top()],
                                    Stroke::new(2.0, COLOR_BORDER_LIGHT)
                                );
                                
                                ui.allocate_new_ui(UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 28.0))), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add_space(16.0);
                                        ui.label(RichText::new("EVENT LOG").size(10.0).strong().color(COLOR_TEXT_MEDIUM));
                                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.add_space(16.0);
                                            if ui.button(RichText::new("CLEAR").size(10.0).color(COLOR_TEXT_LIGHT)).clicked() {
                                                state.logs.clear();
                                            }
                                        });
                                    });
                                });
                                
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.add_space(4.0);
                                        for log in &state.logs {
                                            ui.horizontal(|ui| {
                                                ui.add_space(8.0);
                                                ui.label(RichText::new(log).monospace().size(11.0).color(COLOR_TEXT_MEDIUM));
                                            });
                                        }
                                    });
                                });
                            });

                            // Resize Handle (Visual)
                            let resize_size = Vec2::splat(16.0);
                            let resize_rect = Rect::from_min_size(ui.max_rect().right_bottom() - resize_size, resize_size);
                            
                            // Visual lines
                            let painter = ui.painter();
                            let stroke = Stroke::new(2.0, COLOR_BORDER_DARK);
                            for i in 0..3 {
                                let offset = i as f32 * 4.0;
                                painter.line_segment(
                                    [
                                        resize_rect.right_bottom() - Vec2::new(4.0 + offset, 0.0),
                                        resize_rect.right_bottom() - Vec2::new(0.0, 4.0 + offset)
                                    ],
                                    stroke
                                );
                            }
                        });
                    });
                });
        },
    )
}

fn toggle_speaker(state: &mut GuiState, name: &str) {
    if state.active_speakers.contains(name) {
        state.active_speakers.remove(name);
        state.add_log(&format!("[ROUTING] Speaker {} muted", name));
    } else {
        state.active_speakers.insert(name.to_string());
        state.add_log(&format!("[ROUTING] Speaker {} active", name));
    }
}
