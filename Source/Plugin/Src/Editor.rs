#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, EguiState, widgets};
// 使用 nih_plug_egui 重导出的 egui，避免版本冲突
// 将 Rounding 替换为 CornerRadius (egui 0.31+)
use nih_plug_egui::egui::{self, Color32, Rect, CornerRadius, Sense, Vec2};
use std::sync::Arc;
use crate::Params::MonitorParams;

pub fn create_editor(params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(800, 600);
    
    create_egui_editor(
        egui_state,
        (), // User State
        |_, _| {}, // Update Hook
        move |ctx, setter, _state| { // Draw Hook: (ctx, setter, state)
            // 设置全局样式 (Dark Theme)
            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = Color32::from_rgb(30, 34, 39); // 类似截图的深灰色背景
            ctx.set_visuals(visuals);

            egui::CentralPanel::default().show(ctx, |ui| {
                // 顶部标题栏
                ui.horizontal(|ui| {
                    ui.label("Options");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("MonitorControllerMax");
                        // TODO: Role dropdown
                    });
                });
                
                ui.separator();

                // 主布局：左侧控制栏 + 中间绘图区
                ui.horizontal(|ui| {
                    // 左侧控制栏
                    ui.vertical(|ui| {
                        let btn_size = Vec2::new(60.0, 40.0);
                        if ui.add(egui::Button::new("SOLO").min_size(btn_size)).clicked() {
                            // TODO: Logic
                        }
                        if ui.add(egui::Button::new("DIM").min_size(btn_size)).clicked() {
                            // TODO: Logic
                        }
                        if ui.add(egui::Button::new("MUTE").min_size(btn_size)).clicked() {
                            // TODO: Logic
                        }

                        ui.add_space(20.0);
                        
                        // Master Gain Knob (简化为 Slider 暂代)
                        ui.add(widgets::ParamSlider::for_param(&params.master_gain, setter));
                    });

                    // 中间绘图区 (高性能矢量绘图)
                    let (response, painter) = ui.allocate_painter(
                        ui.available_size(), 
                        Sense::hover()
                    );

                    // 绘制音箱 (示例：L, C, R)
                    let center = response.rect.center();
                    
                    // Center Speaker
                    painter.rect_filled(
                        Rect::from_center_size(center, Vec2::new(50.0, 50.0)), 
                        CornerRadius::same(5), // u8
                        Color32::from_rgb(60, 65, 70)
                    );
                    painter.text(
                        center, 
                        egui::Align2::CENTER_CENTER, 
                        "C", 
                        egui::FontId::proportional(14.0), 
                        Color32::WHITE
                    );

                    // Left Speaker
                    painter.rect_filled(
                        Rect::from_center_size(center - Vec2::new(100.0, 0.0), Vec2::new(50.0, 50.0)), 
                        CornerRadius::same(5), // u8
                        Color32::from_rgb(60, 65, 70)
                    );
                    painter.text(
                        center - Vec2::new(100.0, 0.0), 
                        egui::Align2::CENTER_CENTER, 
                        "L", 
                        egui::FontId::proportional(14.0), 
                        Color32::WHITE
                    );

                     // Right Speaker
                     painter.rect_filled(
                        Rect::from_center_size(center + Vec2::new(100.0, 0.0), Vec2::new(50.0, 50.0)), 
                        CornerRadius::same(5), // u8
                        Color32::from_rgb(60, 65, 70)
                    );
                    painter.text(
                        center + Vec2::new(100.0, 0.0), 
                        egui::Align2::CENTER_CENTER, 
                        "R", 
                        egui::FontId::proportional(14.0), 
                        Color32::WHITE
                    );
                });
            });
        },
    )
}
