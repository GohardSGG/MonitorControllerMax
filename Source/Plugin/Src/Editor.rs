#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, EguiState};
use nih_plug_egui::egui::{self, Color32, Layout, Rect, RichText, Stroke, Vec2, Visuals, UiBuilder};
use std::sync::Arc;
use std::collections::HashSet;
use crate::Params::MonitorParams;

// 引用 crate 根目录下的 Components
use crate::Components::*;

// GUI 专属的临时状态 (不随 DAW 工程保存)
struct GuiState {
    role: String,
    // format: String, // 暂时未使用
    active_speakers: HashSet<String>,
    logs: Vec<String>,
    
    // UI Control States (模拟)
    solo_active: bool,
    mute_active: bool,
    dim_active: bool,
    master_mute_active: bool,
    effect_active: bool,
    
    // Volume temporary state (0.0 - 1.0)
    volume_val: f32,
}

impl Default for GuiState {
    fn default() -> Self {
        let mut s = Self {
            role: "Standalone".to_string(),
            // format: "5.1".to_string(),
            active_speakers: HashSet::new(),
            logs: Vec::new(),
            solo_active: false,
            mute_active: false,
            dim_active: false,
            master_mute_active: false,
            effect_active: false,
            volume_val: 0.08, // 8.0%
        };
        // Initial active speakers
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
        let time = "12:00:00"; // 简化，实际可以用 chrono
        self.logs.insert(0, format!("[{}] {}", time, msg));
        if self.logs.len() > 10 {
            self.logs.pop();
        }
    }
}

