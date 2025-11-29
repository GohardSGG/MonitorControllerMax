#![allow(non_snake_case)]

use nih_plug_egui::egui::{Vec2, FontId};

/// 全局缩放上下文，避免到处传 scale_factor
///
/// 重要：使用物理像素尺寸计算缩放因子，而不是 ctx.screen_rect()
/// 因为 screen_rect() 会被 pixels_per_point 影响，导致循环依赖
pub struct ScaleContext {
    pub factor: f32,
}

impl ScaleContext {
    /// 从物理像素宽度创建缩放上下文
    ///
    /// # Arguments
    /// * `physical_width` - 窗口的物理像素宽度（从 EguiState::size() 获取）
    /// * `base_width` - 基准设计宽度
    pub fn from_physical_size(physical_width: u32, base_width: f32) -> Self {
        let factor = (physical_width as f32 / base_width).clamp(0.5, 4.0);
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
}
