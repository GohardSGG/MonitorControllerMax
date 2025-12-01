#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{
    self, Visuals, Vec2, Color32, Layout, Align, RichText, ComboBox,
    Stroke, LayerId, Frame, TopBottomPanel, SidePanel, CentralPanel, Grid, StrokeKind
};
use std::sync::Arc;
use crate::Params::{MonitorParams, PluginRole};
use crate::Components::*;
use crate::scale::ScaleContext;
use crate::config_manager::CONFIG;
use crate::mcm_info;

// --- 窗口尺寸常量 (1:1 正方形) ---
const BASE_WIDTH: f32 = 720.0;
const BASE_HEIGHT: f32 = 720.0;
const ASPECT_RATIO: f32 = 1.0;

// --- 颜色常量 ---
const COLOR_BORDER_MAIN: Color32 = Color32::from_rgb(30, 41, 59);  // 主边框颜色（深灰蓝）

pub fn create_editor(params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    let egui_state_clone = egui_state.clone();

    let params_clone = params.clone();

    create_egui_editor(
        egui_state,
        (),
        |_, _| {},
        move |ctx, setter, _state| {
            // 获取 params 的引用供渲染函数使用
            let params = &params_clone;
            // 1. 从 EguiState 获取物理像素尺寸（关键！不能用 ctx.screen_rect()）
            let (physical_width, _) = egui_state_clone.size();
            let scale = ScaleContext::from_physical_size(physical_width, BASE_WIDTH);

            // 2. 设置 egui 的 DPI 缩放（让内置组件如 ComboBox 正确缩放）
            // 注意：这里使用物理尺寸计算，不会导致循环
            ctx.set_pixels_per_point(scale.factor);

            // 3. 设置全局样式
            let mut visuals = Visuals::light();
            visuals.panel_fill = COLOR_BG_APP;
            ctx.set_visuals(visuals);

            // --- FIX 1: Global Background Fill (The Ultimate Gap Killer) ---
            // Paint a solid rectangle over the entire screen area before any panels.
            // This ensures that any sub-pixel gaps between panels reveal this color, not black.
            let screen = ctx.screen_rect();
            ctx.layer_painter(LayerId::background())
                .rect_filled(screen, 0.0, COLOR_BG_SIDEBAR); // Use sidebar color as base

            // 3. 绘制最外层边框
            ctx.layer_painter(LayerId::background())
                .rect_stroke(screen, 0.0, Stroke::new(scale.s(2.0), COLOR_BORDER_MAIN), StrokeKind::Outside);
            
            // --- FIX 1: Border fix ---
            // Define a frame that has NO stroke and NO margins.
            // This makes the Panels pure layout tools without any visual artifacts.
            let panel_frame = Frame::new()
                .fill(COLOR_BG_SIDEBAR)
                .stroke(Stroke::NONE)
                .inner_margin(egui::Margin::ZERO)
                .outer_margin(egui::Margin::ZERO);
            
            let central_frame = Frame::new()
                .fill(COLOR_BG_MAIN)
                .stroke(Stroke::NONE)
                .inner_margin(egui::Margin::ZERO)
                .outer_margin(egui::Margin::ZERO);

            // 4. 使用 ResizableWindow 和面板系统
            ResizableWindow::new("main")
                .with_aspect_ratio(ASPECT_RATIO)
                .show(ctx, &egui_state_clone, |ctx| {
                    // 顶部标题栏（包含下拉选择）
                    TopBottomPanel::top("header")
                        .min_height(scale.s(40.0)) // <-- CHANGED to min_height for flexibility
                        .frame(Frame::new().fill(Color32::WHITE))
                        .show(ctx, |ui| {
                            render_header(ui, &scale, params, setter);
                        });

                    // 左侧控制面板
                    SidePanel::left("sidebar")
                        .exact_width(scale.s(180.0))
                        .resizable(false)
                        .frame(panel_frame) // <-- Apply clean frame
                        .show(ctx, |ui| {
                            render_sidebar(ui, &scale, params, setter);
                        });

                    // 中央内容区域（音箱矩阵 + 日志面板）
                    CentralPanel::default()
                        .frame(central_frame) // <-- Apply clean frame
                        .show(ctx, |ui| {
                            // 子面板区域：上方音箱矩阵，下方日志
                            // 1. 获取折叠状态 (持久化ID)
                            let log_collapsed_id = ui.make_persistent_id("log_panel_collapsed");
                            let is_collapsed = ui.data(|d| d.get_temp::<bool>(log_collapsed_id).unwrap_or(false));
                            
                            // 2. 动态高度动画
                            // animate_bool_with_time 返回 0.0 (false) 到 1.0 (true) 的平滑值
                            // 我们定义: false = 展开 (1.0 height), true = 折叠 (0.0 height adjustment)
                            // 实际上: animate_bool: true -> 1.0. 
                            // 让我们反过来用: animate_bool(is_collapsed)
                            // t goes 0.0 (expanded) -> 1.0 (collapsed)
                            let t = ctx.animate_bool_with_time(log_collapsed_id, is_collapsed, 0.2); // 0.2s duration
                            
                            // Interpolate height
                            let expanded_height = scale.s(120.0);
                            let collapsed_height = scale.s(28.0);
                            // FIX: Import egui directly
                            let log_height = egui::lerp(expanded_height..=collapsed_height, t);

                            TopBottomPanel::bottom("log_panel")
                                .exact_height(log_height)
                                .frame(Frame::new())
                                .show_inside(ui, |ui| {
                                    render_log_panel(ui, &scale, log_collapsed_id);
                                });

                            CentralPanel::default()
                                .frame(Frame::new())
                                .show_inside(ui, |ui| {
                                    render_speaker_matrix(ui, &scale, params, setter);
                                });
                        });
                });
        },
    )
}

