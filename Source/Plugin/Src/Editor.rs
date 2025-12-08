#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{
    self, Visuals, Vec2, Color32, Layout, Align, RichText, ComboBox,
    Stroke, LayerId, Frame, TopBottomPanel, SidePanel, CentralPanel, Grid, StrokeKind
};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::Components::{self, *};
use crate::scale::ScaleContext;
use crate::config_manager::CONFIG;
use crate::config_file::APP_CONFIG;
use crate::mcm_info;
use crate::Interaction::{get_interaction_manager, SubClickType, ChannelMarker, InteractionManager};
use crate::osc::{OSC_SENDER, OSC_RECEIVER, OscManager};

// 用于跨帧追踪布局变化的静态变量
static PREV_LAYOUT: AtomicI32 = AtomicI32::new(-1);  // -1 表示未初始化
static PREV_SUB_LAYOUT: AtomicI32 = AtomicI32::new(-1);

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

            // === 布局变化检测（使用 AtomicI32 跨帧持久化）===
            let current_layout = params.layout.value();
            let current_sub_layout = params.sub_layout.value();

            let prev_layout = PREV_LAYOUT.load(Ordering::Relaxed);
            let prev_sub = PREV_SUB_LAYOUT.load(Ordering::Relaxed);

            // 检测变化：prev != -1（已初始化）且值不同
            let first_load = prev_layout == -1;
            let layout_changed = (prev_layout != -1 && prev_layout != current_layout) ||
                                 (prev_sub != -1 && prev_sub != current_sub_layout);

            // 更新存储的值
            PREV_LAYOUT.store(current_layout, Ordering::Relaxed);
            PREV_SUB_LAYOUT.store(current_sub_layout, Ordering::Relaxed);

            // 如果首次加载或布局发生变化且处于手动模式，同步所有通道参数
            if first_load || layout_changed {
                let interaction = get_interaction_manager();
                if !interaction.is_automation_mode() {
                    // 获取布局名称和通道数
                    let speaker_layouts = CONFIG.get_speaker_layouts();
                    let sub_layouts = CONFIG.get_sub_layouts();

                    let prev_speaker_name = speaker_layouts.get(prev_layout as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());
                    let curr_speaker_name = speaker_layouts.get(current_layout as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());
                    let prev_sub_name = sub_layouts.get(prev_sub as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());
                    let curr_sub_name = sub_layouts.get(current_sub_layout as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());

                    let prev_total = CONFIG.get_layout(&prev_speaker_name, &prev_sub_name).total_channels;
                    let curr_layout = CONFIG.get_layout(&curr_speaker_name, &curr_sub_name);
                    let curr_total = curr_layout.total_channels;

                    mcm_info!("[LAYOUT] {}+{} -> {}+{} ({}ch->{}ch), sync triggered",
                        prev_speaker_name, prev_sub_name, curr_speaker_name, curr_sub_name,
                        prev_total, curr_total);

                    // 更新 OSC 通道信息（KISS 方案：动态从布局获取通道名称）
                    OscManager::update_layout_channels(&curr_layout);

                    sync_all_channel_params(params, setter, interaction);

                    // 布局变化后广播完整状态给硬件（KISS：自动清空已删除的通道）
                    OscManager::broadcast_channel_states();
                }
            }

            // === OSC 接收处理：检查是否有从外部接收的参数变化 ===
            if let Some((volume, dim, cut)) = OSC_RECEIVER.get_pending_changes() {
                // 更新 Master Volume
                setter.begin_set_parameter(&params.master_gain);
                setter.set_parameter(&params.master_gain, volume);
                setter.end_set_parameter(&params.master_gain);

                // 更新 Dim
                setter.begin_set_parameter(&params.dim);
                setter.set_parameter(&params.dim, dim);
                setter.end_set_parameter(&params.dim);

                // 更新 Cut
                setter.begin_set_parameter(&params.cut);
                setter.set_parameter(&params.cut, cut);
                setter.end_set_parameter(&params.cut);

                mcm_info!("[OSC Recv] Applied changes: volume={:.3}, dim={}, cut={}", volume, dim, cut);

                // 立即回显 OSC 状态（告诉硬件控制器参数已更新）
                OSC_SENDER.send_master_volume(volume);
                OSC_SENDER.send_dim(dim);
                OSC_SENDER.send_cut(cut);
            }

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

                    // 设置弹窗
                    let dialog_id = egui::Id::new("settings_dialog");
                    let show_settings = ctx.memory(|m| m.data.get_temp::<bool>(dialog_id).unwrap_or(false));

                    if show_settings {
                        egui::Window::new("Settings")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                render_settings_content(ui, &scale, dialog_id, params, setter);
                            });

                        // 自动化确认对话框（从设置窗口触发）
                        let confirm_id = egui::Id::new("automation_confirm_from_settings");
                        let show_confirm = ctx.memory(|m| m.data.get_temp::<bool>(confirm_id).unwrap_or(false));
                        if show_confirm {
                            egui::Window::new("确认启用自动化")
                                .collapsible(false)
                                .resizable(false)
                                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                                .show(ctx, |ui| {
                                    ui.label("启用自动化模式将清空当前的 Solo/Mute 设置。");
                                    ui.label("确定要继续吗？");
                                    ui.add_space(scale.s(12.0));
                                    ui.horizontal(|ui| {
                                        if ui.button("确定").clicked() {
                                            let interaction = get_interaction_manager();
                                            interaction.enter_automation_mode();
                                            mcm_info!("[AUTO] Enter: cleared all state, params unchanged (controlled by DAW)");
                                            ui.memory_mut(|m| m.data.remove::<bool>(confirm_id));
                                        }
                                        if ui.button("取消").clicked() {
                                            ui.memory_mut(|m| m.data.remove::<bool>(confirm_id));
                                        }
                                    });
                                });
                        }
                    }
                });
        },
    )
}

