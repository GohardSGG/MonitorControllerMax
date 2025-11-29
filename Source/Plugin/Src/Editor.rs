#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{self, Visuals, Vec2, CornerRadius, Margin};
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
            // 1. 获取物理尺寸
            let (physical_width, _) = egui_state_clone.size();
            
            // 2. 计算缩放比例
            let scale_factor = (physical_width as f32 / BASE_WIDTH).clamp(0.5, 4.0);

            // -------------------------------------------------------------
            // 核心修复：手动实施“样式缩放”
            // 既然系统忽略了我们的布局缩放请求，我们就手动把所有东西放大
            // -------------------------------------------------------------
            
            // A. 设置渲染精度 (让放大后的文字保持锐利，不模糊)
            ctx.set_pixels_per_point(scale_factor);

            // B. 获取默认样式并进行修改
            let mut style = (*ctx.style()).clone();

            // C. 暴力修正：反向缩放 pixels_per_point
            // 解释：因为我们在上面设置了 ppp，egui 内部可能会把字体渲染得很精细。
            // 但为了让布局由于 ppp 失效而“变大”，我们需要手动修改所有尺寸属性。
            // 这里的逻辑是：如果 Logical Width 被锁定为 Physical Width，
            // 那么我们需要让所有组件的大小 = 原始大小 * Scale。
            
            // --- 1. 缩放字体 ---
            for (text_style, font_id) in style.text_styles.iter_mut() {
                // 我们需要手动指定每种文字类型的默认大小 * scale
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

            // --- 2. 缩放间距 (Spacing) ---
            style.spacing.item_spacing = Vec2::new(8.0, 3.0) * scale_factor;
            // 修复：Margin::same 期望 i8
            style.spacing.window_margin = Margin::same((6.0 * scale_factor).round() as i8);
            style.spacing.button_padding = Vec2::new(4.0, 1.0) * scale_factor;
            style.spacing.indent = 18.0 * scale_factor;
            style.spacing.interact_size = Vec2::new(40.0, 18.0) * scale_factor; // 按钮点击区域
            style.spacing.slider_width = 100.0 * scale_factor;
            style.spacing.icon_width = 14.0 * scale_factor; // 复选框大小
            style.spacing.icon_width_inner = 8.0 * scale_factor;
            style.spacing.icon_spacing = 4.0 * scale_factor;

            
            // --- 3. 缩放视觉元素 (Visuals) ---
            // 修复：Rounding -> CornerRadius, f32 -> u8, .rounding -> .corner_radius
            let r2 = (2.0 * scale_factor).round() as u8;
            let r3 = (3.0 * scale_factor).round() as u8;
            
            // --- 3. 缩放视觉元素 (圆角、线条) ---
            style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(r2);
            style.visuals.widgets.inactive.corner_radius = CornerRadius::same(r2);
            style.visuals.widgets.hovered.corner_radius = CornerRadius::same(r3);
            style.visuals.widgets.active.corner_radius = CornerRadius::same(r2);
            style.visuals.widgets.open.corner_radius = CornerRadius::same(r2);
            
            style.visuals.selection.stroke.width = 1.0 * scale_factor;

            // 应用修改后的样式
            ctx.set_style(style);

            // -------------------------------------------------------------

            let visuals = Visuals::dark();
            ctx.set_visuals(visuals);

            ResizableWindow::new("main_window_resize")
                .min_size(Vec2::ZERO) 
                .show(ctx, &egui_state_clone, |ui| {
                    
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0 * scale_factor); // 别忘了手动缩放固定的 space
                        ui.heading("MonitorControllerMax");
                        ui.label("Fixed Vector Scaling");
                        
                        ui.add_space(20.0 * scale_factor);
                        ui.label(format!("Physical: {} px", physical_width));
                        ui.label(format!("Scale: {:.2}x", scale_factor));
                        
                        // 现在这里的 Logical Width 依然会很大，但因为我们的字体和按钮也变大了
                        // 视觉上它看起来就像是缩放了
                        ui.label(format!("Logical: {:.1}", ui.available_width()));
                        
                        ui.add_space(30.0 * scale_factor);
                        
                        // Example Usage of Scaled Components
                        ui.add(BrutalistButton::new("Scaled Button", scale_factor).full_width(false).active(true));
                        
                        ui.add_space(10.0 * scale_factor);
                        
                        ui.horizontal(|ui| {
                            ui.add(SpeakerBox::new("L", true, scale_factor));
                            ui.add(SpeakerBox::new("R", false, scale_factor));
                        });

                        if ui.button("Standard Egui Button (Auto Scaled)").clicked() {
                           // ...
                        }
                    });
                });
        },
    )
}