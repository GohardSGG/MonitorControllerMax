//! Windows keyboard polling for VST3 plugin text input
//!
//! This module provides keyboard input via GetAsyncKeyState polling,
//! which bypasses the DAW's keyboard message interception.

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicU16, Ordering};

#[cfg(target_os = "windows")]
extern "system" {
    fn GetAsyncKeyState(vKey: i32) -> i16;
}

/// Virtual key codes for numeric input
#[cfg(target_os = "windows")]
mod vk {
    pub const VK_0: i32 = 0x30;
    pub const VK_9: i32 = 0x39;
    pub const VK_NUMPAD0: i32 = 0x60;
    pub const VK_NUMPAD9: i32 = 0x69;
    pub const VK_BACK: i32 = 0x08;      // Backspace
    pub const VK_DELETE: i32 = 0x2E;    // Delete
    pub const VK_OEM_PERIOD: i32 = 0xBE; // Period (.)
    pub const VK_DECIMAL: i32 = 0x6E;    // Numpad decimal
    pub const VK_LEFT: i32 = 0x25;       // Left arrow
    pub const VK_RIGHT: i32 = 0x27;      // Right arrow
    pub const VK_HOME: i32 = 0x24;       // Home
    pub const VK_END: i32 = 0x23;        // End
}

/// Track previous key states to detect key press (not held)
#[cfg(target_os = "windows")]
static PREV_KEY_STATES: [AtomicU16; 256] = {
    const INIT: AtomicU16 = AtomicU16::new(0);
    [INIT; 256]
};

/// Check if a key was just pressed (transition from up to down)
#[cfg(target_os = "windows")]
fn is_key_just_pressed(vk: i32) -> bool {
    let current = unsafe { GetAsyncKeyState(vk) };
    let is_down = (current & 0x8000u16 as i16) != 0;
    let prev = PREV_KEY_STATES[vk as usize].swap(current as u16, Ordering::Relaxed);
    let was_down = (prev & 0x8000) != 0;

    is_down && !was_down
}

/// Poll keyboard and return any numeric characters that should be inserted.
/// Also handles backspace, delete, and navigation keys.
/// Returns: (chars_to_insert, backspace_pressed, delete_pressed, nav_key)
#[cfg(target_os = "windows")]
pub fn poll_numeric_input() -> (String, bool, bool, Option<NavKey>) {
    let mut chars = String::new();
    let mut backspace = false;
    let mut delete = false;
    let mut nav = None;

    // Check number row (0-9)
    for vk in vk::VK_0..=vk::VK_9 {
        if is_key_just_pressed(vk) {
            let digit = (vk - vk::VK_0) as u8;
            chars.push((b'0' + digit) as char);
        }
    }

    // Check numpad (0-9)
    for vk in vk::VK_NUMPAD0..=vk::VK_NUMPAD9 {
        if is_key_just_pressed(vk) {
            let digit = (vk - vk::VK_NUMPAD0) as u8;
            chars.push((b'0' + digit) as char);
        }
    }

    // Check period/decimal for IP addresses
    if is_key_just_pressed(vk::VK_OEM_PERIOD) || is_key_just_pressed(vk::VK_DECIMAL) {
        chars.push('.');
    }

    // Check backspace
    if is_key_just_pressed(vk::VK_BACK) {
        backspace = true;
    }

    // Check delete
    if is_key_just_pressed(vk::VK_DELETE) {
        delete = true;
    }

    // Check navigation keys
    if is_key_just_pressed(vk::VK_LEFT) {
        nav = Some(NavKey::Left);
    } else if is_key_just_pressed(vk::VK_RIGHT) {
        nav = Some(NavKey::Right);
    } else if is_key_just_pressed(vk::VK_HOME) {
        nav = Some(NavKey::Home);
    } else if is_key_just_pressed(vk::VK_END) {
        nav = Some(NavKey::End);
    }

    (chars, backspace, delete, nav)
}

#[derive(Debug, Clone, Copy)]
pub enum NavKey {
    Left,
    Right,
    Home,
    End,
}

/// Apply polled keyboard input to a text field
/// Returns true if the text was modified
#[cfg(target_os = "windows")]
pub fn apply_to_text(text: &mut String, chars: &str, backspace: bool, delete: bool) -> bool {
    let mut modified = false;

    // Insert characters
    if !chars.is_empty() {
        text.push_str(chars);
        modified = true;
    }

    // Handle backspace (delete last character)
    if backspace && !text.is_empty() {
        text.pop();
        modified = true;
    }

    // For delete, we'd need cursor position which egui manages
    // So we just treat it like backspace for simplicity
    if delete && !text.is_empty() {
        text.pop();
        modified = true;
    }

    modified
}

/// Convenience function: poll and apply in one call
#[cfg(target_os = "windows")]
pub fn poll_and_apply(text: &mut String) -> bool {
    let (chars, backspace, delete, _nav) = poll_numeric_input();
    apply_to_text(text, &chars, backspace, delete)
}

// No-op implementations for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn poll_numeric_input() -> (String, bool, bool, Option<NavKey>) {
    (String::new(), false, false, None)
}

#[cfg(not(target_os = "windows"))]
pub fn apply_to_text(_text: &mut String, _chars: &str, _backspace: bool, _delete: bool) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn poll_and_apply(_text: &mut String) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy)]
pub enum NavKey {
    Left,
    Right,
    Home,
    End,
}