/// 渲染顶部标题栏 - 参数绑定版
fn render_header(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, setter: &ParamSetter) {
    let _header_height = scale.s(40.0);
    
    // --- 🟢 关键微调变量 (MANUAL TWEAK VARS) 🟢 ---
    // [下拉框] 垂直位置微调：
    // 正数 = 向下移动
    // 负数 = 向上移动 (通过添加底部填充实现挤压)
    let dropdown_y_offset = scale.s(1.0); 

    // [标签文字] 垂直位置微调：
    // 正数 = 向下移动
    // 负数 = 向上移动
    let label_y_offset = scale.s(5.5);

    // [标题 & 版本号] 垂直位置微调：
    // 正数 = 向下移动
    // 负数 = 向上移动
    let title_y_offset = scale.s(7.0);
    // ----------------------------------------------

    // 1. 顶部留白 (可选，如果依靠 Align::Center 则不需要)
    // let content_height = scale.s(24.0);
    // let top_padding = (header_height - content_height) / 2.0;
    // ui.add_space(top_padding);

    ui.horizontal(|ui| {
        ui.add_space(scale.s(8.0)); // Left padding

        // Title and Version container
        // Align::BOTTOM aligns the text baseline
        ui.vertical(|ui| {
            // Apply manual vertical offset
            ui.add_space(title_y_offset);
            
            ui.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                ui.label(RichText::new("MonitorControllerMax").font(scale.font(20.0)).color(COLOR_TEXT_DARK));
                ui.add_space(scale.s(2.0));
                ui.label(RichText::new("v2").font(scale.mono_font(12.0)).color(COLOR_TEXT_MEDIUM));
            });
        });

        // Right-aligned Dropdowns
        // 使用 right_to_left(Align::Center) 让所有元素默认垂直居中
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(scale.s(8.0)); // Right padding

            let combo_font = scale.font(14.0);
            
            // --- Helper: 带微调偏移的 Label ---
            let label_with_offset = |ui: &mut egui::Ui, text: &str| {
                let galley = ui.painter().layout_no_wrap(
                    text.to_string(), 
                    scale.mono_font(12.0), 
                    COLOR_TEXT_LIGHT
                );
                let (rect, _) = ui.allocate_exact_size(galley.rect.size(), egui::Sense::hover());
                
                // 绘制时应用 label_y_offset
                ui.painter().galley(
                    rect.min + Vec2::new(0.0, label_y_offset), 
                    galley, 
                    COLOR_TEXT_LIGHT
                );
            };

            // === 从配置系统获取布局选项 ===
            let speaker_layouts = CONFIG.get_speaker_layouts();
            let sub_layouts = CONFIG.get_sub_layouts();

            // === 从参数系统读取当前值 ===
            let current_role = params.role.value();
            let current_layout_idx = params.layout.value() as usize;
            let current_sub_idx = params.sub_layout.value() as usize;

            // --- Helper: 带微调偏移的 Dropdown (参数绑定版) ---
            let dropdown_y_offset_local = dropdown_y_offset;
            let combo_font_local = combo_font.clone();

            // 1. Subs dropdown (First in Right-to-Left layout = Last Visually)
            {
                let box_size = Vec2::new(scale.s(80.0), scale.s(40.0));
                ui.allocate_ui(box_size, |ui| {
                    ui.set_min_width(scale.s(80.0));
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        let estimated_combo_height = scale.s(20.0);
                        let base_padding = (box_size.y - estimated_combo_height) / 2.0;
                        let final_padding = base_padding + dropdown_y_offset_local;
                        if final_padding > 0.0 {
                            ui.add_space(final_padding);
                        }

                        let current_sub_name = sub_layouts.get(current_sub_idx)
                            .cloned()
                            .unwrap_or_else(|| "None".to_string());

                        ComboBox::from_id_salt("sub_layout_combo")
                            .selected_text(RichText::new(&current_sub_name).font(combo_font_local.clone()))
                            .width(scale.s(80.0))
                            .show_ui(ui, |ui| {
                                for (i, name) in sub_layouts.iter().enumerate() {
                                    if ui.selectable_label(current_sub_idx == i, RichText::new(name).font(combo_font_local.clone())).clicked() {
                                        mcm_info!("[Editor] Sub layout changed: {} -> {}", current_sub_name, name);
                                        setter.begin_set_parameter(&params.sub_layout);
                                        setter.set_parameter(&params.sub_layout, i as i32);
                                        setter.end_set_parameter(&params.sub_layout);
                                    }
                                }
                            });
                    });
                });
            }

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Sub");
            ui.add_space(scale.s(12.0));

            // 2. Maps dropdown (Speaker Layout)
            {
                let box_size = Vec2::new(scale.s(80.0), scale.s(40.0));
                ui.allocate_ui(box_size, |ui| {
                    ui.set_min_width(scale.s(80.0));
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        let estimated_combo_height = scale.s(20.0);
                        let base_padding = (box_size.y - estimated_combo_height) / 2.0;
                        let final_padding = base_padding + dropdown_y_offset_local;
                        if final_padding > 0.0 {
                            ui.add_space(final_padding);
                        }

                        let current_layout_name = speaker_layouts.get(current_layout_idx)
                            .cloned()
                            .unwrap_or_else(|| "Unknown".to_string());

                        ComboBox::from_id_salt("speaker_layout_combo")
                            .selected_text(RichText::new(&current_layout_name).font(combo_font_local.clone()))
                            .width(scale.s(80.0))
                            .show_ui(ui, |ui| {
                                for (i, name) in speaker_layouts.iter().enumerate() {
                                    if ui.selectable_label(current_layout_idx == i, RichText::new(name).font(combo_font_local.clone())).clicked() {
                                        mcm_info!("[Editor] Speaker layout changed: {} -> {}", current_layout_name, name);
                                        setter.begin_set_parameter(&params.layout);
                                        setter.set_parameter(&params.layout, i as i32);
                                        setter.end_set_parameter(&params.layout);
                                    }
                                }
                            });
                    });
                });
            }

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Map");
            ui.add_space(scale.s(12.0));

            // 3. Role dropdown (Plugin Role)
            {
                let box_size = Vec2::new(scale.s(100.0), scale.s(40.0));
                let role_names = ["Standalone", "Master", "Slave"];
                let current_role_idx = current_role as usize;

                ui.allocate_ui(box_size, |ui| {
                    ui.set_min_width(scale.s(100.0));
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        let estimated_combo_height = scale.s(20.0);
                        let base_padding = (box_size.y - estimated_combo_height) / 2.0;
                        let final_padding = base_padding + dropdown_y_offset_local;
                        if final_padding > 0.0 {
                            ui.add_space(final_padding);
                        }

                        ComboBox::from_id_salt("role_combo")
                            .selected_text(RichText::new(role_names[current_role_idx]).font(combo_font_local.clone()))
                            .width(scale.s(100.0))
                            .show_ui(ui, |ui| {
                                for (i, name) in role_names.iter().enumerate() {
                                    if ui.selectable_label(current_role_idx == i, RichText::new(*name).font(combo_font_local.clone())).clicked() {
                                        let new_role = match i {
                                            0 => PluginRole::Standalone,
                                            1 => PluginRole::Master,
                                            2 => PluginRole::Slave,
                                            _ => PluginRole::Standalone,
                                        };
                                        mcm_info!("[Editor] Role changed: {:?} -> {:?}", current_role, new_role);
                                        setter.begin_set_parameter(&params.role);
                                        setter.set_parameter(&params.role, new_role);
                                        setter.end_set_parameter(&params.role);
                                    }
                                }
                            });
                    });
                });
            }

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Role");

        });
    });

    // 标题栏底部边框（深色）
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(scale.s(1.0), COLOR_BORDER_MAIN)
    );
}

