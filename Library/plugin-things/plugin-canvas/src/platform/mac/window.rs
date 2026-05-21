use std::{
    cell::RefCell,
    ptr::{NonNull, null_mut},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use cursor_icon::CursorIcon;
use objc2::{
    AllocAnyThread, msg_send,
    rc::{Allocated, Retained},
    sel,
};
use objc2_app_kit::{
    NSCursor, NSPasteboardTypeFileURL, NSScreen, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_core_foundation::{CGPoint, CGSize};
use objc2_core_graphics::CGWarpMouseCursorPosition;
use objc2_foundation::{MainThreadMarker, NSArray, NSPoint, NSRect, NSSize, NSTimer};
use raw_window_handle::{AppKitWindowHandle, HasDisplayHandle, HasWindowHandle, RawWindowHandle};

use crate::error::Error;
use crate::event::{EventCallback, EventResponse};
use crate::platform::interface::OsWindowInterface;
use crate::window::WindowAttributes;
use crate::{
    Event, LogicalPosition,
    platform::os_window_handle::OsWindowHandle,
    thread_bound::ThreadBound,
    trace_logging::{TraceLogLevel, TraceLogger, current_trace_logger, emit_trace_log},
};

use super::view::OsWindowView;

pub(crate) struct OsWindow {
    window_handle: AppKitWindowHandle,
    refresh_timer: RefCell<Option<Retained<NSTimer>>>,
    event_callback: Box<EventCallback>,
    logger: Option<TraceLogger>,

    cursor_hidden: AtomicBool,

    main_thread_marker: MainThreadMarker,
}

impl OsWindow {
    pub(super) fn send_event(&self, event: Event) -> EventResponse {
        (self.event_callback)(event)
    }

    fn view(&self) -> &OsWindowView {
        let window_view: *const OsWindowView = self.window_handle.ns_view.as_ptr() as _;
        unsafe { &*window_view }
    }
}

impl OsWindowInterface for OsWindow {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
    ) -> Result<OsWindowHandle, Error> {
        let logger = current_trace_logger();
        let RawWindowHandle::AppKit(parent_window_handle) = parent_window_handle else {
            emit_trace_log(
                logger.as_ref(),
                TraceLogLevel::Error,
                "EditorPlatform",
                "Parent window handle is not AppKit",
                true,
            );
            return Err(Error::PlatformError("Not an AppKit window".into()));
        };

        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "OsWindow::open begin",
            true,
        );
        let view_class = OsWindowView::register_class();
        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "OsWindowView class registered",
            true,
        );

        let physical_size =
            crate::PhysicalSize::from_logical(&window_attributes.size, window_attributes.scale);

        let view_rect = NSRect::new(
            NSPoint { x: 0.0, y: 0.0 },
            NSSize {
                width: physical_size.width as f64,
                height: physical_size.height as f64,
            },
        );

        let (view, window_handle) = unsafe {
            let view: Allocated<OsWindowView> = msg_send![view_class, alloc];
            let view: Retained<OsWindowView> = msg_send![view, initWithFrame: view_rect];

            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                view_rect,
                NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::MouseMoved
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect,
                Some(&view),
                None,
            );
            view.addTrackingArea(&tracking_area);

            let dragged_types = NSArray::arrayWithObject(NSPasteboardTypeFileURL);
            view.registerForDraggedTypes(&dragged_types);

            let parent_view: &mut NSView =
                &mut *(parent_window_handle.ns_view.as_ptr() as *mut NSView);
            parent_view.addSubview(&view);

            let window_handle =
                AppKitWindowHandle::new(NonNull::new(view.as_ref() as *const NSView as _).unwrap());

            (view, window_handle)
        };
        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            format!(
                "Child NSView attached size={}x{}",
                physical_size.width, physical_size.height
            ),
            true,
        );

        let main_thread_marker = MainThreadMarker::new().unwrap();
        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "MainThreadMarker acquired",
            true,
        );

        let window = Self {
            window_handle,
            refresh_timer: Default::default(),
            event_callback,
            logger: logger.clone(),

            cursor_hidden: Default::default(),

            main_thread_marker,
        };

        let window = Arc::new(ThreadBound::new(window));

        // Use NSTimer for display refresh — works on macOS 12+ (unlike CADisplayLink which requires 14+).
        // 1/60s ≈ 16.67ms matches 60 Hz; the Slint/Skia renderer drives actual frame pacing.
        // Use a dedicated timer callback selector here instead of reusing drawRect:,
        // because NSTimer sends itself as the argument.
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                1.0 / 60.0,
                &view,
                sel!(refreshTimerFired:),
                None,
                true,
            )
        };

        *window.refresh_timer.borrow_mut() = Some(timer);
        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "Refresh timer scheduled",
            true,
        );

        view.set_os_window_ptr(Arc::downgrade(&window).into_raw() as _);
        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "OsWindow weak pointer bound to NSView",
            true,
        );

        emit_trace_log(
            logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "OsWindow::open completed",
            true,
        );
        Ok(OsWindowHandle::new(window))
    }

    fn os_scale(&self) -> f64 {
        self.view()
            .window()
            .map(|window| window.backingScaleFactor())
            .unwrap_or(1.0)
    }

    fn resized(&self, size: crate::LogicalSize) {
        let cg_size = CGSize {
            width: size.width as _,
            height: size.height as _,
        };

        self.view().setFrameSize(cg_size);
    }

    fn set_cursor(&self, cursor: Option<CursorIcon>) {
        if let Some(cursor) = cursor {
            // Use classic NSCursor methods that work on macOS 12+.
            // The newer direction-specific APIs (columnResizeCursorInDirections, etc.)
            // require macOS 15+ and would crash on older systems.
            let cursor = match cursor {
                CursorIcon::Default => NSCursor::arrowCursor(),
                CursorIcon::ContextMenu => NSCursor::contextualMenuCursor(),
                CursorIcon::Help => NSCursor::arrowCursor(), // TODO
                CursorIcon::Pointer => NSCursor::pointingHandCursor(),
                CursorIcon::Progress => NSCursor::arrowCursor(), // TODO,
                CursorIcon::Wait => NSCursor::arrowCursor(),     // TODO
                CursorIcon::Cell => NSCursor::crosshairCursor(),
                CursorIcon::Crosshair => NSCursor::crosshairCursor(),
                CursorIcon::Text => NSCursor::IBeamCursor(),
                CursorIcon::VerticalText => NSCursor::IBeamCursorForVerticalLayout(),
                CursorIcon::Alias => NSCursor::dragLinkCursor(),
                CursorIcon::Copy => NSCursor::dragCopyCursor(),
                CursorIcon::Move => NSCursor::openHandCursor(),
                CursorIcon::NoDrop => NSCursor::operationNotAllowedCursor(),
                CursorIcon::NotAllowed => NSCursor::operationNotAllowedCursor(),
                CursorIcon::Grab => NSCursor::openHandCursor(),
                CursorIcon::Grabbing => NSCursor::closedHandCursor(),
                CursorIcon::EResize
                | CursorIcon::WResize
                | CursorIcon::EwResize
                | CursorIcon::ColResize => NSCursor::resizeLeftRightCursor(),
                CursorIcon::NResize
                | CursorIcon::SResize
                | CursorIcon::NsResize
                | CursorIcon::RowResize => NSCursor::resizeUpDownCursor(),
                CursorIcon::NeResize | CursorIcon::SwResize | CursorIcon::NeswResize => {
                    NSCursor::resizeUpDownCursor()
                }
                CursorIcon::NwResize | CursorIcon::SeResize | CursorIcon::NwseResize => {
                    NSCursor::resizeUpDownCursor()
                }
                CursorIcon::AllScroll => NSCursor::openHandCursor(),
                CursorIcon::ZoomIn => NSCursor::arrowCursor(), // TODO
                CursorIcon::ZoomOut => NSCursor::arrowCursor(), // TODO
                _ => NSCursor::arrowCursor(),
            };

            cursor.set();

            if self.cursor_hidden.swap(false, Ordering::Relaxed) {
                NSCursor::unhide();
            }
        } else if !self.cursor_hidden.swap(true, Ordering::Relaxed) {
            NSCursor::hide();
        }
    }

    fn set_input_focus(&self, focus: bool) {
        self.view().set_input_focus(focus);
    }

    fn warp_mouse(&self, position: LogicalPosition) {
        let window_position = self
            .view()
            .convertPoint_toView(CGPoint::new(position.x, position.y), None);
        let screen_position = self
            .view()
            .window()
            .unwrap()
            .convertPointToScreen(window_position);
        let screen_height = NSScreen::mainScreen(self.main_thread_marker)
            .unwrap()
            .frame()
            .size
            .height;
        let cg_point = CGPoint::new(screen_position.x, screen_height - screen_position.y);
        CGWarpMouseCursorPosition(cg_point);
    }

    fn poll_events(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        if let Some(timer) = self.refresh_timer.borrow().as_ref() {
            timer.invalidate();
        }

        self.view().set_os_window_ptr(null_mut());
        emit_trace_log(
            self.logger.as_ref(),
            TraceLogLevel::Info,
            "EditorPlatform",
            "OsWindow dropped and timer invalidated",
            false,
        );
    }
}

impl HasDisplayHandle for OsWindow {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::appkit())
    }
}

impl HasWindowHandle for OsWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw_window_handle = RawWindowHandle::AppKit(self.window_handle);
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw_window_handle) })
    }
}
