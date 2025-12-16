#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{
    self, Visuals, Vec2, Color32, Layout, Align, RichText, ComboBox,
    Stroke, LayerId, Frame, TopBottomPanel, SidePanel, CentralPanel, Grid, StrokeKind
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::Components::{self, *};
use crate::Scale::ScaleContext;
use crate::Config_Manager::ConfigManager;
use crate::Config_File::AppConfig;
use crate::Logger::InstanceLogger;
use crate::Interaction::{SubClickType, ChannelMarker, InteractionManager};
use crate::Osc::OscSharedState;
use crate::Web_Protocol::{WebSharedState, WebRestartAction};

// --- 窗口尺寸常量 (1:1 正方形) ---
const BASE_WIDTH: f32 = 720.0;
const BASE_HEIGHT: f32 = 720.0;
const ASPECT_RATIO: f32 = 1.0;

// --- 颜色常量 ---
const COLOR_BORDER_MAIN: Color32 = Color32::from_rgb(30, 41, 59);  // 主边框颜色（深灰蓝）

// --- Settings 弹窗专用颜色 (Tailwind Slate) ---
const SETTINGS_SLATE_50: Color32 = Color32::from_rgb(248, 250, 252);
const SETTINGS_SLATE_100: Color32 = Color32::from_rgb(241, 245, 249);
const SETTINGS_SLATE_200: Color32 = Color32::from_rgb(226, 232, 240);
const SETTINGS_SLATE_300: Color32 = Color32::from_rgb(203, 213, 225);
const SETTINGS_SLATE_400: Color32 = Color32::from_rgb(148, 163, 184);
const SETTINGS_SLATE_500: Color32 = Color32::from_rgb(100, 116, 139);
const SETTINGS_SLATE_600: Color32 = Color32::from_rgb(71, 85, 105);
const SETTINGS_SLATE_700: Color32 = Color32::from_rgb(51, 65, 85);
const SETTINGS_SLATE_800: Color32 = Color32::from_rgb(30, 41, 59);
const SETTINGS_AMBER_600: Color32 = Color32::from_rgb(217, 119, 6);
const SETTINGS_RED_500: Color32 = Color32::from_rgb(239, 68, 68);