/// Helper: 自定义双行按钮 (Big Primary + Small Secondary)
fn custom_button(ui: &mut egui::Ui, primary: &str, secondary: &str, active: bool, width: f32, scale: &ScaleContext) -> egui::Response {
    // --- 🟢 关键微调变量 (MANUAL TWEAK VARS) 🟢 ---
    // 修改这里来控制这些新按钮的高度
    let height = scale.s(46.0); // 原来是 56.0
    // ----------------------------------------------

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response.hovered();
        
        let (bg_color, text_color, border_color) = if active {
            (crate::Components::COLOR_ACTIVE_YELLOW_BG, crate::Components::COLOR_TEXT_DARK, Color32::from_rgb(100, 116, 139))
        } else if is_hovered {
            (crate::Components::COLOR_BG_SIDEBAR, crate::Components::COLOR_TEXT_DARK, crate::Components::COLOR_BORDER_DARK)
        } else {
            (Color32::WHITE, crate::Components::COLOR_TEXT_MEDIUM, crate::Components::COLOR_BORDER_MEDIUM)
        };

        // Shadow
        if !active && !response.is_pointer_button_down_on() {
             painter.rect_filled(
                rect.translate(scale.vec2(1.0, 1.0)),
                0.0,
                Color32::from_black_alpha(20)
            );
        }

        painter.rect_filled(rect, 0.0, bg_color);
        painter.rect_stroke(rect, 0.0, Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);

        // Primary Text (Top Left)
        let primary_font = scale.font(16.0);
        let primary_galley = painter.layout_no_wrap(primary.to_string(), primary_font, text_color);
        painter.galley(rect.min + Vec2::new(scale.s(8.0), scale.s(8.0)), primary_galley, Color32::TRANSPARENT);

        // Secondary Text (Bottom Right)
        let secondary_font = scale.mono_font(10.0);
        let secondary_galley = painter.layout_no_wrap(secondary.to_string(), secondary_font, text_color);
        painter.galley(rect.max - secondary_galley.rect.size() - Vec2::new(scale.s(8.0), scale.s(8.0)), secondary_galley, Color32::TRANSPARENT);
    }
    response
}

