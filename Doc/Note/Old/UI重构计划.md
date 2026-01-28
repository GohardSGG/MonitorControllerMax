# UI 重构计划：面板系统与布局优化

## 目标概述

重构 MonitorControllerMax 的 UI 架构，实现：
1. 支持 egui 原生面板系统的 ResizableWindow
2. **3:4 竖屏纵横比，默认尺寸 600×800**
3. 简化的缩放系统（不再到处传 scale_factor）
4. 正确的边框和颜色 - **rgb(30, 41, 59)** 深色边框
5. 音箱矩阵居中
6. **可折叠的日志面板**（默认隐藏）

## 用户确认的配置

| 配置项 | 选择 |
|--------|------|
| 默认窗口尺寸 | 600 × 800 (3:4) |
| 侧边栏位置 | 左侧 |
| 日志面板 | 可折叠（默认隐藏）|
| 标题栏下拉选择 | Role + Format 移到标题栏右侧 |
| 关闭按钮 | 移除 |

---

## 第一阶段：重构 ResizableWindow

### 修改文件
`Library/nih-plug/nih_plug_egui/src/resizable_window.rs`

### 核心改动

将 API 从：
```rust
pub fn show<R>(
    self,
    context: &Context,
    egui_state: &EguiState,
    add_contents: impl FnOnce(&mut Ui) -> R,  // 用户拿到 Ui
) -> InnerResponse<R>
```

改为：
```rust
pub fn show<R>(
    self,
    context: &Context,
    egui_state: &EguiState,
    add_contents: impl FnOnce(&Context) -> R,  // 用户拿到 Context
) -> R
```

### 实现细节

```rust
pub fn show<R>(
    self,
    context: &Context,
    egui_state: &EguiState,
    add_contents: impl FnOnce(&Context) -> R,
) -> R {
    // 1. 先让用户添加他们的面板（在 CentralPanel 之前）
    let ret = add_contents(context);

    // 2. 用 Area 在右下角绘制悬浮的 resize handle
    let screen_rect = context.screen_rect();
    let corner_size = 16.0;
    let corner_pos = screen_rect.max - egui::vec2(corner_size, corner_size);

    egui::Area::new(self.id.with("resize_corner"))
        .fixed_pos(corner_pos)
        .order(egui::Order::Foreground)  // 确保在最上层
        .show(context, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                Vec2::splat(corner_size),
                Sense::drag()
            );

            if let Some(pointer_pos) = response.interact_pointer_pos() {
                // 计算新尺寸（复用现有的纵横比逻辑）
                let raw_desired_size = pointer_pos - screen_rect.min;
                let desired_size = self.apply_aspect_ratio(raw_desired_size);

                if response.dragged() {
                    egui_state.set_requested_size((
                        desired_size.x.round() as u32,
                        desired_size.y.round() as u32,
                    ));
                }
            }

            // 绘制 resize corner 图案
            paint_resize_corner(ui, &response);
        });

    ret
}
```

---

## 第二阶段：创建缩放上下文系统

### 新建文件
`Source/Plugin/Src/scale.rs`

### 实现

```rust
use nih_plug_egui::egui::{self, Context, Vec2, FontId, Margin};

/// 全局缩放上下文，避免到处传 scale_factor
pub struct ScaleContext {
    pub factor: f32,
}

impl ScaleContext {
    pub fn new(ctx: &Context, base_width: f32) -> Self {
        // 从 screen_rect 计算缩放因子
        let current_width = ctx.screen_rect().width();
        let factor = (current_width / base_width).clamp(0.5, 4.0);
        Self { factor }
    }

    /// 缩放单个值
    #[inline]
    pub fn s(&self, val: f32) -> f32 {
        val * self.factor
    }

    /// 缩放 Vec2
    #[inline]
    pub fn vec2(&self, x: f32, y: f32) -> Vec2 {
        Vec2::new(x * self.factor, y * self.factor)
    }

    /// 创建缩放后的字体
    #[inline]
    pub fn font(&self, size: f32) -> FontId {
        FontId::proportional(size * self.factor)
    }

    /// 创建缩放后的等宽字体
    #[inline]
    pub fn mono_font(&self, size: f32) -> FontId {
        FontId::monospace(size * self.factor)
    }

    /// 应用缩放到 egui Context
    pub fn apply_to_context(&self, ctx: &Context) {
        ctx.set_pixels_per_point(self.factor);
    }
}
```

---

## 第三阶段：重构 Editor.rs

### 修改文件
`Source/Plugin/Src/Editor.rs`

### 常量更新