pub fn create_editor(_params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(1024, 768); // Larger window for the new design
    
    create_egui_editor(
        egui_state,
        GuiState::default(), // Initialize User State
        |_, _| {}, 
        move |ctx, _setter, state| {
            // --- Global Style Setup ---
            let mut visuals = Visuals::light();
            visuals.window_fill = COLOR_BG_APP;
            visuals.panel_fill = COLOR_BG_MAIN;
            ctx.set_visuals(visuals);

            // --- Main Window Layout ---
            
            // 1. Header (Window Bar)
            egui::TopBottomPanel::top("header").height_range(36.0..=36.0).show(ctx, |ui| {
                ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::WHITE);
                ui.painter().line_segment(
                    [ui.max_rect().left_bottom(), ui.max_rect().right_bottom()], 
                    Stroke::new(2.0, COLOR_BORDER_LIGHT)
                );
                
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    // egui 0.28+ FontId handling is slightly different
                    ui.label(RichText::new("Monitor").size(18.0).family(egui::FontFamily::Proportional));
                    ui.strong("Controller");
                    
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("v5.0.0")
                        .background_color(COLOR_BORDER_LIGHT)
                        .color(COLOR_TEXT_MEDIUM)
                        .monospace()
                        .size(10.0)
                    );

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        // Window Controls (Fake)
                        ui.label("✕");
                        ui.add_space(8.0);
                        ui.label("－");
                        ui.add_space(12.0);
                        // Divider
                        ui.scope(|ui| {
                            ui.style_mut().spacing.item_spacing.x = 0.0;
                            ui.allocate_new_ui(UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, Vec2::new(1.0, 12.0))), |ui| {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, COLOR_BORDER_MEDIUM);
                            });
                        });
                        ui.add_space(12.0);
                        ui.label(RichText::new("Settings").size(12.0).color(COLOR_TEXT_MEDIUM));
                    });
                });
            });

            // 2. Sidebar (Left)
            egui::SidePanel::left("sidebar")
                .exact_width(176.0)
                // Frame::none() -> Frame::NONE
                .frame(egui::Frame::NONE.fill(COLOR_BG_SIDEBAR))
                .show(ctx, |ui| {
                    // Draw right border
                    let panel_rect = ui.max_rect();
                    ui.painter().line_segment(
                        [panel_rect.right_top(), panel_rect.right_bottom()], 
                        Stroke::new(2.0, COLOR_BORDER_LIGHT)
                    );

                    // allocate_ui_at_rect -> allocate_new_ui with builder
                    ui.allocate_new_ui(UiBuilder::new().max_rect(panel_rect.shrink(20.0)), |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            
                            // Top Group
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
                            // Divider
                            ui.painter().line_segment(
                                [ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)], 
                                Stroke::new(1.0, COLOR_BORDER_LIGHT)
                            );
                            ui.add_space(24.0);

                            // Volume Knob
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

                            // Bottom Group
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
                });

            // 3. Central Panel (Main)
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(COLOR_BG_MAIN))
                .show(ctx, |ui| {
                    // Top Bar (Settings)
                    // allocate_ui -> allocate_new_ui
                    ui.allocate_new_ui(UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 56.0))), |ui| {
                        let rect = ui.max_rect();
                         // Border bottom
                        ui.painter().line_segment(
                            [rect.left_bottom(), rect.right_bottom()], 
                            Stroke::new(1.0, COLOR_BG_APP) // slight separator
                        );

                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            // Label
                            ui.label(RichText::new("OUTPUT ROUTING MATRIX").monospace().size(12.0).color(COLOR_TEXT_LIGHT));
                            
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(24.0);
                                
                                // Format Selector (Fake)
                                let _ = ui.selectable_label(false, "5.1 ▼"); 
                                ui.add_space(8.0);
                                
                                // Role Selector (Fake)
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("ROLE").size(10.0).strong().color(COLOR_TEXT_LIGHT));
                                        let _ = ui.selectable_label(false, format!("{} ▼", state.role));
                                    });
                                });
                            });
                        });
                    });

                    // Main Grid Area
                    let grid_rect = ui.available_rect_before_wrap();
                    // Draw Grid Background (Optional)
                    
                    ui.vertical(|ui| {
                        ui.add_space(40.0);
                        
                        // Speaker Grid Layout
                        // Center the grid
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 500.0) / 2.0); // Manual centering
                            
                            ui.vertical(|ui| {
                                // Row 1
                                ui.horizontal(|ui| {
                                    let _w = 96.0;
                                    let gap = 48.0;
                                    
                                    if ui.add(SpeakerBox::new("L", state.active_speakers.contains("L"))).clicked() {
                                        toggle_speaker(state, "L");
                                    }
                                    ui.add_space(gap);
                                    if ui.add(SpeakerBox::new("C", state.active_speakers.contains("C"))).clicked() {
                                        toggle_speaker(state, "C");
                                    }
                                    ui.add_space(gap);
                                    if ui.add(SpeakerBox::new("R", state.active_speakers.contains("R"))).clicked() {
                                        toggle_speaker(state, "R");
                                    }
                                });
                                
                                ui.add_space(64.0); // Row Gap

                                // Row 2
                                ui.horizontal(|ui| {
                                    let gap = 48.0;
                                    // SUB L
                                    ui.add_space(16.0); // Offset for smaller box
                                    if ui.add(SpeakerBox::new("SUB L", state.active_speakers.contains("SUB L"))).clicked() {
                                        toggle_speaker(state, "SUB L");
                                    }
                                    ui.add_space(gap + 16.0); 
                                    
                                    // LFE
                                    if ui.add(SpeakerBox::new("LFE", state.active_speakers.contains("LFE"))).clicked() {
                                        toggle_speaker(state, "LFE");
                                    }
                                    ui.add_space(gap + 16.0);
                                    
                                    // SUB R
                                    if ui.add(SpeakerBox::new("SUB R", state.active_speakers.contains("SUB R"))).clicked() {
                                        toggle_speaker(state, "SUB R");
                                    }
                                });
                                
                                ui.add_space(64.0); // Row Gap
                                
                                // Row 3
                                ui.horizontal(|ui| {
                                    let gap = 48.0;
                                    if ui.add(SpeakerBox::new("LR", state.active_speakers.contains("LR"))).clicked() {
                                        toggle_speaker(state, "LR");
                                    }
                                    ui.add_space(gap);
                                    
                                    // SUB (AUX)
                                    ui.add_space(16.0);
                                    if ui.add(SpeakerBox::new("SUB", state.active_speakers.contains("SUB"))).clicked() {
                                        toggle_speaker(state, "SUB");
                                    }
                                    ui.add_space(gap + 16.0);
                                    
                                    if ui.add(SpeakerBox::new("RR", state.active_speakers.contains("RR"))).clicked() {
                                        toggle_speaker(state, "RR");
                                    }
                                });
                            });
                        });
                    });
                    
                    // Bottom Log Panel (Fixed height at bottom)
                    let log_height = 160.0;
                    let log_rect = Rect::from_min_size(
                        grid_rect.left_bottom() - Vec2::new(0.0, log_height),
                        Vec2::new(grid_rect.width(), log_height)
                    );
                    
                    ui.allocate_new_ui(UiBuilder::new().max_rect(log_rect), |ui| {
                        // Header
                        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(248, 250, 252)); // slate-50
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
                                    if ui.button(RichText::new("CLEAR CONSOLE").size(10.0).color(COLOR_TEXT_LIGHT)).clicked() {
                                        state.logs.clear();
                                    }
                                });
                            });
                             ui.painter().line_segment(
                                [ui.max_rect().left_bottom(), ui.max_rect().right_bottom()],
                                Stroke::new(1.0, COLOR_BORDER_LIGHT)
                            );
                        });
                        
                        // Content
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(4.0);
                                for log in &state.logs {
                                    ui.horizontal(|ui| {
                                        ui.add_space(8.0);
                                        ui.label(RichText::new(log).monospace().size(11.0).color(COLOR_TEXT_MEDIUM));
                                    });
                                }
                                if state.logs.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.add_space(8.0);
                                        ui.label(RichText::new("-- No events logged --").monospace().size(11.0).color(COLOR_BORDER_DARK));
                                    });
                                }
                            });
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