pub fn create_editor(
    params: Arc<MonitorParams>,
    interaction: Arc<InteractionManager>,
    osc_state: Arc<OscSharedState>,
    network_connected: Arc<AtomicBool>,
    logger: Arc<InstanceLogger>,
    app_config: AppConfig,
    layout_config: Arc<ConfigManager>,
    web_state: Arc<WebSharedState>,
) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    let egui_state_clone = egui_state.clone();

    let params_clone = params.clone();
    let interaction_clone = interaction.clone();
    let osc_state_clone = osc_state.clone();
    let network_connected_clone = network_connected.clone();
    let logger_clone = logger.clone();
    let app_config_clone = app_config.clone();
    let layout_config_clone = layout_config.clone();
    let web_state_clone = web_state.clone();

    // 实例级布局追踪变量（替代全局静态变量）
    let prev_layout = Arc::new(AtomicI32::new(-1));  // -1 表示未初始化
    let prev_sub_layout = Arc::new(AtomicI32::new(-1));
    let prev_role = Arc::new(AtomicI32::new(-1));  // Role 追踪变量
    let prev_layout_clone = prev_layout.clone();
    let prev_sub_clone = prev_sub_layout.clone();
    let prev_role_clone = prev_role.clone();

    create_egui_editor(
        egui_state,
        (),
        |_, _| {},
        move |ctx, setter, _state| {
            // === OSC 重绘请求检查：GUI 活跃时处理 OSC 驱动的 UI 更新 ===
            // 当 DAW 窗口没有焦点时，egui 不会主动刷新，
            // 但 OSC 仍在后台线程运行并更新参数。
            // 这里检查 OSC 是否请求了重绘，如果是则触发重绘。
            if osc_state_clone.take_repaint_request() {
                ctx.request_repaint();
            }

            // 获取 params 的引用供渲染函数使用
            let params = &params_clone;

            // === 布局变化检测（使用实例级 Arc<AtomicI32>）===
            let current_layout = params.layout.value();
            let current_sub_layout = params.sub_layout.value();

            let prev_layout_val = prev_layout_clone.load(Ordering::Relaxed);
            let prev_sub_val = prev_sub_clone.load(Ordering::Relaxed);

            // 检测变化：prev != -1（已初始化）且值不同
            let first_load = prev_layout_val == -1;
            let layout_changed = (prev_layout_val != -1 && prev_layout_val != current_layout) ||
                                 (prev_sub_val != -1 && prev_sub_val != current_sub_layout);

            // 更新存储的值
            prev_layout_clone.store(current_layout, Ordering::Relaxed);
            prev_sub_clone.store(current_sub_layout, Ordering::Relaxed);

            // 如果首次加载或布局发生变化且处于手动模式，同步所有通道参数
            if first_load || layout_changed {
                if !interaction_clone.is_automation_mode() {
                    // 获取布局名称和通道数
                    let speaker_layouts = layout_config_clone.get_speaker_layouts();
                    let sub_layouts = layout_config_clone.get_sub_layouts();

                    let prev_speaker_name = speaker_layouts.get(prev_layout_val as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());
                    let curr_speaker_name = speaker_layouts.get(current_layout as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());
                    let prev_sub_name = sub_layouts.get(prev_sub_val as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());
                    let curr_sub_name = sub_layouts.get(current_sub_layout as usize)
                        .cloned().unwrap_or_else(|| "?".to_string());

                    let prev_total = layout_config_clone.get_layout(&prev_speaker_name, &prev_sub_name).total_channels;
                    let curr_layout = layout_config_clone.get_layout(&curr_speaker_name, &curr_sub_name);
                    let curr_total = curr_layout.total_channels;

                    logger_clone.important("editor", &format!("[LAYOUT] {}+{} -> {}+{} ({}ch->{}ch), sync triggered",
                        prev_speaker_name, prev_sub_name, curr_speaker_name, curr_sub_name,
                        prev_total, curr_total));

                    // H2: 布局变化时清理旧 Solo/Mute 状态（防止通道状态污染）
                    if layout_changed {
                        interaction_clone.clear_on_layout_change();
                        logger_clone.info("editor", "[LAYOUT] Cleared old Solo/Mute state");
                    }

                    // 更新 OSC 通道信息（KISS 方案：动态从布局获取通道名称）
                    osc_state_clone.update_layout_channels(&curr_layout);

                    sync_all_channel_params(params, setter, &interaction_clone, &layout_config_clone, &logger_clone);

                    // 布局变化后广播完整状态给硬件（KISS：自动清空已删除的通道）
                    osc_state_clone.broadcast_channel_states(&interaction_clone);
                }
            }

            // === Role 变化检测（触发网络热重载）===
            let current_role_value = params.role.value() as i32;
            let prev_role_val = prev_role_clone.load(Ordering::Relaxed);

            if prev_role_val != -1 && prev_role_val != current_role_value {
                let new_role = params.role.value();
                let old_role_name = match prev_role_val { 0 => "Standalone", 1 => "Master", _ => "Slave" };
                logger_clone.important("editor", &format!("[ROLE] {} -> {:?}, triggering network re-init",
                    old_role_name, new_role));

                // 触发网络热重载
                interaction_clone.request_network_restart(app_config_clone.clone());
            }

            prev_role_clone.store(current_role_value, Ordering::Relaxed);

            // === OSC 接收处理：检查是否有从外部接收的参数变化 ===
            // B1 修复：分别处理每个参数，只在有变化时更新，避免返回未初始化的默认值
            if let Some(volume) = osc_state_clone.take_pending_volume() {
                setter.begin_set_parameter(&params.master_gain);
                setter.set_parameter(&params.master_gain, volume);
                setter.end_set_parameter(&params.master_gain);
                logger_clone.info("editor", &format!("[OSC Recv] Volume: {:.3}", volume));
            }

            if let Some(dim) = osc_state_clone.take_pending_dim() {
                setter.begin_set_parameter(&params.dim);
                setter.set_parameter(&params.dim, dim);
                setter.end_set_parameter(&params.dim);
                // 回显 OSC 状态
                osc_state_clone.send_dim(dim);
                logger_clone.info("editor", &format!("[OSC Recv] Dim: {}", dim));
            }

            if let Some(cut) = osc_state_clone.take_pending_cut() {
                setter.begin_set_parameter(&params.cut);
                setter.set_parameter(&params.cut, cut);
                setter.end_set_parameter(&params.cut);
                // 同步 Cut 状态（用于 toggle 支持）
                osc_state_clone.sync_cut_state(cut);
                // 回显 OSC 状态
                osc_state_clone.send_cut(cut);
                logger_clone.info("editor", &format!("[OSC Recv] Cut: {}", cut));
            }

            // === Slave 网络同步：检查是否有从 Master 接收的参数变化 ===
            let role = params.role.value();
            if role == PluginRole::Slave {
                // 检查并应用网络接收的 master_gain
                if let Some(gain) = interaction_clone.take_network_master_gain() {
                    setter.begin_set_parameter(&params.master_gain);
                    setter.set_parameter(&params.master_gain, gain);
                    setter.end_set_parameter(&params.master_gain);
                }

                // 检查并应用网络接收的 dim
                if let Some(dim) = interaction_clone.take_network_dim() {
                    setter.begin_set_parameter(&params.dim);
                    setter.set_parameter(&params.dim, dim);
                    setter.end_set_parameter(&params.dim);
                }

                // 检查并应用网络接收的 cut
                if let Some(cut) = interaction_clone.take_network_cut() {
                    setter.begin_set_parameter(&params.cut);
                    setter.set_parameter(&params.cut, cut);
                    setter.end_set_parameter(&params.cut);
                }

                // 检查并应用网络接收的布局索引
                if let Some(layout) = interaction_clone.take_network_layout() {
                    let current_layout = params.layout.value();
                    if layout != current_layout {
                        // 先清理交互状态，防止通道名错配
                        interaction_clone.clear_on_layout_change();
                        logger.clone().info("editor", &format!("[Slave] Layout sync from Master: {} -> {} (cleared state)", current_layout, layout));
                        setter.begin_set_parameter(&params.layout);
                        setter.set_parameter(&params.layout, layout);
                        setter.end_set_parameter(&params.layout);
                    }
                }

                // 检查并应用网络接收的 SUB 布局索引
                if let Some(sub_layout) = interaction_clone.take_network_sub_layout() {
                    let current_sub = params.sub_layout.value();
                    if sub_layout != current_sub {
                        // 先清理交互状态，防止通道名错配
                        interaction_clone.clear_on_layout_change();
                        logger.clone().info("editor", &format!("[Slave] Sub layout sync from Master: {} -> {} (cleared state)", current_sub, sub_layout));
                        setter.begin_set_parameter(&params.sub_layout);
                        setter.set_parameter(&params.sub_layout, sub_layout);
                        setter.end_set_parameter(&params.sub_layout);
                    }
                }
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
                            render_header(ui, &scale, params, setter, &interaction_clone, &layout_config_clone, &logger_clone);
                        });

                    // 左侧控制面板
                    SidePanel::left("sidebar")
                        .exact_width(scale.s(180.0))
                        .resizable(false)
                        .frame(panel_frame) // <-- Apply clean frame
                        .show(ctx, |ui| {
                            render_sidebar(ui, &scale, params, setter, &interaction_clone, &osc_state_clone, &layout_config_clone, &logger_clone);
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
                                    render_log_panel(ui, &scale, log_collapsed_id, &logger_clone);
                                });

                            CentralPanel::default()
                                .frame(Frame::new())
                                .show_inside(ui, |ui| {
                                    render_speaker_matrix(ui, &scale, params, setter, &interaction_clone, &osc_state_clone, &network_connected_clone, &layout_config_clone, &logger_clone);
                                });
                        });

                    // 设置弹窗 - 使用 Area 替代 Window 以获得完全控制
                    let dialog_id = egui::Id::new("settings_dialog");
                    let show_settings = ctx.memory(|m| m.data.get_temp::<bool>(dialog_id).unwrap_or(false));

                    if show_settings {
                        // 绘制半透明背景遮罩（增强模态感）
                        let screen_rect = ctx.screen_rect();
                        ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, dialog_id.with("overlay")))
                            .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));

                        egui::Area::new(dialog_id)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                let dialog_width = scale.s(500.0);
                                ui.set_width(dialog_width);
                                ui.set_max_width(dialog_width);

                                render_settings_content_v2(ui, &scale, dialog_id, params, setter, &interaction_clone, &layout_config_clone, &logger_clone, &app_config_clone, &osc_state_clone, &web_state_clone);
                            });

                        // Automation confirmation dialog (triggered from settings)
                        let confirm_id = egui::Id::new("automation_confirm_from_settings");
                        let show_confirm = ctx.memory(|m| m.data.get_temp::<bool>(confirm_id).unwrap_or(false));
                        if show_confirm {
                            egui::Window::new("Confirm Enable Automation")
                                .collapsible(false)
                                .resizable(false)
                                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                                .show(ctx, |ui| {
                                    ui.label("Enabling automation will clear current Solo/Mute settings.");
                                    ui.label("Continue?");
                                    ui.add_space(scale.s(12.0));
                                    ui.horizontal(|ui| {
                                        if ui.button("OK").clicked() {
                                            interaction_clone.enter_automation_mode();
                                            logger_clone.info("editor", "[AUTO] Enter: cleared all state, params unchanged (controlled by DAW)");
                                            ui.memory_mut(|m| m.data.remove::<bool>(confirm_id));
                                        }
                                        if ui.button("Cancel").clicked() {
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
fn sync_all_channel_params(params: &Arc<MonitorParams>, setter: &ParamSetter, interaction: &InteractionManager, layout_config: &ConfigManager, logger: &InstanceLogger) {
    // 获取当前布局信息
    let layout_idx = params.layout.value() as usize;
    let sub_idx = params.sub_layout.value() as usize;

    let speaker_layouts = layout_config.get_speaker_layouts();
    let sub_layouts = layout_config.get_sub_layouts();

    let speaker_name = speaker_layouts.get(layout_idx)
        .cloned()
        .unwrap_or_else(|| "7.1.4".to_string());
    let sub_name = sub_layouts.get(sub_idx)
        .cloned()
        .unwrap_or_else(|| "None".to_string());

    let layout = layout_config.get_layout(&speaker_name, &sub_name);

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
    logger.info("editor", &format!("[SYNC] {}ch: {}on/{}off mask=0x{:x}",
        layout.total_channels, on_count, off_count, on_mask));
}

/// 渲染顶部标题栏 - 参数绑定版
fn render_header(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, setter: &ParamSetter, interaction: &InteractionManager, layout_config: &ConfigManager, logger: &InstanceLogger) {
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
            let speaker_layouts = layout_config.get_speaker_layouts();
            let sub_layouts = layout_config.get_sub_layouts();

            // === 从参数系统读取当前值 ===
            let current_role = params.role.value();
            let current_layout_idx = params.layout.value() as usize;
            let current_sub_idx = params.sub_layout.value() as usize;

            // === 检查 Settings 窗口是否打开（模态行为）===
            let settings_open = ui.ctx().memory(|m| m.data.get_temp::<bool>(egui::Id::new("settings_dialog")).unwrap_or(false));

            // === 检查是否允许布局切换 ===
            let is_automation = interaction.is_automation_mode();
            let is_slave = current_role == PluginRole::Slave;
            let can_change_layout = !is_automation && !is_slave && !settings_open; // 自动化模式、Slave 模式或 Settings 打开时禁止切换布局
            let can_change_role = !settings_open; // Settings 打开时禁止切换 Role

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
                                        logger.info("editor", &format!("[Editor] Sub layout changed: {} -> {}", current_sub_name, name));
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
                                        logger.info("editor", &format!("[Editor] Speaker layout changed: {} -> {}", current_layout_name, name));
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

            // 3. Role dropdown (Plugin Role) - Settings 打开时禁用
            ui.add_enabled_ui(can_change_role, |ui| {
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
                                        logger.info("editor", &format!("[Editor] Role changed: {:?} -> {:?}", current_role, new_role));

                                        // 如果切换到 Master/Slave，自动退出自动化模式
                                        if new_role != PluginRole::Standalone {
                                            if interaction.is_automation_mode() {
                                                interaction.exit_automation_mode();
                                                logger.info("editor", &format!("[Editor] Auto-exited automation mode (switched to {:?})", new_role));
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
            });

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Role");

            ui.add_space(scale.s(12.0));

            // 齿轮设置按钮 (移到 Role 左边)
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
    let height = scale.s(46.0); // 双行按钮高度
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
fn render_sidebar(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, setter: &ParamSetter, interaction: &InteractionManager, osc_state: &Arc<OscSharedState>, layout_config: &ConfigManager, logger: &InstanceLogger) {

    // === 检查是否为 Slave 模式 ===
    let role = params.role.value();
    let is_slave = role == PluginRole::Slave;

    ui.add_space(scale.s(24.0));

    let sidebar_content_width = scale.s(180.0) - scale.s(32.0);

    ui.horizontal(|ui| {
        ui.add_space(scale.s(16.0));

        ui.vertical(|ui| {
            ui.set_max_width(sidebar_content_width);

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

            // Slave 模式下禁用 SOLO 按钮
            ui.add_enabled_ui(!is_slave, |ui| {
                if ui.add(solo_btn).clicked() {
                    let primary_before = interaction.get_primary();
                    let compare_before = interaction.get_compare();
                    interaction.on_solo_button_click();
                    logger.info("editor", &format!("[Editor] SOLO clicked: ({:?}, {:?}) -> ({:?}, {:?})",
                        primary_before, compare_before,
                        interaction.get_primary(), interaction.get_compare()));

                    // 同步所有通道的 enable 参数
                    sync_all_channel_params(params, setter, &interaction, layout_config, logger);

                    // 发送 OSC 模式状态
                    osc_state.send_mode_solo(interaction.is_solo_active());
                    if !interaction.is_mute_active() {
                        osc_state.send_mode_mute(false);
                    }

                    // 广播所有通道的 LED 状态（防止退出模式时 LED 状态不同步）
                    osc_state.broadcast_channel_states(interaction);
                }
            });

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

            // Slave 模式下禁用 MUTE 按钮
            ui.add_enabled_ui(!is_slave, |ui| {
                if ui.add(mute_btn).clicked() {
                    let primary_before = interaction.get_primary();
                    let compare_before = interaction.get_compare();
                    interaction.on_mute_button_click();
                    logger.info("editor", &format!("[Editor] MUTE clicked: ({:?}, {:?}) -> ({:?}, {:?})",
                        primary_before, compare_before,
                        interaction.get_primary(), interaction.get_compare()));

                    // 同步所有通道的 enable 参数
                    sync_all_channel_params(params, setter, &interaction, layout_config, logger);

                    // 发送 OSC 模式状态
                    osc_state.send_mode_mute(interaction.is_mute_active());
                    if !interaction.is_solo_active() {
                        osc_state.send_mode_solo(false);
                    }

                    // 广播所有通道的 LED 状态（防止退出模式时 LED 状态不同步）
                    osc_state.broadcast_channel_states(interaction);
                }
            });

            ui.add_space(scale.s(24.0));
            ui.separator();
            ui.add_space(scale.s(24.0));

            // Volume Knob Area - 绑定到 params.master_gain
            // Slave 模式下禁用 Volume 旋钮
            ui.add_enabled_ui(!is_slave, |ui| {
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
                        osc_state.send_master_volume(new_gain);
                    }

                    // 只在拖动结束时记录日志
                    if response.drag_stopped() {
                        logger.info("editor", &format!("[Editor] Master volume set to: {:.1}%", volume_percent));
                    }
                });
            });

            // --- FIX 2: Layout spacing ---
            // Manually draw the separator line for precise control over spacing.
            ui.add_space(scale.s(16.0)); // Space above the line
            let line_rect = ui.available_rect_before_wrap();
            ui.painter().hline(line_rect.x_range(), line_rect.top(), Stroke::new(1.0, COLOR_BORDER_LIGHT));
            ui.add_space(scale.s(16.0)); // Space below the line

            // DIM + CUT buttons - 绑定到 params
            // Slave 模式下禁用 DIM/CUT 按钮
            let button_width = (sidebar_content_width - scale.s(8.0)) / 2.0; // 减去中间间隙
            ui.add_enabled_ui(!is_slave, |ui| {
                ui.horizontal(|ui| {
                    // DIM 按钮
                    let dim_active = params.dim.value();
                    let dim_btn = BrutalistButton::new("DIM", scale)
                        .width(button_width)
                        .warning(true)
                        .active(dim_active);
                    if ui.add(dim_btn).clicked() {
                        let new_value = !dim_active;
                        logger.info("editor", &format!("[Editor] DIM toggled: {} -> {}", dim_active, new_value));
                        setter.begin_set_parameter(&params.dim);
                        setter.set_parameter(&params.dim, new_value);
                        setter.end_set_parameter(&params.dim);

                        // 发送 OSC
                        osc_state.send_dim(new_value);
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
                        logger.info("editor", &format!("[Editor] CUT toggled: {} -> {}", cut_active, new_value));
                        setter.begin_set_parameter(&params.cut);
                        setter.set_parameter(&params.cut, new_value);
                        setter.end_set_parameter(&params.cut);

                        // 同步 Cut 状态（用于 toggle 支持）
                        osc_state.sync_cut_state(new_value);

                        // 发送 OSC
                        osc_state.send_cut(new_value);
                    }
                });
            });

            // Second separator
            ui.add_space(scale.s(16.0));
            let line_rect_2 = ui.available_rect_before_wrap();
            ui.painter().hline(line_rect_2.x_range(), line_rect_2.top(), Stroke::new(1.0, COLOR_BORDER_LIGHT));
            ui.add_space(scale.s(16.0));

            // --- NEW: Low/High Boost Group ---
            // Slave 模式下禁用这些按钮
            ui.add_enabled_ui(!is_slave, |ui| {
                ui.horizontal(|ui| {
                    // Low Boost - 与硬件同步
                    let lb_active = osc_state.get_low_boost();
                    if custom_button(ui, "Low", "Boost", lb_active, button_width, scale).clicked() {
                        let new_value = !lb_active;
                        osc_state.set_low_boost(new_value);
                        osc_state.send_low_boost(new_value);
                    }

                    ui.add_space(scale.s(8.0));

                    // High Boost - 与硬件同步
                    let hb_active = osc_state.get_high_boost();
                    if custom_button(ui, "High", "Boost", hb_active, button_width, scale).clicked() {
                        let new_value = !hb_active;
                        osc_state.set_high_boost(new_value);
                        osc_state.send_high_boost(new_value);
                    }
                });
            });

            ui.add_space(scale.s(12.0));

            // --- NEW: MONO / +10dB LFE Group ---
            // Slave 模式下禁用这些按钮
            ui.add_enabled_ui(!is_slave, |ui| {
                ui.horizontal(|ui| {
                    // MONO Button - 与硬件同步
                    let mono_active = osc_state.get_mono();
                    let mut btn = BrutalistButton::new("MONO", scale)
                        .width(button_width)
                        .height(scale.s(46.0));  // 与 custom_button 高度一致
                    btn = btn.danger(true).active(mono_active);  // 红色样式与硬件一致
                    if ui.add(btn).clicked() {
                        let new_value = !mono_active;
                        osc_state.set_mono(new_value);
                        osc_state.send_mono(new_value);
                    }

                    ui.add_space(scale.s(8.0));

                    // +10dB LFE - 与硬件同步
                    let lfe_active = osc_state.get_lfe_add_10db();
                    if custom_button(ui, "+10dB", "LFE", lfe_active, button_width, scale).clicked() {
                        let new_value = !lfe_active;
                        osc_state.set_lfe_add_10db(new_value);
                        osc_state.send_lfe_add_10db(new_value);
                    }
                });
            });

            ui.add_space(scale.s(12.0));

            // --- NEW: Curve Button (Full Width) ---
            // Slave 模式下禁用 Curve 按钮
            ui.add_enabled_ui(!is_slave, |ui| {
                let curve_id = ui.id().with("curve_btn");
                let mut curve_active = ui.memory(|m| m.data.get_temp::<bool>(curve_id).unwrap_or(false));
                let mut curve_btn = BrutalistButton::new("Curve", scale).full_width(true); // Removed .large()
                curve_btn = curve_btn.active(curve_active);
                if ui.add(curve_btn).clicked() {
                    curve_active = !curve_active;
                    ui.memory_mut(|m| m.data.insert_temp(curve_id, curve_active));
                }
            });
        });

        ui.add_space(scale.s(16.0));
    });
}

/// 渲染音箱矩阵（新版：SUB 在上下轨道，整体居中）
fn render_speaker_matrix(ui: &mut egui::Ui, scale: &ScaleContext, params: &Arc<MonitorParams>, _setter: &ParamSetter, interaction: &InteractionManager, osc_state: &Arc<OscSharedState>, network_connected: &Arc<AtomicBool>, layout_config: &ConfigManager, logger: &InstanceLogger) {
    // 检查 Role 和模式状态
    let role = params.role.value();
    let is_slave = role == PluginRole::Slave;
    let is_automation = interaction.is_automation_mode();

    // Automation mode global indicator
    if is_automation {
        ui.horizontal(|ui| {
            ui.add_space(scale.s(16.0));
            ui.label(egui::RichText::new("Automation Active")
                .size(scale.s(14.0))
                .color(egui::Color32::from_rgb(251, 191, 36))); // Amber-400
            ui.label(egui::RichText::new("(Channel states controlled by VST3 parameters)")
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

    let speaker_layouts = layout_config.get_speaker_layouts();
    let sub_layouts = layout_config.get_sub_layouts();

    let speaker_name = speaker_layouts.get(layout_idx)
        .cloned()
        .unwrap_or_else(|| "7.1.4".to_string());
    let sub_name = sub_layouts.get(sub_idx)
        .cloned()
        .unwrap_or_else(|| "None".to_string());

    let layout = layout_config.get_layout(&speaker_name, &sub_name);

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
            render_sub_row_dynamic(ui, scale, &layout, 1..=3, sub_diameter, main_grid_width, params, _setter, interaction, osc_state, layout_config, logger);
        });

        ui.add_space(sub_spacing);

        // 主网格
        render_main_grid_dynamic(ui, scale, &layout, box_size, grid_spacing, label_height, params, _setter, interaction, osc_state, layout_config, logger);

        ui.add_space(sub_spacing);

        // 下方 SUB 行
        ui.horizontal(|ui| {
            let padding = (available_width - main_grid_width) / 2.0;
            ui.add_space(padding.max(0.0));
            render_sub_row_dynamic(ui, scale, &layout, 4..=6, sub_diameter, main_grid_width, params, _setter, interaction, osc_state, layout_config, logger);
        });
    });

    // === Slave 模式：在音箱矩阵上方绘制半透明灰色遮罩 ===
    if is_slave {
        let overlay_rect = rect;

        // 半透明灰色遮罩
        ui.painter().rect_filled(
            overlay_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(100, 116, 139, 180)
        );

        // 获取连接状态
        let connected = network_connected.load(Ordering::Relaxed);
        let status_text = if connected { "Connected" } else { "Connecting..." };
        let status_color = if connected {
            egui::Color32::from_rgb(34, 197, 94)   // 绿色
        } else {
            egui::Color32::from_rgb(251, 191, 36)  // 黄色
        };

        // 绘制居中状态文字
        let galley = ui.painter().layout_no_wrap(
            status_text.to_string(),
            scale.font(28.0),
            status_color
        );
        let text_pos = overlay_rect.center() - galley.rect.size() / 2.0;
        ui.painter().galley(text_pos, galley, status_color);

        // 绘制 Slave 模式标签
        let label_galley = ui.painter().layout_no_wrap(
            "Slave Mode".to_string(),
            scale.font(14.0),
            egui::Color32::from_rgb(226, 232, 240)  // 浅灰色
        );
        let label_pos = egui::pos2(
            overlay_rect.center().x - label_galley.rect.width() / 2.0,
            overlay_rect.center().y + scale.s(30.0)
        );
        ui.painter().galley(label_pos, label_galley, egui::Color32::from_rgb(226, 232, 240));
    }
}

/// 渲染日志面板
fn render_log_panel(ui: &mut egui::Ui, scale: &ScaleContext, collapse_id: egui::Id, logger: &Arc<InstanceLogger>) {
    let is_collapsed = ui.data(|d| d.get_temp::<bool>(collapse_id).unwrap_or(false));
    let rect = ui.max_rect();

    // 顶部边框线
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(scale.s(1.0), COLOR_BORDER_MEDIUM)
    );

    // 标题栏
    let header_height = scale.s(28.0);
    ui.allocate_ui(Vec2::new(ui.available_width(), header_height), |ui| {
        let header_rect = ui.max_rect();
        ui.painter().rect_filled(header_rect, 0.0, COLOR_BG_SIDEBAR);

        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(scale.s(1.0), COLOR_BORDER_LIGHT)
        );

        ui.horizontal(|ui| {
            ui.add_space(scale.s(12.0));

            // 标题
            ui.vertical(|ui| {
                ui.add_space(scale.s(4.0));
                ui.label(RichText::new("EVENT LOG").font(scale.mono_font(10.0)).color(COLOR_TEXT_MEDIUM));
            });

            // 右上角折叠/释放按钮
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(scale.s(8.0));

                let (btn_text, btn_hover) = if is_collapsed {
                    ("Show", "Expand Log")
                } else {
                    ("Hide", "Collapse Log")
                };

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
        let content_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            content_rect,
            0.0,
            Color32::from_rgb(230, 235, 240)
        );

        // 获取最近的日志条目
        let logs = logger.get_recent_logs();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add_space(scale.s(4.0));

                if logs.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(scale.s(12.0));
                        ui.label(RichText::new("-- No events logged --")
                            .font(scale.mono_font(10.0))
                            .color(COLOR_TEXT_LIGHT));
                    });
                } else {
                    // 只显示最后几条日志（根据可用空间）
                    let max_display = 5;
                    let start_idx = if logs.len() > max_display { logs.len() - max_display } else { 0 };

                    for log_entry in logs.iter().skip(start_idx) {
                        ui.horizontal(|ui| {
                            ui.add_space(scale.s(8.0));
                            ui.label(RichText::new(log_entry)
                                .font(scale.mono_font(9.0))
                                .color(COLOR_TEXT_MEDIUM));
                        });
                    }
                }

                ui.add_space(scale.s(4.0));
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
    layout: &crate::Config_Manager::Layout,
    pos_range: std::ops::RangeInclusive<u32>,
    sub_diameter: f32,
    container_width: f32,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter,
    interaction: &InteractionManager,
    osc_state: &Arc<OscSharedState>,
    layout_config: &ConfigManager,
    logger: &InstanceLogger,
) {
    let is_automation = interaction.is_automation_mode();
    let is_slave = params.role.value() == PluginRole::Slave;

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

            // 点击处理（仅手动模式，非 Slave）
            if response.clicked() && !is_automation && !is_slave {
                // 使用相对索引进行双击检测（保持一致性）
                let click_type = interaction.detect_sub_click(sub_relative_idx);
                match click_type {
                    SubClickType::SingleClick => {
                        // on_channel_click 使用通道名称
                        interaction.on_channel_click(&ch.name);
                        logger.info("editor", &format!("[Editor] SUB {} ({}) single click", sub_relative_idx, ch.name));
                    }
                    SubClickType::DoubleClick => {
                        // on_sub_double_click 使用通道名称
                        interaction.on_sub_double_click(&ch.name);
                        logger.info("editor", &format!("[Editor] SUB {} ({}) double click -> Mute toggle", sub_relative_idx, ch.name));
                    }
                }

                // 全通道同步（Solo/Mute 操作会影响所有通道的 has_sound 状态）
                sync_all_channel_params(params, setter, interaction, layout_config, logger);

                // 发送 OSC 所有通道 LED 状态（三态）
                osc_state.broadcast_channel_states(interaction);
            }

            // 右键：SUB 的 User Mute 反转（替代双击）（仅手动模式，非 Slave）
            if response.secondary_clicked() && !is_automation && !is_slave {
                // on_sub_double_click 使用通道名称
                interaction.on_sub_double_click(&ch.name);
                logger.info("editor", &format!("[Editor] SUB {} ({}) right-click -> Mute toggle", sub_relative_idx, ch.name));

                // 全通道同步（SUB Mute 操作可能影响整体状态）
                sync_all_channel_params(params, setter, interaction, layout_config, logger);

                // 发送 OSC 所有通道 LED 状态（三态）
                osc_state.broadcast_channel_states(interaction);
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
    layout: &crate::Config_Manager::Layout,
    box_size: f32,
    grid_spacing: f32,
    label_height: f32,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter,
    interaction: &InteractionManager,
    osc_state: &Arc<OscSharedState>,
    layout_config: &ConfigManager,
    logger: &InstanceLogger,
) {
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
                                let is_automation = interaction.is_automation_mode();
                                let is_slave = params.role.value() == PluginRole::Slave;

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

                                // 点击处理（仅手动模式，非 Slave）
                                if response.clicked() && !is_automation && !is_slave {
                                    interaction.on_channel_click(&ch.name);
                                    logger.info("editor", &format!("[Editor] Main {} ({}) clicked", ch_idx, ch.name));

                                    // 全通道同步（Solo/Mute 操作会影响所有通道的 has_sound 状态）
                                    sync_all_channel_params(params, setter, interaction, layout_config, logger);

                                    // 发送 OSC 所有通道 LED 状态（三态）
                                    osc_state.broadcast_channel_states(interaction);
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

/// iOS-style toggle switch for Settings dialog
fn settings_toggle_switch(ui: &mut egui::Ui, on: &mut bool, scale: &ScaleContext) -> egui::Response {
    let size = Vec2::new(scale.s(44.0), scale.s(24.0));
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
    }

    if ui.is_rect_visible(rect) {
        let bg = if *on { SETTINGS_SLATE_800 } else { SETTINGS_SLATE_200 };
        let rounding = scale.s(12.0);
        ui.painter().rect_filled(rect, rounding, bg);

        // Animate knob position
        let target_t = if *on { 1.0 } else { 0.0 };
        let t = ui.ctx().animate_value_with_time(response.id, target_t, 0.15);

        let knob_radius = scale.s(8.0);
        let knob_margin = scale.s(4.0);
        let knob_x = egui::lerp(
            rect.left() + knob_margin + knob_radius..=rect.right() - knob_margin - knob_radius,
            t
        );
        let knob_center = egui::pos2(knob_x, rect.center().y);
        ui.painter().circle_filled(knob_center, knob_radius, Color32::WHITE);
    }

    response
}

/// Settings dialog state for editable fields
#[derive(Clone)]
struct SettingsState {
    osc_send_port: String,
    osc_receive_port: String,
    network_port: String,
    master_ip: String,
    dirty: bool,  // Whether settings have been modified
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            osc_send_port: "7444".to_string(),
            osc_receive_port: "7445".to_string(),
            network_port: "9123".to_string(),
            master_ip: "127.0.0.1".to_string(),
            dirty: false,
        }
    }
}

/// Render settings window content (Legacy - kept for reference)
#[allow(dead_code)]
fn render_settings_content(
    ui: &mut egui::Ui,
    scale: &ScaleContext,
    dialog_id: egui::Id,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter,
    interaction: &InteractionManager,
    layout_config: &ConfigManager,
    logger: &InstanceLogger,
    app_config: &AppConfig,
    _osc_state: &Arc<OscSharedState>,
) {
    let state_id = egui::Id::new("settings_state");

    // Load or initialize settings state
    let mut state = ui.memory(|m| {
        m.data.get_temp::<SettingsState>(state_id).unwrap_or_else(|| {
            SettingsState {
                osc_send_port: app_config.osc_send_port.to_string(),
                osc_receive_port: app_config.osc_receive_port.to_string(),
                network_port: app_config.network_port.to_string(),
                master_ip: app_config.master_ip.clone(),
                dirty: false,
            }
        })
    });

    ui.add_space(scale.s(8.0));

    // ========== Automation Mode ==========
    ui.heading(RichText::new("Automation Mode").font(scale.font(16.0)));
    ui.add_space(scale.s(12.0));

    let role = params.role.value();
    let is_automation = interaction.is_automation_mode();
    let can_use_automation = role == crate::Params::PluginRole::Standalone;

    ui.add_enabled_ui(can_use_automation, |ui| {
        let button_text = if is_automation { "Exit Automation" } else { "Enable Automation" };
        let auto_btn = BrutalistButton::new(button_text, scale)
            .full_width(true)
            .active(is_automation);

        if ui.add(auto_btn).clicked() {
            if is_automation {
                interaction.exit_automation_mode();
                logger.info("editor", "[AUTO] Exit: idle state, will sync to all=On on next UI update");
                sync_all_channel_params(params, setter, &interaction, layout_config, logger);
            } else {
                let confirm_id = egui::Id::new("automation_confirm_from_settings");
                ui.memory_mut(|m| m.data.insert_temp(confirm_id, true));
            }
        }
    });

    if !can_use_automation {
        ui.label(egui::RichText::new("(Standalone only)")
            .size(scale.s(9.0))
            .color(egui::Color32::from_rgb(156, 163, 175)));
    }

    ui.add_space(scale.s(16.0));
    ui.separator();
    ui.add_space(scale.s(16.0));

    // ========== OSC Settings ==========
    ui.heading(RichText::new("OSC (Hardware Controller)").font(scale.font(16.0)));
    ui.label(egui::RichText::new("Communication with TouchOSC, X-Touch, etc.")
        .size(scale.s(10.0))
        .color(egui::Color32::from_rgb(156, 163, 175)));
    ui.add_space(scale.s(12.0));

    let label_width = scale.s(100.0);
    let field_width = scale.s(120.0);

    // Send Port
    ui.horizontal(|ui| {
        ui.add_sized([label_width, scale.s(20.0)],
            egui::Label::new(RichText::new("Send Port:").font(scale.font(14.0))));
        let response = ui.add_sized([field_width, scale.s(24.0)],
            egui::TextEdit::singleline(&mut state.osc_send_port)
                .font(scale.font(14.0)));
        if response.changed() {
            state.dirty = true;
        }
        // Poll keyboard when TextEdit has focus
        if response.has_focus() {
            if crate::Keyboard_Polling::poll_and_apply(&mut state.osc_send_port) {
                state.dirty = true;
            }
        }
    });

    ui.add_space(scale.s(8.0));

    // Receive Port
    ui.horizontal(|ui| {
        ui.add_sized([label_width, scale.s(20.0)],
            egui::Label::new(RichText::new("Receive Port:").font(scale.font(14.0))));
        let response = ui.add_sized([field_width, scale.s(24.0)],
            egui::TextEdit::singleline(&mut state.osc_receive_port)
                .font(scale.font(14.0)));
        if response.changed() {
            state.dirty = true;
        }
        // Poll keyboard when TextEdit has focus
        if response.has_focus() {
            if crate::Keyboard_Polling::poll_and_apply(&mut state.osc_receive_port) {
                state.dirty = true;
            }
        }
    });

    ui.add_space(scale.s(16.0));
    ui.separator();
    ui.add_space(scale.s(16.0));

    // ========== Network Settings ==========
    ui.heading(RichText::new("Network (Instance Sync)").font(scale.font(16.0)));
    ui.label(egui::RichText::new("Master/Slave communication between plugin instances")
        .size(scale.s(10.0))
        .color(egui::Color32::from_rgb(156, 163, 175)));
    ui.add_space(scale.s(12.0));

    // Master IP - 仅 Slave 模式可编辑
    let is_slave = role == crate::Params::PluginRole::Slave;
    ui.horizontal(|ui| {
        ui.add_sized([label_width, scale.s(20.0)],
            egui::Label::new(RichText::new("Master IP:").font(scale.font(14.0))));

        ui.add_enabled_ui(is_slave, |ui| {
            let response = ui.add_sized([field_width, scale.s(24.0)],
                egui::TextEdit::singleline(&mut state.master_ip)
                    .font(scale.font(14.0)));
            if response.changed() {
                state.dirty = true;
            }
            // Poll keyboard when TextEdit has focus
            if response.has_focus() {
                if crate::Keyboard_Polling::poll_and_apply(&mut state.master_ip) {
                    state.dirty = true;
                }
            }
        });

        // 非 Slave 模式显示提示
        if !is_slave {
            ui.label(egui::RichText::new("(Slave only)")
                .size(scale.s(9.0))
                .color(egui::Color32::from_rgb(156, 163, 175)));
        }
    });

    ui.add_space(scale.s(8.0));

    // Network Port
    ui.horizontal(|ui| {
        ui.add_sized([label_width, scale.s(20.0)],
            egui::Label::new(RichText::new("Port:").font(scale.font(14.0))));
        let response = ui.add_sized([field_width, scale.s(24.0)],
            egui::TextEdit::singleline(&mut state.network_port)
                .font(scale.font(14.0)));
        if response.changed() {
            state.dirty = true;
        }
        // Poll keyboard when TextEdit has focus
        if response.has_focus() {
            if crate::Keyboard_Polling::poll_and_apply(&mut state.network_port) {
                state.dirty = true;
            }
        }
    });

    ui.add_space(scale.s(8.0));
    ui.label(egui::RichText::new("Note: Network changes require plugin restart")
        .size(scale.s(10.0))
        .color(egui::Color32::from_rgb(156, 163, 175)));

    ui.add_space(scale.s(16.0));
    ui.separator();
    ui.add_space(scale.s(16.0));

    // ========== Config Path ==========
    let config_path = crate::Config_File::AppConfig::config_path();
    let path_str = config_path.display().to_string();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Config:").font(scale.font(12.0)).color(egui::Color32::from_rgb(156, 163, 175)));
        ui.add_space(scale.s(4.0));
        // Truncate path if too long
        let display_path = if path_str.len() > 40 {
            format!("...{}", &path_str[path_str.len()-37..])
        } else {
            path_str.clone()
        };
        ui.label(RichText::new(display_path).font(scale.font(10.0)).color(egui::Color32::from_rgb(156, 163, 175)));
    });

    ui.add_space(scale.s(4.0));

    // Open folder button
    if ui.button(RichText::new("Open Config Folder").font(scale.font(12.0))).clicked() {
        if let Some(parent) = config_path.parent() {
            let _ = open::that(parent);
        }
    }

    ui.add_space(scale.s(16.0));
    ui.separator();
    ui.add_space(scale.s(16.0));

    // ========== Buttons ==========
    ui.horizontal(|ui| {
        // Save button
        let save_btn = ui.add_enabled(state.dirty,
            egui::Button::new(RichText::new("Save").font(scale.font(14.0))));

        if save_btn.clicked() {
            // Parse and validate
            let osc_send: u16 = state.osc_send_port.parse().unwrap_or(7444);
            let osc_recv: u16 = state.osc_receive_port.parse().unwrap_or(7445);
            let net_port: u16 = state.network_port.parse().unwrap_or(9123);
            let master_ip = state.master_ip.clone();

            // Create new config
            let new_config = crate::Config_File::AppConfig {
                osc_send_port: osc_send,
                osc_receive_port: osc_recv,
                network_port: net_port,
                master_ip: master_ip.clone(),
                default_speaker_layout: app_config.default_speaker_layout.clone(),
                default_sub_layout: app_config.default_sub_layout.clone(),
                log_directory: app_config.log_directory.clone(),
            };

            // Save to disk
            match new_config.save_to_disk() {
                Ok(_) => {
                    logger.info("editor", &format!(
                        "[Settings] Saved: osc_send={}, osc_recv={}, net_port={}, master_ip={}",
                        osc_send, osc_recv, net_port, master_ip
                    ));
                    state.dirty = false;

                    // Trigger OSC hot reload with new config
                    interaction.request_osc_restart(new_config.clone());

                    // Trigger Network hot reload (only for Master/Slave modes)
                    if role != crate::Params::PluginRole::Standalone {
                        interaction.request_network_restart(new_config.clone());
                    }

                    // Auto-close window after save
                    ui.memory_mut(|m| {
                        m.data.remove::<SettingsState>(state_id);
                        m.data.remove::<bool>(dialog_id);
                    });
                    logger.info("editor", "[Settings] Saved and closed");
                }
                Err(e) => {
                    logger.error("editor", &format!("[Settings] Save failed: {}", e));
                }
            }
        }

        ui.add_space(scale.s(16.0));

        // Close button
        if ui.button(RichText::new("Close").font(scale.font(14.0))).clicked() {
            logger.info("editor", "[Settings] Closed");
            ui.memory_mut(|m| {
                m.data.remove::<SettingsState>(state_id);
                m.data.remove::<bool>(dialog_id);
            });
        }
    });

    // Save state back to memory
    ui.memory_mut(|m| m.data.insert_temp(state_id, state));

    ui.add_space(scale.s(8.0));
}

/// Render settings window content v2 - New Brutalist design
fn render_settings_content_v2(
    ui: &mut egui::Ui,
    scale: &ScaleContext,
    dialog_id: egui::Id,
    params: &Arc<MonitorParams>,
    setter: &ParamSetter,
    interaction: &InteractionManager,
    layout_config: &ConfigManager,
    logger: &InstanceLogger,
    app_config: &AppConfig,
    _osc_state: &Arc<OscSharedState>,
    web_state: &Arc<WebSharedState>,
) {
    let state_id = egui::Id::new("settings_state_v2");

    // Load or initialize settings state
    let mut state = ui.memory(|m| {
        m.data.get_temp::<SettingsState>(state_id).unwrap_or_else(|| {
            SettingsState {
                osc_send_port: app_config.osc_send_port.to_string(),
                osc_receive_port: app_config.osc_receive_port.to_string(),
                network_port: app_config.network_port.to_string(),
                master_ip: app_config.master_ip.clone(),
                dirty: false,
            }
        })
    });

    let role = params.role.value();
    let is_automation = interaction.is_automation_mode();
    let can_use_automation = role == crate::Params::PluginRole::Standalone;
    let is_slave = role == crate::Params::PluginRole::Slave;

    // Darker text colors for better readability
    let text_dark = Color32::from_rgb(30, 41, 59);      // slate-800
    let text_medium = Color32::from_rgb(71, 85, 105);   // slate-600
    let text_label = Color32::from_rgb(100, 116, 139);  // slate-500
    let border_color = Color32::from_rgb(203, 213, 225); // slate-300

    // ========== 获取内容区域并绘制阴影/背景/边框 ==========
    let content_rect = ui.max_rect();

    // 1. 绘制阴影（偏移的深色矩形）
    let shadow_offset = scale.s(6.0);
    let shadow_rect = content_rect.translate(Vec2::new(shadow_offset, shadow_offset));
    ui.painter().rect_filled(shadow_rect, scale.s(4.0), Color32::from_black_alpha(40));

    // 2. 绘制白色背景
    ui.painter().rect_filled(content_rect, 0.0, Color32::WHITE);

    // 3. 绘制边框
    ui.painter().rect_stroke(content_rect, 0.0, Stroke::new(scale.s(1.0), SETTINGS_SLATE_300), StrokeKind::Inside);

    // ========== HEADER ==========
    let header_height = scale.s(48.0);
    let header_rect = egui::Rect::from_min_max(
        content_rect.min,
        egui::pos2(content_rect.max.x, content_rect.min.y + header_height)
    );
    ui.painter().rect_filled(header_rect, 0.0, SETTINGS_SLATE_50);

    // Header bottom border
    ui.painter().line_segment(
        [header_rect.left_bottom(), header_rect.right_bottom()],
        Stroke::new(scale.s(1.0), SETTINGS_SLATE_200)
    );

    ui.allocate_ui(Vec2::new(content_rect.width(), header_height), |ui| {
        // 使用 Align::Center 让所有元素垂直居中
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add_space(scale.s(16.0));

            // Gear icon + SETTINGS title
            ui.label(RichText::new("⚙").font(scale.font(18.0)).color(text_medium).strong());
            ui.add_space(scale.s(8.0));
            ui.label(RichText::new("SETTINGS").font(scale.font(16.0)).color(text_dark).strong());

            // Right-aligned close button (X)
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(scale.s(16.0));

                // Custom X button with visible rendering
                let btn_size = Vec2::splat(scale.s(28.0));
                let (rect, response) = ui.allocate_exact_size(btn_size, egui::Sense::click());

                if ui.is_rect_visible(rect) {
                    let is_hovered = response.hovered();
                    let color = if is_hovered { SETTINGS_RED_500 } else { text_label };

                    // Draw X using lines
                    let margin = scale.s(8.0);
                    let stroke = Stroke::new(scale.s(2.0), color);
                    ui.painter().line_segment(
                        [rect.min + Vec2::splat(margin), rect.max - Vec2::splat(margin)],
                        stroke
                    );
                    ui.painter().line_segment(
                        [egui::pos2(rect.max.x - margin, rect.min.y + margin),
                         egui::pos2(rect.min.x + margin, rect.max.y - margin)],
                        stroke
                    );
                }

                if response.clicked() {
                    logger.info("editor", "[Settings] Closed via X button");
                    ui.memory_mut(|m| {
                        m.data.remove::<SettingsState>(state_id);
                        m.data.remove::<bool>(dialog_id);
                    });
                }
            });
        });
    });

    // ========== BODY ==========
    ui.add_space(scale.s(24.0));

    ui.horizontal(|ui| {
        ui.add_space(scale.s(24.0));

        ui.vertical(|ui| {
            ui.set_width(scale.s(500.0) - scale.s(48.0));

            // === Section A: Automation Mode ===
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⊞").font(scale.font(13.0)).color(text_medium));
                        ui.add_space(scale.s(6.0));
                        ui.label(RichText::new("AUTOMATION MODE").font(scale.font(12.0)).color(text_dark).strong());
                    });
                    ui.add_space(scale.s(4.0));
                    ui.label(RichText::new("Enable external automation control | Standalone only")
                        .font(scale.font(12.0)).color(text_label));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut automation_on = is_automation;
                    ui.add_enabled_ui(can_use_automation, |ui| {
                        let response = settings_toggle_switch(ui, &mut automation_on, scale);
                        if response.clicked() && can_use_automation {
                            if automation_on {
                                let confirm_id = egui::Id::new("automation_confirm_from_settings");
                                ui.memory_mut(|m| m.data.insert_temp(confirm_id, true));
                            } else {
                                interaction.exit_automation_mode();
                                logger.info("editor", "[AUTO] Exit: idle state");
                                sync_all_channel_params(params, setter, interaction, layout_config, logger);
                            }
                        }
                    });
                });
            });

            ui.add_space(scale.s(28.0));

            // === Section B: OSC (Hardware Controller) ===
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔊").font(scale.font(13.0)).color(text_medium));
                ui.add_space(scale.s(6.0));
                ui.label(RichText::new("OSC (HARDWARE CONTROLLER)").font(scale.font(12.0)).color(text_dark).strong());
            });
            ui.add_space(scale.s(12.0));

            // Two-column layout for ports with proper borders
            ui.horizontal(|ui| {
                let col_width = (scale.s(500.0) - scale.s(48.0) - scale.s(24.0)) / 2.0;

                // Send Port column
                ui.vertical(|ui| {
                    ui.set_width(col_width);
                    ui.label(RichText::new("Send Port").font(scale.font(12.0)).color(text_medium));
                    ui.add_space(scale.s(6.0));

                    // Custom bordered input
                    let input_height = scale.s(40.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(col_width, input_height), egui::Sense::hover());

                    // Draw border
                    ui.painter().rect_stroke(rect, scale.s(4.0), Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);
                    ui.painter().rect_filled(rect.shrink(scale.s(1.0)), scale.s(3.0), Color32::WHITE);

                    // Put input inside
                    ui.allocate_ui_at_rect(rect.shrink(scale.s(8.0)), |ui| {
                        ui.centered_and_justified(|ui| {
                            let response = ui.add(egui::TextEdit::singleline(&mut state.osc_send_port)
                                .font(scale.font(14.0))
                                .frame(false)
                                .text_color(text_dark));
                            if response.changed() {
                                state.dirty = true;
                            }
                            if response.has_focus() {
                                if crate::Keyboard_Polling::poll_and_apply(&mut state.osc_send_port) {
                                    state.dirty = true;
                                }
                            }
                        });
                    });
                });

                ui.add_space(scale.s(24.0));

                // Receive Port column
                ui.vertical(|ui| {
                    ui.set_width(col_width);
                    ui.label(RichText::new("Receive Port").font(scale.font(12.0)).color(text_medium));
                    ui.add_space(scale.s(6.0));

                    // Custom bordered input
                    let input_height = scale.s(40.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(col_width, input_height), egui::Sense::hover());

                    // Draw border
                    ui.painter().rect_stroke(rect, scale.s(4.0), Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);
                    ui.painter().rect_filled(rect.shrink(scale.s(1.0)), scale.s(3.0), Color32::WHITE);

                    // Put input inside
                    ui.allocate_ui_at_rect(rect.shrink(scale.s(8.0)), |ui| {
                        ui.centered_and_justified(|ui| {
                            let response = ui.add(egui::TextEdit::singleline(&mut state.osc_receive_port)
                                .font(scale.font(14.0))
                                .frame(false)
                                .text_color(text_dark));
                            if response.changed() {
                                state.dirty = true;
                            }
                            if response.has_focus() {
                                if crate::Keyboard_Polling::poll_and_apply(&mut state.osc_receive_port) {
                                    state.dirty = true;
                                }
                            }
                        });
                    });
                });
            });

            ui.add_space(scale.s(28.0));

            // === Section C: Network (Instance Sync) ===
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔗").font(scale.font(13.0)).color(text_medium));
                ui.add_space(scale.s(6.0));
                ui.label(RichText::new("NETWORK (INSTANCE SYNC)").font(scale.font(12.0)).color(text_dark).strong());
            });
            ui.add_space(scale.s(12.0));

            // Bordered container for network settings
            // 使用"先布局内容，后绘制背景"的方式，让高度自动适应
            let container_width = scale.s(500.0) - scale.s(48.0);
            let start_pos = ui.cursor().min;

            // 内容布局
            ui.add_space(scale.s(12.0));  // 上边距
            ui.horizontal(|ui| {
                ui.add_space(scale.s(16.0));  // 左边距
                ui.vertical(|ui| {
                    let label_width = scale.s(80.0);
                    let field_width = container_width - scale.s(32.0) - label_width - scale.s(16.0);

                    // Master IP row
                    ui.horizontal(|ui| {
                        // Right-aligned label
                        ui.allocate_ui(Vec2::new(label_width, scale.s(36.0)), |ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(scale.s(8.0));
                                ui.label(RichText::new("Master IP").font(scale.font(12.0)).color(text_medium));
                            });
                        });

                        ui.add_space(scale.s(8.0));

                        // Input field with border
                        let input_height = scale.s(36.0);
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(field_width, input_height), egui::Sense::hover());

                        let input_bg = if is_slave { Color32::WHITE } else { SETTINGS_SLATE_100 };
                        ui.painter().rect_stroke(rect, scale.s(4.0), Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);
                        ui.painter().rect_filled(rect.shrink(scale.s(1.0)), scale.s(3.0), input_bg);

                        ui.allocate_ui_at_rect(rect.shrink(scale.s(10.0)), |ui| {
                            ui.add_enabled_ui(is_slave, |ui| {
                                ui.centered_and_justified(|ui| {
                                    let response = ui.add(egui::TextEdit::singleline(&mut state.master_ip)
                                        .font(scale.font(14.0))
                                        .frame(false)
                                        .text_color(text_dark));
                                    if response.changed() {
                                        state.dirty = true;
                                    }
                                    if response.has_focus() {
                                        if crate::Keyboard_Polling::poll_and_apply(&mut state.master_ip) {
                                            state.dirty = true;
                                        }
                                    }
                                });
                            });
                        });
                    });

                    ui.add_space(scale.s(8.0));

                    // Port row
                    ui.horizontal(|ui| {
                        // Right-aligned label
                        ui.allocate_ui(Vec2::new(label_width, scale.s(36.0)), |ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(scale.s(8.0));
                                ui.label(RichText::new("Port").font(scale.font(12.0)).color(text_medium));
                            });
                        });

                        ui.add_space(scale.s(8.0));

                        // Input field with border
                        let input_height = scale.s(36.0);
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(field_width, input_height), egui::Sense::hover());

                        ui.painter().rect_stroke(rect, scale.s(4.0), Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);
                        ui.painter().rect_filled(rect.shrink(scale.s(1.0)), scale.s(3.0), Color32::WHITE);

                        ui.allocate_ui_at_rect(rect.shrink(scale.s(10.0)), |ui| {
                            ui.centered_and_justified(|ui| {
                                let response = ui.add(egui::TextEdit::singleline(&mut state.network_port)
                                    .font(scale.font(14.0))
                                    .frame(false)
                                    .text_color(text_dark));
                                if response.changed() {
                                    state.dirty = true;
                                }
                                if response.has_focus() {
                                    if crate::Keyboard_Polling::poll_and_apply(&mut state.network_port) {
                                        state.dirty = true;
                                    }
                                }
                            });
                        });
                    });
                });
            });
            ui.add_space(scale.s(12.0));  // 下边距

            // 计算内容实际占用的矩形，只绘制边框（背景已经在前面预绘制了）
            let end_pos = ui.cursor().min;
            let actual_rect = egui::Rect::from_min_max(
                egui::pos2(start_pos.x, start_pos.y),
                egui::pos2(start_pos.x + container_width, end_pos.y)
            );
            // 只绘制边框（不绘制背景，避免覆盖输入框）
            ui.painter().rect_stroke(actual_rect, scale.s(4.0), Stroke::new(scale.s(1.0), SETTINGS_SLATE_200), StrokeKind::Inside);

            // Warning note
            ui.add_space(scale.s(6.0));
            ui.label(RichText::new("* Network changes require plugin restart")
                .font(scale.font(11.0)).color(SETTINGS_AMBER_600));

            ui.add_space(scale.s(16.0));

            // === Section D: Config Path ===
            let config_path = crate::Config_File::AppConfig::config_path();
            let path_str = config_path.display().to_string();
            let display_path = if path_str.len() > 40 {
                format!("...{}", &path_str[path_str.len()-37..])
            } else {
                path_str.clone()
            };

            // 使用固定高度的行，让两侧元素垂直居中
            let row_height = scale.s(28.0);
            ui.allocate_ui(Vec2::new(ui.available_width(), row_height), |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Config: {}", display_path))
                        .font(scale.font(11.0)).color(text_label));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Styled OPEN FOLDER button
                        let btn_text = "📁 OPEN FOLDER";
                        let btn_size = Vec2::new(scale.s(110.0), scale.s(28.0));
                        let (rect, response) = ui.allocate_exact_size(btn_size, egui::Sense::click());

                        if ui.is_rect_visible(rect) {
                            let bg = if response.hovered() { SETTINGS_SLATE_100 } else { Color32::WHITE };
                            ui.painter().rect_filled(rect, scale.s(4.0), bg);
                            ui.painter().rect_stroke(rect, scale.s(4.0), Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);

                            let galley = ui.painter().layout_no_wrap(
                                btn_text.to_string(),
                                scale.font(11.0),
                                text_medium
                            );
                            let text_pos = rect.center() - galley.rect.size() / 2.0;
                            ui.painter().galley(text_pos, galley, text_medium);
                        }

                        if response.clicked() {
                            if let Some(parent) = config_path.parent() {
                                let _ = open::that(parent);
                            }
                        }
                    });
                });
            });

            ui.add_space(scale.s(28.0));

            // === Section E: Web Controller ===
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🌐").font(scale.font(13.0)).color(text_medium));
                        ui.add_space(scale.s(6.0));
                        ui.label(RichText::new("WEB CONTROLLER").font(scale.font(12.0)).color(text_dark).strong());
                    });
                    ui.add_space(scale.s(4.0));

                    // 显示当前状态：运行中时显示访问地址，否则显示说明
                    let web_running = web_state.is_running();
                    if web_running {
                        if let Some(addr) = web_state.get_address() {
                            ui.label(RichText::new(format!("Running at: http://{}", addr))
                                .font(scale.font(12.0)).color(Color32::from_rgb(16, 185, 129))); // green
                        }
                    } else {
                        ui.label(RichText::new("Control plugin from phone/tablet browser")
                            .font(scale.font(12.0)).color(text_label));
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let web_running = web_state.is_running();
                    let mut toggle_state = web_running;

                    // Slave 模式下禁用 Web 控制器
                    ui.add_enabled_ui(!is_slave, |ui| {
                        let response = settings_toggle_switch(ui, &mut toggle_state, scale);
                        if response.clicked() && !is_slave {
                            if toggle_state {
                                // 请求启动 Web 服务器
                                interaction.request_web_restart(WebRestartAction::Start);
                                logger.info("editor", "[Web] Requesting start");
                            } else {
                                // 请求停止 Web 服务器
                                interaction.request_web_restart(WebRestartAction::Stop);
                                logger.info("editor", "[Web] Requesting stop");
                            }
                        }
                    });
                });
            });

            if is_slave {
                ui.add_space(scale.s(4.0));
                ui.label(RichText::new("(Disabled in Slave mode)")
                    .font(scale.font(10.0)).color(SETTINGS_AMBER_600));
            }
        });

        ui.add_space(scale.s(24.0));
    });

    ui.add_space(scale.s(24.0));

    // ========== FOOTER ==========
    let footer_height = scale.s(72.0);

    // Footer 使用 content_rect 的宽度
    let current_y = ui.available_rect_before_wrap().top();

    // Footer top border
    ui.painter().line_segment(
        [egui::pos2(content_rect.min.x, current_y), egui::pos2(content_rect.max.x, current_y)],
        Stroke::new(scale.s(1.0), SETTINGS_SLATE_200)
    );

    // Footer background
    let footer_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.min.x, current_y),
        egui::pos2(content_rect.max.x, current_y + footer_height)
    );
    ui.painter().rect_filled(footer_rect, 0.0, SETTINGS_SLATE_50);

    // Footer bottom border
    ui.painter().line_segment(
        [footer_rect.left_bottom(), footer_rect.right_bottom()],
        Stroke::new(scale.s(1.0), SETTINGS_SLATE_200)
    );

    ui.allocate_ui(Vec2::new(ui.available_width(), footer_height), |ui| {
        // 使用 Align::Center 让按钮在 footer 区域垂直居中
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add_space(scale.s(20.0));

            // CLOSE button (left) - white background with border
            let close_size = Vec2::new(scale.s(90.0), scale.s(36.0));
            let (close_rect, close_response) = ui.allocate_exact_size(close_size, egui::Sense::click());

            if ui.is_rect_visible(close_rect) {
                let bg = if close_response.hovered() { SETTINGS_SLATE_100 } else { Color32::WHITE };
                ui.painter().rect_filled(close_rect, scale.s(4.0), bg);
                ui.painter().rect_stroke(close_rect, scale.s(4.0), Stroke::new(scale.s(1.0), border_color), StrokeKind::Inside);

                let galley = ui.painter().layout_no_wrap(
                    "CLOSE".to_string(),
                    scale.font(13.0),
                    text_medium
                );
                let text_pos = close_rect.center() - galley.rect.size() / 2.0;
                ui.painter().galley(text_pos, galley, text_medium);
            }

            if close_response.clicked() {
                logger.info("editor", "[Settings] Closed");
                ui.memory_mut(|m| {
                    m.data.remove::<SettingsState>(state_id);
                    m.data.remove::<bool>(dialog_id);
                });
            }

            // SAVE button (right)
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(scale.s(20.0));

                let save_size = Vec2::new(scale.s(90.0), scale.s(36.0));
                let (save_rect, save_response) = ui.allocate_exact_size(save_size, egui::Sense::click());

                let save_enabled = state.dirty;
                if ui.is_rect_visible(save_rect) {
                    let bg = if save_enabled {
                        if save_response.hovered() { SETTINGS_SLATE_700 } else { SETTINGS_SLATE_800 }
                    } else {
                        SETTINGS_SLATE_300
                    };
                    ui.painter().rect_filled(save_rect, scale.s(4.0), bg);

                    let galley = ui.painter().layout_no_wrap(
                        "SAVE".to_string(),
                        scale.font(13.0),
                        Color32::WHITE
                    );
                    let text_pos = save_rect.center() - galley.rect.size() / 2.0;
                    ui.painter().galley(text_pos, galley, Color32::WHITE);
                }

                if save_response.clicked() && save_enabled {
                    // Parse and validate
                    let osc_send: u16 = state.osc_send_port.parse().unwrap_or(7444);
                    let osc_recv: u16 = state.osc_receive_port.parse().unwrap_or(7445);
                    let net_port: u16 = state.network_port.parse().unwrap_or(9123);
                    let master_ip = state.master_ip.clone();

                    // Create new config
                    let new_config = crate::Config_File::AppConfig {
                        osc_send_port: osc_send,
                        osc_receive_port: osc_recv,
                        network_port: net_port,
                        master_ip: master_ip.clone(),
                        default_speaker_layout: app_config.default_speaker_layout.clone(),
                        default_sub_layout: app_config.default_sub_layout.clone(),
                        log_directory: app_config.log_directory.clone(),
                    };

                    // Save to disk
                    match new_config.save_to_disk() {
                        Ok(_) => {
                            logger.info("editor", &format!(
                                "[Settings] Saved: osc_send={}, osc_recv={}, net_port={}, master_ip={}",
                                osc_send, osc_recv, net_port, master_ip
                            ));
                            state.dirty = false;

                            // Trigger OSC hot reload with new config
                            interaction.request_osc_restart(new_config.clone());

                            // Trigger Network hot reload (only for Master/Slave modes)
                            if role != crate::Params::PluginRole::Standalone {
                                interaction.request_network_restart(new_config.clone());
                            }

                            // Auto-close window after save
                            ui.memory_mut(|m| {
                                m.data.remove::<SettingsState>(state_id);
                                m.data.remove::<bool>(dialog_id);
                            });
                            logger.info("editor", "[Settings] Saved and closed");
                        }
                        Err(e) => {
                            logger.error("editor", &format!("[Settings] Save failed: {}", e));
                        }
                    }
                }
            });
        });
    });

    // Save state back to memory
    ui.memory_mut(|m| m.data.insert_temp(state_id, state));
}