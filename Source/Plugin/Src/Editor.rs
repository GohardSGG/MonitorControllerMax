#![allow(non_snake_case)]

use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, EguiState, resizable_window::ResizableWindow};
use nih_plug_egui::egui::{
    self, Visuals, Vec2, Color32, Layout, Align, RichText, ComboBox,
    Stroke, LayerId, Frame, TopBottomPanel, SidePanel, CentralPanel, Grid, StrokeKind
};
use std::sync::Arc;
use crate::Params::MonitorParams;
use crate::Components::*;
use crate::scale::ScaleContext;

// --- 窗口尺寸常量 (1:1 正方形) ---
const BASE_WIDTH: f32 = 720.0;
const BASE_HEIGHT: f32 = 720.0;
const ASPECT_RATIO: f32 = 1.0;

// --- 颜色常量 ---
const COLOR_BORDER_MAIN: Color32 = Color32::from_rgb(30, 41, 59);  // 主边框颜色（深灰蓝）

pub fn create_editor(params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    let egui_state = EguiState::from_size(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    let egui_state_clone = egui_state.clone();

    let _params_clone = params.clone();

    create_egui_editor(
        egui_state,
        (),
        |_, _| {},
        move |ctx, _setter, _state| {
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
                            render_header(ui, &scale);
                        });

                    // 左侧控制面板
                    SidePanel::left("sidebar")
                        .exact_width(scale.s(180.0))
                        .resizable(false)
                        .frame(panel_frame) // <-- Apply clean frame
                        .show(ctx, |ui| {
                            render_sidebar(ui, &scale);
                        });

                    // 中央内容区域（音箱矩阵 + 日志面板）
                    CentralPanel::default()
                        .frame(central_frame) // <-- Apply clean frame
                        .show(ctx, |ui| {
                            // 子面板区域：上方音箱矩阵，下方日志
                            TopBottomPanel::bottom("log_panel")
                                .exact_height(scale.s(120.0))
                                .frame(Frame::new())
                                .show_inside(ui, |ui| {
                                    render_log_panel(ui, &scale);
                                });

                            CentralPanel::default()
                                .frame(Frame::new())
                                .show_inside(ui, |ui| {
                                    render_speaker_matrix(ui, &scale);
                                });
                        });
                });
        },
    )
}

/// 渲染顶部标题栏 - 手动精细校准版 (Scheme B)
fn render_header(ui: &mut egui::Ui, scale: &ScaleContext) {
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

            // --- Helper: 带微调偏移的 Dropdown ---
            // 使用 allocate_ui 分配固定空间，彻底防止布局重叠
            let mut dropdown_render = |ui: &mut egui::Ui, id: &str, width: f32, current_val: &mut usize, options: &[&str]| {
                // 1. 定义容器尺寸：宽度由参数决定，高度占满 Header (40.0)
                let box_size = Vec2::new(width, scale.s(40.0));
                
                ui.allocate_ui(box_size, |ui| {
                    // 2. 内部垂直布局 (Top-Down)
                    ui.set_min_width(width);
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        // 3. 计算居中 Padding
                        // 估算 ComboBox 高度约 20.0 (包含边框可能略多，这里主要控制视觉重心)
                        let estimated_combo_height = scale.s(20.0);
                        let base_padding = (box_size.y - estimated_combo_height) / 2.0;
                        
                        // 4. 应用 Padding + 用户微调偏移
                        let final_padding = base_padding + dropdown_y_offset;
                        if final_padding > 0.0 {
                            ui.add_space(final_padding);
                        }

                        ComboBox::from_id_salt(id)
                            .selected_text(RichText::new(options[*current_val]).font(combo_font.clone()))
                            .width(width)
                            .show_ui(ui, |ui| {
                                for (i, opt) in options.iter().enumerate() {
                                    if ui.selectable_label(*current_val == i, RichText::new(*opt).font(combo_font.clone())).clicked() {
                                        *current_val = i;
                                        ui.memory_mut(|mem| mem.data.insert_temp(egui::Id::new(id), *current_val));
                                    }
                                }
                            });
                    });
                });
            };

            // 1. Subs dropdown (First in Right-to-Left layout = Last Visually)
            let subs_id_str = "subs_select_combo";
            let subs_id = ui.id().with(subs_id_str);
            let mut selected_subs = ui.memory(|mem| mem.data.get_temp::<usize>(subs_id).unwrap_or(0));
            let subs_options = ["None", "Mono", "Stereo", "LCR"];

            dropdown_render(ui, subs_id_str, scale.s(80.0), &mut selected_subs, &subs_options);
            
            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Sub");
            ui.add_space(scale.s(12.0));

            // 2. Maps dropdown (Middle)
            let format_id_str = "channel_format_combo";
            let format_id = ui.id().with(format_id_str);
            let mut selected_format = ui.memory(|mem| mem.data.get_temp::<usize>(format_id).unwrap_or(1));
            let formats = ["Stereo", "5.1", "7.1", "7.1.4"];

            dropdown_render(ui, format_id_str, scale.s(80.0), &mut selected_format, &formats);

            ui.add_space(scale.s(2.0));
            label_with_offset(ui, "Map");
            ui.add_space(scale.s(12.0));

            // 3. Role dropdown (Last in Right-to-Left layout = First Visually)
            let role_id_str = "role_select_combo";
            let role_id = ui.id().with(role_id_str);
            let mut selected_role = ui.memory(|mem| mem.data.get_temp::<usize>(role_id).unwrap_or(0));
            let roles = ["Standalone", "Master", "Slave"];
            
            dropdown_render(ui, role_id_str, scale.s(100.0), &mut selected_role, &roles);
            
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