/// 同步所有通道的 enable 参数到 VST3（手动模式下使用）
fn sync_all_channel_params(params: &Arc<MonitorParams>, setter: &ParamSetter, interaction: &InteractionManager) {
    // 获取当前布局信息
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

    // 同步所有通道并生成摘要
    let mut on_mask: u32 = 0;
    for i in 0..layout.total_channels {
        if i >= MAX_CHANNELS { break; }

        // 查找通道信息
        let channel_info = layout.main_channels.iter()
            .chain(layout.sub_channels.iter())
            .find(|ch| ch.channel_index == i);

        if let Some(ch_info) = channel_info {
            // 获取通道显示状态（基于通道名称）
            let display = interaction.get_channel_display(&ch_info.name);

            // 记录到位掩码
            if display.has_sound {
                on_mask |= 1 << i;
            }

            // 同步到 VST3 参数
            setter.begin_set_parameter(&params.channels[i].enable);
            setter.set_parameter(&params.channels[i].enable, display.has_sound);
            setter.end_set_parameter(&params.channels[i].enable);
        }
    }

    // 输出同步摘要日志
    let on_count = on_mask.count_ones();
    let off_count = layout.total_channels as u32 - on_count;
    mcm_info!("[SYNC] {}ch: {}on/{}off mask=0x{:x}",
        layout.total_channels, on_count, off_count, on_mask);
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

            // === 检查是否允许布局切换 ===
            let interaction = get_interaction_manager();
            let is_automation = interaction.is_automation_mode();
            let can_change_layout = !is_automation; // 自动化模式下禁止切换布局

            // --- Helper: 带微调偏移的 Dropdown (参数绑定版) ---
            let dropdown_y_offset_local = dropdown_y_offset;
            let combo_font_local = combo_font.clone();

            // 1. Subs dropdown (First in Right-to-Left layout = Last Visually)
            ui.add_enabled_ui(can_change_layout, |ui| {
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
            });

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Sub");
            ui.add_space(scale.s(12.0));

            // 2. Maps dropdown (Speaker Layout)
            ui.add_enabled_ui(can_change_layout, |ui| {
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
            });

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Map");
            ui.add_space(scale.s(12.0));

            // 齿轮设置按钮
            {
                let gear_btn = ui.add(egui::Button::new(RichText::new("⚙")
                    .font(scale.font(18.0))
                    .color(COLOR_TEXT_MEDIUM))
                    .frame(false));

                if gear_btn.clicked() {
                    let dialog_id = egui::Id::new("settings_dialog");
                    ui.ctx().memory_mut(|m| m.data.insert_temp(dialog_id, true));
                }
            }

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

                                        // 如果切换到 Master/Slave，自动退出自动化模式
                                        if new_role != PluginRole::Standalone {
                                            let interaction = get_interaction_manager();
                                            if interaction.is_automation_mode() {
                                                interaction.exit_automation_mode();
                                                mcm_info!("[Editor] Auto-exited automation mode (switched to {:?})", new_role);
                                            }
                                        }

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

            // 获取交互管理器
            let interaction = get_interaction_manager();

            // 更新闪烁动画计数器
            interaction.tick_blink();
            let blink_show = interaction.should_blink_show();

            // Group 1: Solo/Mute 全局按钮
            // SOLO 按钮状态：常亮 或 闪烁
            let solo_steady = interaction.is_solo_steady();
            let solo_blinking = interaction.is_solo_blinking();
            let solo_visible = if solo_blinking {
                blink_show  // 闪烁模式：跟随 blink
            } else {
                solo_steady  // 常亮模式：直接显示
            };

            let solo_btn = BrutalistButton::new("SOLO", scale)
                .large()
                .full_width(true)
                .success(true)  // 绿色按钮
                .active(solo_visible);

            if ui.add(solo_btn).clicked() {
                let primary_before = interaction.get_primary();
                let compare_before = interaction.get_compare();
                interaction.on_solo_button_click();
                mcm_info!("[Editor] SOLO clicked: ({:?}, {:?}) -> ({:?}, {:?})",
                    primary_before, compare_before,
                    interaction.get_primary(), interaction.get_compare());

                // 同步所有通道的 enable 参数
                sync_all_channel_params(params, setter, &interaction);

                // 发送 OSC 模式状态
                OSC_SENDER.send_mode_solo(interaction.is_solo_active());
                if !interaction.is_mute_active() {
                    OSC_SENDER.send_mode_mute(false);
                }

                // 广播所有通道的 LED 状态（防止退出模式时 LED 状态不同步）
                OscManager::broadcast_channel_states();
            }

            ui.add_space(scale.s(12.0));

            // MUTE 按钮状态：常亮 或 闪烁
            let mute_steady = interaction.is_mute_steady();
            let mute_blinking = interaction.is_mute_blinking();
            let mute_visible = if mute_blinking {
                blink_show  // 闪烁模式：跟随 blink
            } else {
                mute_steady  // 常亮模式：直接显示
            };

            let mute_btn = BrutalistButton::new("MUTE", scale)
                .large()
                .danger(true)  // 红色按钮
                .full_width(true)
                .active(mute_visible);

            if ui.add(mute_btn).clicked() {
                let primary_before = interaction.get_primary();
                let compare_before = interaction.get_compare();
                interaction.on_mute_button_click();
                mcm_info!("[Editor] MUTE clicked: ({:?}, {:?}) -> ({:?}, {:?})",
                    primary_before, compare_before,
                    interaction.get_primary(), interaction.get_compare());

                // 同步所有通道的 enable 参数
                sync_all_channel_params(params, setter, &interaction);

                // 发送 OSC 模式状态
                OSC_SENDER.send_mode_mute(interaction.is_mute_active());
                if !interaction.is_solo_active() {
                    OSC_SENDER.send_mode_solo(false);
                }

                // 广播所有通道的 LED 状态（防止退出模式时 LED 状态不同步）
                OscManager::broadcast_channel_states();
            }

            ui.add_space(scale.s(24.0));
            ui.separator();
            ui.add_space(scale.s(24.0));

            // Volume Knob Area - 绑定到 params.master_gain
            ui.vertical_centered(|ui| {
                // 从 params 读取当前增益值并转换为百分比显示（匹配旧 C++ 版本）
                let current_gain = params.master_gain.value();
                // 0.0-1.0 增益 → 0-100 百分比（线性映射）
                let mut volume_percent = current_gain * 100.0;

                let response = ui.add(TechVolumeKnob::new(&mut volume_percent, scale));

                if response.changed() {
                    // 转换回增益值：0-100% → 0.0-1.0
                    let new_gain = (volume_percent / 100.0).clamp(0.0, 1.0);
                    setter.begin_set_parameter(&params.master_gain);
                    setter.set_parameter(&params.master_gain, new_gain);
                    setter.end_set_parameter(&params.master_gain);

                    // 发送 OSC（使用 0-1 线性值）
                    OSC_SENDER.send_master_volume(new_gain);
                }

                // 只在拖动结束时记录日志
                if response.drag_stopped() {
                    mcm_info!("[Editor] Master volume set to: {:.1}%", volume_percent);
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

                    // 发送 OSC
                    OSC_SENDER.send_dim(new_value);
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

                    // 发送 OSC
                    OSC_SENDER.send_cut(new_value);
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

/// 渲染音箱矩阵（新版：SUB 在上下轨道，整体居中）
fn render_speaker_matrix(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, _setter: &ParamSetter) {
    // 检查是否处于自动化模式
    let interaction = get_interaction_manager();
    let is_automation = interaction.is_automation_mode();

    // 自动化模式全局提示
    if is_automation {
        ui.horizontal(|ui| {
            ui.add_space(scale.s(16.0));
            ui.label(egui::RichText::new("🔒 自动化控制中")
                .size(scale.s(14.0))
                .color(egui::Color32::from_rgb(251, 191, 36))); // Amber-400
            ui.label(egui::RichText::new("(通道状态由 VST3 参数控制)")
                .size(scale.s(11.0))
                .color(egui::Color32::from_rgb(156, 163, 175))); // Gray-400
        });
        ui.add_space(scale.s(8.0));
    }

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

    // === 动态计算尺寸 ===
    let grid_w = layout.width as f32;
    let grid_h = layout.height as f32;

    // 可用区域
    let available_width = ui.available_width();
    let available_height = ui.available_height();

    // 间距常量
    let grid_spacing = scale.s(12.0);      // 主网格内部间距
    let sub_spacing = scale.s(16.0);       // SUB 行与主网格的间距
    let label_height = scale.s(20.0);      // 通道标签高度

    // 计算主网格按钮大小（基于可用宽度）
    // 宽度约束：降低比例让主音箱稍小
    let max_width_for_grid = available_width * 0.75;
    let box_size_from_width = (max_width_for_grid - grid_spacing * (grid_w - 1.0)) / grid_w;

    // 高度约束：需要容纳 SUB行 + 间距 + 主网格 + 间距 + SUB行
    // SUB 比例提高到 0.7，让 SUB 相对更大
    let sub_ratio = 0.7;
    let total_sub_overhead = 2.0 * (sub_spacing);  // 两个间距
    let main_grid_overhead = label_height * grid_h + grid_spacing * (grid_h - 1.0);
    let max_height_for_content = available_height * 0.95;
    let box_size_from_height = (max_height_for_content - total_sub_overhead - main_grid_overhead) / (2.0 * sub_ratio + grid_h);

    // 取较小值，确保两个方向都能容纳
    let box_size = box_size_from_width.min(box_size_from_height).max(scale.s(40.0));  // 最小 40px

    // SUB 按钮直径 = 主按钮的 55%
    let sub_diameter = box_size * sub_ratio;
    let sub_row_height = sub_diameter + scale.s(4.0);  // 一点余量

    // 计算实际内容尺寸
    let main_grid_width = box_size * grid_w + grid_spacing * (grid_w - 1.0);
    let main_grid_height = (box_size + label_height) * grid_h + grid_spacing * (grid_h - 1.0);
    let total_content_height = sub_row_height + sub_spacing + main_grid_height + sub_spacing + sub_row_height;

    // 计算居中偏移
    let top_padding = ((available_height - total_content_height) / 2.0).max(0.0);

    // 垂直布局：整体居中
    ui.vertical(|ui| {
        // 顶部留白实现垂直居中
        ui.add_space(top_padding);

        // 上方 SUB 行
        ui.horizontal(|ui| {
            let padding = (available_width - main_grid_width) / 2.0;
            ui.add_space(padding.max(0.0));
            render_sub_row_dynamic(ui, scale, &layout, 1..=3, sub_diameter, main_grid_width, params, _setter);
        });

        ui.add_space(sub_spacing);

        // 主网格
        render_main_grid_dynamic(ui, scale, &layout, box_size, grid_spacing, label_height, params, _setter);

        ui.add_space(sub_spacing);

        // 下方 SUB 行
        ui.horizontal(|ui| {
            let padding = (available_width - main_grid_width) / 2.0;
            ui.add_space(padding.max(0.0));
            render_sub_row_dynamic(ui, scale, &layout, 4..=6, sub_diameter, main_grid_width, params, _setter);
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

/// 渲染 SUB 通道行（动态尺寸版本）
fn render_sub_row_dynamic(
    ui: &mut egui::Ui,
    scale: &ScaleContext,
    layout: &crate::config_manager::Layout,
    pos_range: std::ops::RangeInclusive<u32>,
    sub_diameter: f32,
    container_width: f32,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter,
) {
    let interaction = get_interaction_manager();
    let is_automation = interaction.is_automation_mode();

    // 计算 SUB 行内的间距，使 3 个按钮均匀分布在 container_width 内
    // 总宽度 = 3 * sub_diameter + 2 * spacing = container_width
    let sub_spacing = (container_width - sub_diameter * 3.0) / 2.0;

    let range_end = *pos_range.end();
    for pos in pos_range.clone() {
        // 查找该位置的 SUB 通道
        if let Some(ch) = layout.sub_channels.iter().find(|c| c.grid_pos == pos) {
            // 计算 SUB 相对索引（0-3），用于 Interaction 函数
            // ch.channel_index 是绝对索引（12-15），需要减去 main 通道数量
            let sub_relative_idx = ch.channel_index - layout.main_channels.len();

            let sub_btn = if is_automation {
                // 自动化模式：从参数读取状态，显示为锁定样式
                let enable = params.channels[ch.channel_index].enable.value();
                Components::SubButton::new(&ch.name, scale)
                    .diameter(sub_diameter)
                    .enabled(enable)
                    .locked(true)
            } else {
                // 手动模式：使用 InteractionManager 状态（基于通道名称）
                let display = interaction.get_channel_display(&ch.name);
                Components::SubButton::new(&ch.name, scale)
                    .diameter(sub_diameter)
                    .solo(display.marker == Some(ChannelMarker::Solo))
                    .muted(display.marker == Some(ChannelMarker::Mute))
            };

            let response = ui.add(sub_btn);

            // 点击处理（仅手动模式）
            if response.clicked() && !is_automation {
                // 使用相对索引进行双击检测（保持一致性）
                let click_type = interaction.detect_sub_click(sub_relative_idx);
                match click_type {
                    SubClickType::SingleClick => {
                        // on_channel_click 使用通道名称
                        interaction.on_channel_click(&ch.name);
                        mcm_info!("[Editor] SUB {} ({}) single click", sub_relative_idx, ch.name);
                    }
                    SubClickType::DoubleClick => {
                        // on_sub_double_click 使用通道名称
                        interaction.on_sub_double_click(&ch.name);
                        mcm_info!("[Editor] SUB {} ({}) double click -> Mute toggle", sub_relative_idx, ch.name);
                    }
                }

                // 全通道同步（Solo/Mute 操作会影响所有通道的 has_sound 状态）
                sync_all_channel_params(params, setter, interaction);

                // 发送 OSC 所有通道 LED 状态（三态）
                OscManager::broadcast_channel_states();
            }

            // 右键：SUB 的 User Mute 反转（替代双击）（仅手动模式）
            if response.secondary_clicked() && !is_automation {
                // on_sub_double_click 使用通道名称
                interaction.on_sub_double_click(&ch.name);
                mcm_info!("[Editor] SUB {} ({}) right-click -> Mute toggle", sub_relative_idx, ch.name);

                // 全通道同步（SUB Mute 操作可能影响整体状态）
                sync_all_channel_params(params, setter, interaction);

                // 发送 OSC 所有通道 LED 状态（三态）
                OscManager::broadcast_channel_states();
            }
        } else {
            // 空槽位占位（圆形直径）
            ui.allocate_space(Vec2::splat(sub_diameter));
        }

        if pos != range_end {
            ui.add_space(sub_spacing.max(scale.s(8.0)));  // 最小间距 8px
        }
    }
}

/// 渲染主网格（动态尺寸版本，接收预计算的 box_size）
fn render_main_grid_dynamic(
    ui: &mut egui::Ui,
    scale: &ScaleContext,
    layout: &crate::config_manager::Layout,
    box_size: f32,
    grid_spacing: f32,
    label_height: f32,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter,
) {
    let interaction = get_interaction_manager();
    let grid_w = layout.width as f32;

    // 居中
    let actual_width = box_size * grid_w + grid_spacing * (grid_w - 1.0);
    let padding = (ui.available_width() - actual_width) / 2.0;

    ui.horizontal(|ui| {
        ui.add_space(padding.max(0.0));

        ui.vertical(|ui| {
            Grid::new("main_speaker_grid")
                .num_columns(layout.width as usize)
                .spacing(Vec2::new(grid_spacing, grid_spacing))
                .show(ui, |ui| {
                    for row in 0..layout.height {
                        for col in 0..layout.width {
                            let grid_pos = row * layout.width + col + 1;

                            if let Some(ch) = layout.main_channels.iter().find(|c| c.grid_pos == grid_pos) {
                                let ch_idx = ch.channel_index;
                                let is_sub = false;
                                let is_automation = interaction.is_automation_mode();

                                let channel_label = format!("CH {}", ch_idx + 1);
                                let speaker_box = if is_automation {
                                    // 自动化模式：从参数读取状态，显示为锁定样式
                                    let enable = params.channels[ch_idx].enable.value();
                                    SpeakerBox::new(&ch.name, scale)
                                        .size(box_size)
                                        .enabled(enable)
                                        .locked(true)
                                        .with_label(&channel_label)
                                } else {
                                    // 手动模式：使用 InteractionManager 状态
                                    let display = interaction.get_channel_display(&ch.name);
                                    let blink_show = interaction.should_blink_show();
                                    let (show_solo, show_mute) = if display.is_blinking && !blink_show {
                                        (false, false)
                                    } else {
                                        (display.marker == Some(ChannelMarker::Solo),
                                         display.marker == Some(ChannelMarker::Mute))
                                    };

                                    SpeakerBox::new(&ch.name, scale)
                                        .size(box_size)
                                        .solo(show_solo)
                                        .muted(show_mute)
                                        .with_label(&channel_label)
                                };

                                let response = ui.add(speaker_box);

                                // 点击处理（仅手动模式）
                                if response.clicked() && !is_automation {
                                    interaction.on_channel_click(&ch.name);
                                    mcm_info!("[Editor] Main {} ({}) clicked", ch_idx, ch.name);

                                    // 全通道同步（Solo/Mute 操作会影响所有通道的 has_sound 状态）
                                    sync_all_channel_params(params, setter, interaction);

                                    // 发送 OSC 所有通道 LED 状态（三态）
                                    OscManager::broadcast_channel_states();
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

/// 渲染设置窗口内容
fn render_settings_content(
    ui: &mut egui::Ui,
    scale: &ScaleContext,
    dialog_id: egui::Id,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter
) {
    let mut config = APP_CONFIG.get();
    let mut changed = false;

    ui.add_space(scale.s(8.0));

    // ========== 自动化模式设置 ==========
    ui.heading(RichText::new("Automation Mode").font(scale.font(16.0)));
    ui.add_space(scale.s(12.0));

    let interaction = get_interaction_manager();
    let role = params.role.value();
    let is_automation = interaction.is_automation_mode();
    let can_use_automation = role == crate::Params::PluginRole::Standalone;

    ui.add_enabled_ui(can_use_automation, |ui| {
        let button_text = if is_automation { "退出自动化" } else { "启用自动化" };
        let auto_btn = BrutalistButton::new(button_text, scale)
            .full_width(true)
            .active(is_automation);

        if ui.add(auto_btn).clicked() {
            if is_automation {
                interaction.exit_automation_mode();
                mcm_info!("[AUTO] Exit: idle state, will sync to all=On on next UI update");
                // 同步所有通道参数到全 On（退出自动化 = Idle）
                sync_all_channel_params(params, setter, &interaction);
            } else {
                // 弹出确认对话框
                let confirm_id = egui::Id::new("automation_confirm_from_settings");
                ui.memory_mut(|m| m.data.insert_temp(confirm_id, true));
            }
        }
    });

    if !can_use_automation {
        ui.label(egui::RichText::new("(仅 Standalone 可用)")
            .size(scale.s(9.0))
            .color(egui::Color32::from_rgb(156, 163, 175)));
    }

    ui.add_space(scale.s(16.0));
    ui.separator();
    ui.add_space(scale.s(16.0));

    // OSC 设置
    ui.heading(RichText::new("OSC Settings").font(scale.font(16.0)));
    ui.add_space(scale.s(12.0));

    ui.horizontal(|ui| {
        ui.label(RichText::new("Send Port:").font(scale.font(14.0)));
        ui.add_space(scale.s(8.0));
        let mut port_str = config.osc_send_port.to_string();
        let text_edit = egui::TextEdit::singleline(&mut port_str)
            .desired_width(scale.s(80.0));
        if ui.add(text_edit).changed() {
            if let Ok(port) = port_str.parse::<u16>() {
                config.osc_send_port = port;
                changed = true;
            }
        }
    });

    ui.add_space(scale.s(8.0));

    ui.horizontal(|ui| {
        ui.label(RichText::new("Receive Port:").font(scale.font(14.0)));
        ui.add_space(scale.s(8.0));
        let mut port_str = config.osc_receive_port.to_string();
        let text_edit = egui::TextEdit::singleline(&mut port_str)
            .desired_width(scale.s(80.0));
        if ui.add(text_edit).changed() {
            if let Ok(port) = port_str.parse::<u16>() {
                config.osc_receive_port = port;
                changed = true;
            }
        }
    });

    ui.add_space(scale.s(16.0));
    ui.separator();
    ui.add_space(scale.s(16.0));

    // 按钮
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Save").font(scale.font(14.0))).clicked() {
            if let Err(e) = APP_CONFIG.apply_and_save(|c| *c = config.clone()) {
                mcm_info!("[Settings] Failed to save config: {}", e);
            } else {
                mcm_info!("[Settings] Config saved: send_port={}, recv_port={}",
                    config.osc_send_port, config.osc_receive_port);
            }
            ui.memory_mut(|m| m.data.remove::<bool>(dialog_id));
        }

        ui.add_space(scale.s(8.0));

        if ui.button(RichText::new("Cancel").font(scale.font(14.0))).clicked() {
            ui.memory_mut(|m| m.data.remove::<bool>(dialog_id));
        }
    });

    ui.add_space(scale.s(8.0));
}