/// 渲染左侧控制面板 - 参数绑定版
fn render_sidebar(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, setter: &ParamSetter) {
    
    ui.add_space(scale.s(24.0));

    let sidebar_content_width = scale.s(180.0) - scale.s(32.0);

    ui.horizontal(|ui| {
        ui.add_space(scale.s(16.0));

        ui.vertical(|ui| {
            ui.set_max_width(sidebar_content_width);

            // Group 1: Solo/Mute
            ui.add(BrutalistButton::new("SOLO", scale).large().full_width(true));
            ui.add_space(scale.s(12.0));
            ui.add(BrutalistButton::new("MUTE", scale).large().danger(true).full_width(true));

            ui.add_space(scale.s(24.0));
            ui.separator();
            ui.add_space(scale.s(24.0));

            // Volume Knob Area - 绑定到 params.master_gain
            ui.vertical_centered(|ui| {
                // 从 params 读取当前增益值并转换为 dB 显示
                let current_gain = params.master_gain.value();
                let current_db = nih_plug::util::gain_to_db(current_gain);

                // TechVolumeKnob 使用 dB 值（范围 -∞ 到 0 dB）
                let mut volume_val = current_db;
                let response = ui.add(TechVolumeKnob::new(&mut volume_val, scale));

                if response.changed() {
                    // 转换回增益值并设置参数（拖动时静默更新）
                    let new_gain = nih_plug::util::db_to_gain(volume_val);
                    setter.begin_set_parameter(&params.master_gain);
                    setter.set_parameter(&params.master_gain, new_gain);
                    setter.end_set_parameter(&params.master_gain);
                }

                // 只在拖动结束时记录日志
                if response.drag_stopped() {
                    mcm_info!("[Editor] Master volume set to: {:.1} dB", volume_val);
                }
            });

            // --- FIX 2: Layout spacing ---
            // Manually draw the separator line for precise control over spacing.
            ui.add_space(scale.s(16.0)); // Space above the line
            let line_rect = ui.available_rect_before_wrap();
            ui.painter().hline(line_rect.x_range(), line_rect.top(), Stroke::new(1.0, COLOR_BORDER_LIGHT));
            ui.add_space(scale.s(16.0)); // Space below the line

            // DIM + CUT buttons - 绑定到 params
            let button_width = (sidebar_content_width - scale.s(8.0)) / 2.0; // 减去中间间隙
            ui.horizontal(|ui| {
                // DIM 按钮
                let dim_active = params.dim.value();
                let dim_btn = BrutalistButton::new("DIM", scale)
                    .width(button_width)
                    .active(dim_active);
                if ui.add(dim_btn).clicked() {
                    let new_value = !dim_active;
                    mcm_info!("[Editor] DIM toggled: {} -> {}", dim_active, new_value);
                    setter.begin_set_parameter(&params.dim);
                    setter.set_parameter(&params.dim, new_value);
                    setter.end_set_parameter(&params.dim);
                }

                ui.add_space(scale.s(8.0));

                // CUT 按钮
                let cut_active = params.cut.value();
                let cut_btn = BrutalistButton::new("CUT", scale)
                    .width(button_width)
                    .danger(true)
                    .active(cut_active);
                if ui.add(cut_btn).clicked() {
                    let new_value = !cut_active;
                    mcm_info!("[Editor] CUT toggled: {} -> {}", cut_active, new_value);
                    setter.begin_set_parameter(&params.cut);
                    setter.set_parameter(&params.cut, new_value);
                    setter.end_set_parameter(&params.cut);
                }
            });

            // Second separator
            ui.add_space(scale.s(16.0));
            let line_rect_2 = ui.available_rect_before_wrap();
            ui.painter().hline(line_rect_2.x_range(), line_rect_2.top(), Stroke::new(1.0, COLOR_BORDER_LIGHT));
            ui.add_space(scale.s(16.0));
            
            // --- NEW: Low/High Boost Group ---
            ui.horizontal(|ui| {
                // Using custom_button for Low Boost
                // Need state management? Just placeholders for now or use memory
                let lb_id = ui.id().with("low_boost");
                let mut lb_active = ui.memory(|m| m.data.get_temp::<bool>(lb_id).unwrap_or(false));
                if custom_button(ui, "Low", "Boost", lb_active, button_width, scale).clicked() {
                     lb_active = !lb_active;
                     ui.memory_mut(|m| m.data.insert_temp(lb_id, lb_active));
                }

                ui.add_space(scale.s(8.0));

                let hb_id = ui.id().with("high_boost");
                let mut hb_active = ui.memory(|m| m.data.get_temp::<bool>(hb_id).unwrap_or(false));
                if custom_button(ui, "High", "Boost", hb_active, button_width, scale).clicked() {
                     hb_active = !hb_active;
                     ui.memory_mut(|m| m.data.insert_temp(hb_id, hb_active));
                }
            });

            ui.add_space(scale.s(12.0));

            // --- NEW: MONO / +10dB LFE Group ---
            ui.horizontal(|ui| {
                // MONO Button (Standard Brutalist?)
                let mono_id = ui.id().with("mono_btn");
                let mut mono_active = ui.memory(|m| m.data.get_temp::<bool>(mono_id).unwrap_or(false));
                // Use BrutalistButton but with same width logic
                // Or custom_button with empty secondary?
                // User said: "MONO 和 +10dB LFE"
                // Assuming MONO is standard style but split width
                let mut btn = BrutalistButton::new("MONO", scale).width(button_width); // Removed .large()
                btn = btn.active(mono_active);
                if ui.add(btn).clicked() {
                    mono_active = !mono_active;
                    ui.memory_mut(|m| m.data.insert_temp(mono_id, mono_active));
                }

                ui.add_space(scale.s(8.0));

                // +10dB LFE (Custom Button)
                let lfe_id = ui.id().with("lfe_boost");
                let mut lfe_active = ui.memory(|m| m.data.get_temp::<bool>(lfe_id).unwrap_or(false));
                if custom_button(ui, "+10dB", "LFE", lfe_active, button_width, scale).clicked() {
                     lfe_active = !lfe_active;
                     ui.memory_mut(|m| m.data.insert_temp(lfe_id, lfe_active));
                }
            });

            ui.add_space(scale.s(12.0));

            // --- NEW: Curve Button (Full Width) ---
            let curve_id = ui.id().with("curve_btn");
            let mut curve_active = ui.memory(|m| m.data.get_temp::<bool>(curve_id).unwrap_or(false));
            let mut curve_btn = BrutalistButton::new("Curve", scale).full_width(true); // Removed .large()
            curve_btn = curve_btn.active(curve_active);
            if ui.add(curve_btn).clicked() {
                curve_active = !curve_active;
                ui.memory_mut(|m| m.data.insert_temp(curve_id, curve_active));
            }
        });

        ui.add_space(scale.s(16.0));
    });
}

