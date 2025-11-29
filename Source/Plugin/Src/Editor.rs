#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{self, Visuals, Vec2, CornerRadius, Margin, Color32, Rect, Layout, Align}; // 引入 Layout, Align
use std::sync::Arc;
use crate::Params::MonitorParams;
use crate::Components::*; 

const BASE_WIDTH: f32 = 800.0;
const BASE_HEIGHT: f32 = 600.0;

pub fn create_editor(_params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    let egui_state_clone = egui_state.clone();

    create_egui_editor(
        egui_state,
        (), 
        |_, _| {}, 
        move |ctx, _setter, _state| {
            // ... (缩放和样式代码保持不变，请务必保留) ...
            let (physical_width, _) = egui_state_clone.size();
            let scale_factor = (physical_width as f32 / BASE_WIDTH).clamp(0.5, 4.0);
            ctx.set_pixels_per_point(scale_factor);

            // --- 样式设置 (简写，请确保保留完整的) ---
            let mut style = (*ctx.style()).clone();
             for (text_style, font_id) in style.text_styles.iter_mut() {
                let default_size = match text_style {
                    egui::TextStyle::Heading => 30.0,
                    egui::TextStyle::Body => 14.0,
                    egui::TextStyle::Monospace => 14.0,
                    egui::TextStyle::Button => 14.0,
                    egui::TextStyle::Small => 10.0,
                    _ => 12.0,
                };
                font_id.size = default_size * scale_factor;
            }
            style.spacing.item_spacing = Vec2::new(8.0, 3.0) * scale_factor;
            style.spacing.window_margin = Margin::same((6.0 * scale_factor).round() as i8);
            style.spacing.button_padding = Vec2::new(4.0, 1.0) * scale_factor;
            style.spacing.indent = 18.0 * scale_factor;
            style.spacing.interact_size = Vec2::new(40.0, 18.0) * scale_factor; 
            style.spacing.slider_width = 100.0 * scale_factor;
            style.spacing.icon_width = 14.0 * scale_factor;
            style.spacing.icon_width_inner = 8.0 * scale_factor;
            style.spacing.icon_spacing = 4.0 * scale_factor;

            let r2 = (2.0 * scale_factor).round() as u8;
            let r3 = (3.0 * scale_factor).round() as u8;
            style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(r2);
            style.visuals.widgets.inactive.corner_radius = CornerRadius::same(r2);
            style.visuals.widgets.hovered.corner_radius = CornerRadius::same(r3);
            style.visuals.widgets.active.corner_radius = CornerRadius::same(r2);
            style.visuals.widgets.open.corner_radius = CornerRadius::same(r2);
            style.visuals.selection.stroke.width = 1.0 * scale_factor;
            ctx.set_style(style);
            
            let visuals = Visuals::dark();
            ctx.set_visuals(visuals);

            // --- UI 渲染 ---
            ResizableWindow::new("main_window_resize")
                .min_size(Vec2::ZERO) 
                .show(ctx, &egui_state_clone, |ui| {
                    
                    // 1. Header (Window Bar) - 依然使用 allocate_ui
                    let header_height = 36.0 * scale_factor;
                    ui.allocate_ui(Vec2::new(ui.available_width(), header_height), |ui| {
                        let rect = ui.max_rect();
                        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
                        ui.painter().line_segment(
                            [rect.left_bottom(), rect.right_bottom()], 
                            nih_plug_egui::egui::Stroke::new(2.0 * scale_factor, COLOR_BORDER_LIGHT)
                        );
                        
                        ui.horizontal_centered(|ui| {
                            ui.add_space(12.0 * scale_factor);
                            ui.label(nih_plug_egui::egui::RichText::new("Monitor").font(nih_plug_egui::egui::FontId::proportional(18.0 * scale_factor)).color(COLOR_TEXT_DARK));
                            ui.label(nih_plug_egui::egui::RichText::new("ControllerMax").font(nih_plug_egui::egui::FontId::proportional(18.0 * scale_factor)).strong().color(COLOR_TEXT_DARK));
                            
                            // ... Header 内容 ...
                        });
                    });

                    // 2. 计算剩余空间的绝对矩形
                    // 关键：获取当前 ui 的 cursor 位置。Header 画完后，Cursor 应该在 y=36
                    let available_rect = ui.available_rect_before_wrap();
                    
                    // 定义侧边栏宽度
                    let sidebar_width = 176.0 * scale_factor;

                    // 手动构造两个绝对 Rect
                    // 左侧：从 current_y 开始，宽 176，高到底
                    let mut left_rect = available_rect;
                    left_rect.set_width(sidebar_width);
                    
                    // 右侧：从 current_y + 176 开始，宽剩余，高到底
                    let mut right_rect = available_rect;
                    right_rect.min.x += sidebar_width; 
                    
                    // --- 3. 绘制左侧 Sidebar (NO ui.horizontal wrapper!) ---
                    ui.allocate_ui_at_rect(left_rect, |ui| {
                        // 背景
                        ui.painter().rect_filled(ui.max_rect(), 0.0, COLOR_BG_SIDEBAR);
                        // 边框
                        let rect = ui.max_rect();
                        ui.painter().line_segment(
                            [rect.right_top(), rect.right_bottom()], 
                            nih_plug_egui::egui::Stroke::new(2.0 * scale_factor, COLOR_BORDER_LIGHT)
                        );
                        
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0 * scale_factor);
                            ui.add(BrutalistButton::new("SOLO", scale_factor).large().full_width(true));
                            ui.add_space(12.0 * scale_factor);
                            ui.add(BrutalistButton::new("MUTE", scale_factor).large().danger(true).full_width(true));
                            ui.add_space(24.0 * scale_factor);
                            ui.separator();
                            ui.add_space(24.0 * scale_factor);
                            
                            let mut dummy_vol = 8.0; 
                            ui.add(TechVolumeKnob::new(&mut dummy_vol, scale_factor));
                            
                            ui.add_space(16.0 * scale_factor);
                            ui.add(BrutalistButton::new("DIM", scale_factor).full_width(true));
                        });
                    });

                    // --- 4. 绘制右侧 Main Panel (NO ui.horizontal wrapper!) ---
                    ui.allocate_ui_at_rect(right_rect, |ui| {
                         ui.painter().rect_filled(ui.max_rect(), 0.0, COLOR_BG_MAIN);
                         
                         // Top Bar
                         ui.horizontal(|ui| {
                            ui.add_space(24.0 * scale_factor);
                            ui.label(nih_plug_egui::egui::RichText::new("OUTPUT ROUTING MATRIX").font(nih_plug_egui::egui::FontId::monospace(12.0 * scale_factor)).color(COLOR_TEXT_LIGHT));
                         });
                         ui.separator();

                         // Matrix
                         ui.vertical_centered(|ui| {
                            ui.add_space(40.0 * scale_factor);
                            let spacing = Vec2::new(48.0 * scale_factor, 64.0 * scale_factor);
                            nih_plug_egui::egui::Grid::new("speaker_matrix")
                                .spacing(spacing)
                                .show(ui, |ui| {
                                    ui.add(SpeakerBox::new("L", true, scale_factor));
                                    ui.add(SpeakerBox::new("C", true, scale_factor));
                                    ui.add(SpeakerBox::new("R", true, scale_factor));
                                    ui.end_row();
                                    ui.add(SpeakerBox::new("SUB L", false, scale_factor));
                                    ui.add(SpeakerBox::new("LFE", true, scale_factor));
                                    ui.add(SpeakerBox::new("SUB R", false, scale_factor));
                                    ui.end_row();
                                });
                         });
                    });
                });
        },
    )
}