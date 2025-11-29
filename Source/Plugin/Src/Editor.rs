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

// --- 窗口尺寸常量 (3:4 竖屏比例) ---
const BASE_WIDTH: f32 = 600.0;
const BASE_HEIGHT: f32 = 800.0;
const ASPECT_RATIO: f32 = BASE_WIDTH / BASE_HEIGHT; // 0.75

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

            // 3. 绘制最外层边框
            let screen = ctx.screen_rect();
            ctx.layer_painter(LayerId::background())
                .rect_stroke(screen, 0.0, Stroke::new(scale.s(2.0), COLOR_BORDER_MAIN), StrokeKind::Outside);

            // 4. 使用 ResizableWindow 和面板系统
            ResizableWindow::new("main")
                .with_aspect_ratio(ASPECT_RATIO)
                .show(ctx, &egui_state_clone, |ctx| {
                    // 顶部标题栏（包含下拉选择）
                    TopBottomPanel::top("header")
                        .exact_height(scale.s(40.0))
                        .frame(Frame::new().fill(Color32::WHITE))
                        .show(ctx, |ui| {
                            render_header(ui, &scale);
                        });

                    // 左侧控制面板
                    SidePanel::left("sidebar")
                        .exact_width(scale.s(180.0))
                        .resizable(false)
                        .frame(Frame::new().fill(COLOR_BG_SIDEBAR))
                        .show(ctx, |ui| {
                            render_sidebar(ui, &scale);
                        });

                    // 中央内容区域（音箱矩阵 + 日志面板）
                    CentralPanel::default()
                        .frame(Frame::new().fill(COLOR_BG_MAIN))
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

/// 渲染顶部标题栏（包含 Logo、版本、下拉选择）
fn render_header(ui: &mut egui::Ui, scale: &ScaleContext) {
    ui.horizontal_centered(|ui| {
        ui.add_space(scale.s(16.0));

        // Logo
        ui.label(RichText::new("Monitor").font(scale.font(16.0)).color(COLOR_TEXT_DARK));
        ui.label(RichText::new("ControllerMax").font(scale.font(16.0)).strong().color(COLOR_TEXT_DARK));

        ui.add_space(scale.s(8.0));

        // 版本标签
        ui.label(RichText::new("v5.0.0").font(scale.mono_font(10.0)).color(COLOR_TEXT_MEDIUM));

        // 右侧下拉选择
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(scale.s(16.0));

            // 通道格式选择
            let format_id = ui.id().with("channel_format");
            let mut selected_format = ui.memory(|mem| mem.data.get_temp::<usize>(format_id).unwrap_or(1));
            let formats = ["Stereo", "5.1", "7.1", "7.1.4"];
            ComboBox::from_id_salt("format")
                .selected_text(formats[selected_format])
                .width(scale.s(70.0))
                .show_ui(ui, |ui| {
                    for (i, format) in formats.iter().enumerate() {
                        if ui.selectable_value(&mut selected_format, i, *format).changed() {
                            ui.memory_mut(|mem| mem.data.insert_temp(format_id, selected_format));
                        }
                    }
                });

            ui.add_space(scale.s(8.0));

            // 角色选择
            let role_id = ui.id().with("role_select");
            let mut selected_role = ui.memory(|mem| mem.data.get_temp::<usize>(role_id).unwrap_or(0));
            let roles = ["Standalone", "Master", "Slave"];
            ui.label(RichText::new("Role:").font(scale.mono_font(10.0)).color(COLOR_TEXT_LIGHT));
            ComboBox::from_id_salt("role")
                .selected_text(roles[selected_role])
                .width(scale.s(90.0))
                .show_ui(ui, |ui| {
                    for (i, role) in roles.iter().enumerate() {
                        if ui.selectable_value(&mut selected_role, i, *role).changed() {
                            ui.memory_mut(|mem| mem.data.insert_temp(role_id, selected_role));
                        }
                    }
                });
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
    // 右侧边框
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [rect.right_top(), rect.right_bottom()],
        Stroke::new(scale.s(1.0), COLOR_BORDER_MEDIUM)
    );

    ui.add_space(scale.s(24.0));

    // 内容区域（带左右padding）
    ui.horizontal(|ui| {
        ui.add_space(scale.s(16.0)); // 左侧padding

        ui.vertical(|ui| {
            ui.set_max_width(scale.s(180.0) - scale.s(32.0)); // 减去左右padding

            // Group 1: Solo/Mute
            ui.add(BrutalistButton::new("SOLO", scale).large().full_width(true));
            ui.add_space(scale.s(12.0));
            ui.add(BrutalistButton::new("MUTE", scale).large().danger(true).full_width(true));

            ui.add_space(scale.s(24.0));
            ui.separator();
            ui.add_space(scale.s(24.0));

            // Volume Knob Area
            ui.vertical_centered(|ui| {
                let mut dummy_val = 8.0;
                ui.add(TechVolumeKnob::new(&mut dummy_val, scale));
            });

            ui.add_space(scale.s(16.0));
            ui.add(BrutalistButton::new("DIM", scale).full_width(true));

            ui.add_space(scale.s(24.0));
            ui.separator();
            ui.add_space(scale.s(24.0));

            // Bottom Group
            ui.add(BrutalistButton::new("M. MUTE", scale).danger(true).full_width(true));
            ui.add_space(scale.s(12.0));
            ui.add(BrutalistButton::new("EFFECT", scale).full_width(true));
        });

        ui.add_space(scale.s(16.0)); // 右侧padding
    });
}

/// 渲染音箱矩阵（居中显示）
fn render_speaker_matrix(ui: &mut egui::Ui, scale: &ScaleContext) {
    // 绘制背景网格
    let rect = ui.max_rect();
    draw_grid_background(ui, rect, scale);

    // 使用居中布局
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(scale.s(40.0)); // 顶部padding

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