```rust
const BASE_WIDTH: f32 = 600.0;   // 3:4 比例
const BASE_HEIGHT: f32 = 800.0;  // 竖屏
const ASPECT_RATIO: f32 = BASE_WIDTH / BASE_HEIGHT; // 0.75

// 颜色常量
const COLOR_BORDER_MAIN: Color32 = Color32::from_rgb(30, 41, 59);  // 主边框颜色（深灰蓝）
```

### 新的布局结构

```rust
pub fn create_editor(params: Arc<MonitorParams>) -> Option<Box<dyn Editor>> {
    // ... 初始化代码 ...

    create_egui_editor(
        egui_state,
        (),
        |_, _| {},
        move |ctx, _setter, _state| {
            // 1. 创建缩放上下文
            let scale = ScaleContext::new(ctx, BASE_WIDTH);
            scale.apply_to_context(ctx);

            // 2. 绘制最外层边框
            let screen = ctx.screen_rect();
            ctx.layer_painter(LayerId::background())
                .rect_stroke(screen, 0.0, Stroke::new(scale.s(2.0), COLOR_BORDER_MAIN));

            // 3. 使用面板系统
            ResizableWindow::new("main")
                .with_aspect_ratio(ASPECT_RATIO)
                .show(ctx, &egui_state_clone, |ctx| {

                    // 顶部标题栏（包含下拉选择）
                    TopBottomPanel::top("header")
                        .exact_height(scale.s(40.0))
                        .frame(Frame::none().fill(Color32::WHITE))
                        .show(ctx, |ui| {
                            render_header(ui, &scale);
                        });

                    // 可折叠的底部日志面板（默认隐藏）
                    let log_expanded_id = ctx.data(|d| d.get_temp::<bool>(Id::new("log_expanded")).unwrap_or(false));
                    if log_expanded_id {
                        TopBottomPanel::bottom("log_panel")
                            .exact_height(scale.s(120.0))
                            .show(ctx, |ui| {
                                render_log_panel(ui, &scale, true);  // true = 显示折叠按钮
                            });
                    }

                    // 左侧控制面板
                    SidePanel::left("sidebar")
                        .exact_width(scale.s(180.0))
                        .resizable(false)
                        .show(ctx, |ui| {
                            render_sidebar(ui, &scale);
                        });

                    // 中央内容区域（音箱矩阵）
                    CentralPanel::default().show(ctx, |ui| {
                        render_speaker_matrix(ui, &scale);
                    });
                });
        },
    )
}
```

### 标题栏实现（含下拉选择）

```rust
fn render_header(ui: &mut Ui, scale: &ScaleContext) {
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
```

### 音箱矩阵居中实现

```rust
fn render_speaker_matrix(ui: &mut Ui, scale: &ScaleContext) {
    // 绘制背景网格
    let rect = ui.max_rect();
    draw_grid_background(ui, rect, scale);

    // 使用 centered_and_justified 确保居中
    ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
        ui.vertical_centered(|ui| {
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
```

---

## 第四阶段：更新 Components.rs

### 修改文件
`Source/Plugin/Src/Components.rs`

### 改动要点

1. 所有组件改为接收 `&ScaleContext` 而不是 `scale_factor: f32`
2. 内部使用 `scale.s()` 方法

```rust
pub struct SpeakerBox<'a> {
    name: &'a str,
    active: bool,
    is_sub: bool,
    scale: &'a ScaleContext,  // 改为引用 ScaleContext
    label: Option<&'a str>,
}

impl<'a> SpeakerBox<'a> {
    pub fn new(name: &'a str, active: bool, scale: &'a ScaleContext) -> Self {
        Self {
            name,
            active,
            is_sub: name.contains("SUB") || name == "LFE",
            scale,
            label: None,
        }
    }
}

impl<'a> Widget for SpeakerBox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let s = self.scale;
        let box_size = if self.is_sub {
            s.vec2(70.0, 70.0)
        } else {
            s.vec2(85.0, 85.0)
        };
        // ... 其余实现使用 s.s() 替代 * scale_factor
    }
}
```

---

## 实施顺序

1. **resizable_window.rs** - 修改 API 支持面板系统
2. **scale.rs** - 创建缩放上下文
3. **Components.rs** - 更新组件接口
4. **Editor.rs** - 重构布局代码
5. **测试验证**

---

## 关键文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `Library/nih-plug/nih_plug_egui/src/resizable_window.rs` | 修改 | 核心 API 改动 |
| `Library/nih-plug/nih_plug_egui/src/lib.rs` | 检查 | 确认 set_requested_size 是 pub(crate) |
| `Source/Plugin/Src/scale.rs` | 新建 | 缩放上下文 |
| `Source/Plugin/Src/Components.rs` | 修改 | 组件接口更新 |
| `Source/Plugin/Src/Editor.rs` | 重写 | 布局重构 |
| `Source/Plugin/Src/Lib.rs` | 修改 | 添加 mod scale |