/// 渲染音箱矩阵（动态布局，参数绑定版）
fn render_speaker_matrix(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, setter: &ParamSetter) {
    // 绘制背景网格
    let rect = ui.max_rect();
    draw_grid_background(ui, rect, scale);

    // === 从配置系统获取当前布局 ===
    let layout_idx = params.layout.value() as usize;
    let sub_idx = params.sub_layout.value() as usize;

    let speaker_layouts = CONFIG.get_speaker_layouts();
    let sub_layouts = CONFIG.get_sub_layouts();

    let speaker_name = speaker_layouts.get(layout_idx)
        .cloned()
        .unwrap_or_else(|| "7.1.4".to_string());
    let sub_name = sub_layouts.get(sub_idx)
        .cloned()
        .unwrap_or_else(|| "None".to_string());

    let layout = CONFIG.get_layout(&speaker_name, &sub_name);

    // 计算矩阵尺寸以实现居中
    let box_size = scale.s(96.0);      // 音箱盒子尺寸
    let spacing_x = scale.s(32.0);
    let spacing_y = scale.s(24.0);
    let label_height = scale.s(20.0);  // 底部标签高度

    // 动态计算矩阵尺寸
    let grid_width = layout.width as f32;
    let grid_height = layout.height as f32;
    let matrix_width = box_size * grid_width + spacing_x * (grid_width - 1.0).max(0.0);
    let matrix_height = (box_size + label_height) * grid_height + spacing_y * (grid_height - 1.0).max(0.0);

    // 计算居中所需的间距
    let available_width = ui.available_width();
    let available_height = ui.available_height();
    let left_padding = ((available_width - matrix_width) / 2.0).max(0.0);
    let top_padding = ((available_height - matrix_height) / 2.0).max(0.0);

    // 使用水平布局添加左侧间距
    ui.horizontal(|ui| {
        ui.add_space(left_padding);

        ui.vertical(|ui| {
            ui.add_space(top_padding);

            let spacing = scale.vec2(32.0, 24.0);
            Grid::new("speaker_matrix")
                .num_columns(layout.width as usize)
                .spacing(spacing)
                .show(ui, |ui| {
                    // 遍历网格位置
                    for row in 0..layout.height {
                        for col in 0..layout.width {
                            // grid_pos 从 1 开始，计算方式：row * width + col + 1
                            let grid_pos = row * layout.width + col + 1;

                            // 查找该位置的通道
                            if let Some(ch) = layout.channels.iter()
                                .find(|c| c.grid_pos == grid_pos) {
                                // 获取通道状态
                                let ch_idx = ch.channel_index;
                                let is_muted = if ch_idx < params.channels.len() {
                                    params.channels[ch_idx].mute.value()
                                } else {
                                    false
                                };
                                let is_solo = if ch_idx < params.channels.len() {
                                    params.channels[ch_idx].solo.value()
                                } else {
                                    false
                                };

                                // 渲染音箱盒子
                                let label_text = format!("CH {}", ch_idx + 1);
                                let speaker_box = SpeakerBox::new(&ch.name, !is_muted, scale)
                                    .solo(is_solo)
                                    .with_label(&label_text);

                                let response = ui.add(speaker_box);

                                // 点击切换 Solo
                                if response.clicked() && ch_idx < params.channels.len() {
                                    let new_solo = !is_solo;
                                    mcm_info!("[Editor] Channel {} ({}) Solo toggled: {} -> {}",
                                        ch_idx, ch.name, is_solo, new_solo);
                                    setter.begin_set_parameter(&params.channels[ch_idx].solo);
                                    setter.set_parameter(&params.channels[ch_idx].solo, new_solo);
                                    setter.end_set_parameter(&params.channels[ch_idx].solo);
                                }

                                // 右键切换 Mute
                                if response.secondary_clicked() && ch_idx < params.channels.len() {
                                    let new_mute = !is_muted;
                                    mcm_info!("[Editor] Channel {} ({}) Mute toggled: {} -> {}",
                                        ch_idx, ch.name, is_muted, new_mute);
                                    setter.begin_set_parameter(&params.channels[ch_idx].mute);
                                    setter.set_parameter(&params.channels[ch_idx].mute, new_mute);
                                    setter.end_set_parameter(&params.channels[ch_idx].mute);
                                }
                            } else {
                                // 空位：绘制占位符
                                ui.allocate_space(Vec2::new(box_size, box_size + label_height));
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    });
}

/// 渲染日志面板
fn render_log_panel(ui: &mut egui::Ui, scale: &ScaleContext, collapse_id: egui::Id) {
    let is_collapsed = ui.data(|d| d.get_temp::<bool>(collapse_id).unwrap_or(false));
    let rect = ui.max_rect();

    // 顶部边框线
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(scale.s(1.0), COLOR_BORDER_MEDIUM)
    );

    // 标题栏
    let header_height = scale.s(28.0); // 稍微增加高度
    ui.allocate_ui(Vec2::new(ui.available_width(), header_height), |ui| {
        let header_rect = ui.max_rect();
        ui.painter().rect_filled(header_rect, 0.0, COLOR_BG_SIDEBAR);

        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(scale.s(1.0), COLOR_BORDER_LIGHT)
        );

        ui.horizontal(|ui| {
            ui.add_space(scale.s(12.0));
            
            // 标题: 稍微向上偏移以留出底部间隙
            ui.vertical(|ui| {
                ui.add_space(scale.s(4.0)); // Top padding
                ui.label(RichText::new("EVENT LOG").font(scale.mono_font(10.0)).color(COLOR_TEXT_MEDIUM));
                ui.add_space(scale.s(0.0)); // Bottom padding request
            });

            // 右上角折叠/释放按钮
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(scale.s(8.0));
                
                let (btn_text, btn_hover) = if is_collapsed { 
                    ("Show", "Expand Log") 
                } else { 
                    ("Hide", "Collapse Log") 
                };

                // 使用小巧的文本按钮
                if ui.add(egui::Button::new(
                    RichText::new(btn_text).font(scale.mono_font(10.0)).color(COLOR_TEXT_MEDIUM)
                ).frame(false)).on_hover_text(btn_hover).clicked() {
                    ui.data_mut(|d| d.insert_temp(collapse_id, !is_collapsed));
                }
            });
        });
    });

    // 仅在展开时绘制内容
    if !is_collapsed {
        // 日志内容区域
        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            Color32::from_rgb(230, 235, 240) // 更深的灰蓝色背景
        );

        ui.vertical(|ui| {
            ui.add_space(scale.s(8.0));
            ui.horizontal(|ui| {
                ui.add_space(scale.s(12.0));
                ui.label(RichText::new("-- No events logged --").font(scale.mono_font(10.0)).color(COLOR_TEXT_LIGHT));
            });
        });
    }
}

/// 绘制背景网格
fn draw_grid_background(ui: &mut egui::Ui, rect: egui::Rect, scale: &ScaleContext) {
    let grid_size = scale.s(40.0);
    let grid_color = Color32::from_gray(245); // 极淡的网格线

    // 垂直线
    let mut x = rect.min.x;
    while x < rect.max.x {
        ui.painter().line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            Stroke::new(scale.s(1.0), grid_color)
        );
        x += grid_size;
    }

    // 水平线
    let mut y = rect.min.y;
    while y < rect.max.y {
        ui.painter().line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            Stroke::new(scale.s(1.0), grid_color)
        );
        y += grid_size;
    }
}