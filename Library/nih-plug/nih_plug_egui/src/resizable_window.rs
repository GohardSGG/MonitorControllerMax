//! Resizable window wrapper for Egui editor.

use egui_baseview::egui::emath::GuiRounding;
// use egui_baseview::egui::UiBuilder;

use crate::egui::{pos2, Area, Context, Id, Order, Response, Sense, Ui, Vec2};
use crate::EguiState;

/// Adds a corner to the plugin window that can be dragged in order to resize it.
/// Resizing happens through plugin API, hence a custom implementation is needed.
pub struct ResizableWindow {
    id: Id,
    min_size: Vec2,
    /// If set, the window will maintain this aspect ratio (width / height) when resizing.
    aspect_ratio: Option<f32>,
}

impl ResizableWindow {
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(id_source),
            min_size: Vec2::splat(16.0),
            aspect_ratio: None,
        }
    }

    /// Won't shrink to smaller than this
    #[inline]
    pub fn min_size(mut self, min_size: impl Into<Vec2>) -> Self {
        self.min_size = min_size.into();
        self
    }

    /// Lock the window to a fixed aspect ratio (width / height).
    /// When the user drags the resize corner, the window will maintain this ratio.
    #[inline]
    pub fn with_aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }

    /// Apply aspect ratio constraint to a raw size
    fn apply_aspect_ratio(&self, raw_size: Vec2) -> Vec2 {
        if let Some(ratio) = self.aspect_ratio {
            // Calculate what width and height "would be" if we used each as the basis
            let w_based_h = raw_size.x / ratio; // height if we use width as basis
            let h_based_w = raw_size.y * ratio; // width if we use height as basis

            // Pick the larger resulting size (so dragging in any direction = enlarging)
            if raw_size.x >= h_based_w {
                // Width is the driving dimension
                Vec2::new(raw_size.x, w_based_h)
            } else {
                // Height is the driving dimension
                Vec2::new(h_based_w, raw_size.y)
            }
            .max(self.min_size)
        } else {
            raw_size.max(self.min_size)
        }
    }

    /// Show the resizable window. The closure receives the Context instead of a Ui,
    /// allowing the use of egui's panel system (TopBottomPanel, SidePanel, CentralPanel).
    pub fn show<R>(
        self,
        context: &Context,
        egui_state: &EguiState,
        add_contents: impl FnOnce(&Context) -> R,
    ) -> R {
        // 1. Let the user add their panels first
        let ret = add_contents(context);

        // 2. Draw the floating resize handle in the bottom-right corner using Area
        let screen_rect = context.content_rect();
        let corner_size = 16.0;
        let corner_pos = screen_rect.max - Vec2::splat(corner_size);

        Area::new(self.id.with("resize_corner"))
            .fixed_pos(corner_pos)
            .order(Order::Foreground) // Ensure it's on top
            .show(context, |ui| {
                let (_rect, response) =
                    ui.allocate_exact_size(Vec2::splat(corner_size), Sense::drag());

                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    // Calculate new size (reuse existing aspect ratio logic)
                    let raw_desired_size = pointer_pos - screen_rect.min;
                    let desired_size = self.apply_aspect_ratio(raw_desired_size);

                    if response.dragged() {
                        egui_state.set_requested_size((
                            desired_size.x.round() as u32,
                            desired_size.y.round() as u32,
                        ));
                    }
                }

                // Draw the resize corner pattern
                paint_resize_corner(ui, &response);
            });

        ret
    }
}

pub fn paint_resize_corner(ui: &Ui, response: &Response) {
    let stroke = ui.style().interact(response).fg_stroke;

    let painter = ui.painter();
    let rect = response.rect.translate(-Vec2::splat(2.0)); // move away from the corner
    let cp = rect.max.round_to_pixels(painter.pixels_per_point());

    let mut w = 2.0;

    while w <= rect.width() && w <= rect.height() {
        painter.line_segment([pos2(cp.x - w, cp.y), pos2(cp.x, cp.y - w)], stroke);
        w += 4.0;
    }
}