/// 渲染左侧控制面板
fn render_sidebar(ui: &mut egui::Ui, scale: &ScaleContext) {
    
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

            // Volume Knob Area（使用 memory 持久化值）
            ui.vertical_centered(|ui| {
                let volume_id = ui.id().with("main_volume");
                let mut volume_val = ui.memory(|mem| mem.data.get_temp::<f32>(volume_id).unwrap_or(8.0));
                let response = ui.add(TechVolumeKnob::new(&mut volume_val, scale));
                if response.changed() {
                    ui.memory_mut(|mem| mem.data.insert_temp(volume_id, volume_val));
                }
            });

            // --- FIX 2: Layout spacing ---
            // Manually draw the separator line for precise control over spacing.
            ui.add_space(scale.s(16.0)); // Space above the line
            let line_rect = ui.available_rect_before_wrap();
            ui.painter().hline(line_rect.x_range(), line_rect.top(), Stroke::new(1.0, COLOR_BORDER_LIGHT));
            ui.add_space(scale.s(16.0)); // Space below the line

            // DIM + CUT buttons
            let button_width = (sidebar_content_width - scale.s(8.0)) / 2.0; // 减去中间间隙
            ui.horizontal(|ui| {
                ui.add(BrutalistButton::new("DIM", scale).width(button_width));
                ui.add_space(scale.s(8.0));
                // --- FIX 3: Button label change ---
                ui.add(BrutalistButton::new("CUT", scale).width(button_width).danger(true));
            });

            // Second separator
            ui.add_space(scale.s(16.0));
            let line_rect_2 = ui.available_rect_before_wrap();
            ui.painter().hline(line_rect_2.x_range(), line_rect_2.top(), Stroke::new(1.0, COLOR_BORDER_LIGHT));
            ui.add_space(scale.s(16.0));
            
            // Bottom Group
            ui.add(BrutalistButton::new("EFFECT", scale).full_width(true));
        });

        ui.add_space(scale.s(16.0));
    });
}

/// 渲染音箱矩阵（居中显示）
fn render_speaker_matrix(ui: &mut egui::Ui, scale: &ScaleContext) {
    // 绘制背景网格
    let rect = ui.max_rect();
    draw_grid_background(ui, rect, scale);

    // 计算矩阵尺寸以实现居中
    let box_size = scale.s(96.0);      // 最大的盒子尺寸
    let spacing_x = scale.s(48.0);
    let spacing_y = scale.s(40.0);
    let label_height = scale.s(20.0);  // 底部标签高度

    // 矩阵总宽度 = 3个盒子 + 2个间距
    let matrix_width = box_size * 3.0 + spacing_x * 2.0;
    // 矩阵总高度 = 3行盒子 + 2个间距 + 标签
    let matrix_height = (box_size + label_height) * 3.0 + spacing_y * 2.0;

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

            let spacing = scale.vec2(48.0, 40.0);
            Grid::new("speaker_matrix")
                .spacing(spacing)
                .show(ui, |ui| {
                    // Row 1: L C R
                    ui.add(SpeakerBox::new("L", true, scale));
                    ui.add(SpeakerBox::new("C", true, scale));
                    ui.add(SpeakerBox::new("R", true, scale));
                    ui.end_row();

                    // Row 2: SUB-L LFE SUB-R
                    ui.add(SpeakerBox::new("SUB L", false, scale));
                    ui.add(SpeakerBox::new("LFE", true, scale));
                    ui.add(SpeakerBox::new("SUB R", false, scale));
                    ui.end_row();

                    // Row 3: LR SUB RR
                    ui.add(SpeakerBox::new("LR", true, scale).with_label("CH 7"));
                    ui.add(SpeakerBox::new("SUB", false, scale).with_label("AUX"));
                    ui.add(SpeakerBox::new("RR", true, scale).with_label("CH 8"));
                    ui.end_row();
                });
        });
    });
}

/// 渲染日志面板
fn render_log_panel(ui: &mut egui::Ui, scale: &ScaleContext) {
    let rect = ui.max_rect();

    // 顶部边框线
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(scale.s(1.0), COLOR_BORDER_MEDIUM)
    );

    // 标题栏
    let header_height = scale.s(24.0);
    ui.allocate_ui(Vec2::new(ui.available_width(), header_height), |ui| {
        let header_rect = ui.max_rect();
        ui.painter().rect_filled(header_rect, 0.0, COLOR_BG_SIDEBAR);

        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            Stroke::new(scale.s(1.0), COLOR_BORDER_LIGHT)
        );

        ui.horizontal(|ui| {
            ui.add_space(scale.s(12.0));
            ui.label(RichText::new("EVENT LOG").font(scale.mono_font(10.0)).color(COLOR_TEXT_MEDIUM));
        });
    });

    // 日志内容区域
    ui.painter().rect_filled(
        ui.available_rect_before_wrap(),
        0.0,
        Color32::from_rgb(248, 250, 252) // 极浅灰
    );

    ui.vertical(|ui| {
        ui.add_space(scale.s(8.0));
        ui.horizontal(|ui| {
            ui.add_space(scale.s(12.0));
            ui.label(RichText::new("-- No events logged --").font(scale.mono_font(10.0)).color(COLOR_TEXT_LIGHT));
        });
    });
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