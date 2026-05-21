use std::cell::OnceCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use plugin_canvas::{Event, event::EventResponse, window::WindowAttributes};
use raw_window_handle::RawWindowHandle;
use slint::platform::WindowAdapter;

use crate::{
    logging::{
        EditorStageLogLevel, EditorStageLogger, clear_current_editor_stage_logger, emit_stage_log,
        set_current_editor_stage_logger,
    },
    platform::PluginCanvasPlatform,
    view::PluginView,
    window_adapter::{PluginCanvasWindowAdapter, WINDOW_ADAPTER_FROM_SLINT, WINDOW_TO_SLINT},
};

pub struct SlintEditor;

impl SlintEditor {
    pub fn open<C, B>(
        parent: RawWindowHandle,
        window_attributes: WindowAttributes,
        view_builder: B,
    ) -> Rc<EditorHandle>
    where
        C: PluginView + 'static,
        B: Fn(Arc<plugin_canvas::Window>) -> Result<C, String> + 'static,
    {
        Self::open_with_logger(parent, window_attributes, None, view_builder)
    }

    pub fn open_with_logger<C, B>(
        parent: RawWindowHandle,
        window_attributes: WindowAttributes,
        logger: Option<EditorStageLogger>,
        view_builder: B,
    ) -> Rc<EditorHandle>
    where
        C: PluginView + 'static,
        B: Fn(Arc<plugin_canvas::Window>) -> Result<C, String> + 'static,
    {
        set_current_editor_stage_logger(logger.clone());
        plugin_canvas::trace_logging::set_current_trace_logger(logger.as_ref().map(|logger| {
            let logger = logger.clone();
            std::sync::Arc::new(move |level, module, message, blocking| {
                let level = match level {
                    plugin_canvas::trace_logging::TraceLogLevel::Info => EditorStageLogLevel::Info,
                    plugin_canvas::trace_logging::TraceLogLevel::Warn => EditorStageLogLevel::Warn,
                    plugin_canvas::trace_logging::TraceLogLevel::Error => {
                        EditorStageLogLevel::Error
                    }
                };
                logger(level, module, message, blocking);
            }) as plugin_canvas::trace_logging::TraceLogger
        }));
        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "plugin-canvas-slint open begin",
            true,
        );

        let editor_handle = Rc::new(EditorHandle::new());

        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "Before plugin_canvas::Window::open()",
            true,
        );
        let window = match plugin_canvas::Window::open(parent, window_attributes.clone(), {
            let editor_weak_ptr = Rc::downgrade(&editor_handle).into_raw();
            let editor_thread = std::thread::current().id();
            let logger = logger.clone();

            Box::new(move |event| {
                if std::thread::current().id() != editor_thread {
                    emit_stage_log(
                        logger.as_ref(),
                        EditorStageLogLevel::Warn,
                        "EditorOpen",
                        "Tried to call event callback from non-editor thread",
                        false,
                    );
                    return EventResponse::Ignored;
                }

                let editor_weak = unsafe { Weak::from_raw(editor_weak_ptr) };
                let response = if let Some(editor_handle) = editor_weak.upgrade() {
                    editor_handle.on_event(&event)
                } else {
                    EventResponse::Ignored
                };

                // Leak the weak reference to avoid dropping it
                let _ = editor_weak.into_raw();

                response
            })
        }) {
            Ok(window) => {
                emit_stage_log(
                    logger.as_ref(),
                    EditorStageLogLevel::Info,
                    "EditorOpen",
                    "plugin_canvas::Window::open() succeeded",
                    true,
                );
                window
            }
            Err(err) => {
                emit_stage_log(
                    logger.as_ref(),
                    EditorStageLogLevel::Error,
                    "EditorOpen",
                    format!("Failed to open plugin canvas window: {err:?}"),
                    true,
                );
                clear_open_state();
                return editor_handle;
            }
        };

        // It's ok if this fails as it just means it has already been set
        slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();
        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "Slint platform registered or already present",
            true,
        );

        let window = Arc::new(window);
        WINDOW_TO_SLINT.set(Some(window.clone()));

        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "Before view_builder(window)",
            true,
        );
        let view = match view_builder(window) {
            Ok(view) => {
                emit_stage_log(
                    logger.as_ref(),
                    EditorStageLogLevel::Info,
                    "EditorOpen",
                    "view_builder(window) succeeded",
                    true,
                );
                view
            }
            Err(err) => {
                emit_stage_log(
                    logger.as_ref(),
                    EditorStageLogLevel::Error,
                    "EditorOpen",
                    format!("Failed to build Slint plugin view: {err}"),
                    true,
                );
                clear_open_state();
                return editor_handle;
            }
        };

        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "Before view.window().show()",
            true,
        );
        if let Err(err) = view.window().show() {
            emit_stage_log(
                logger.as_ref(),
                EditorStageLogLevel::Error,
                "EditorOpen",
                format!("Failed to show Slint window: {err}"),
                true,
            );
            if let Some(window_adapter) = WINDOW_ADAPTER_FROM_SLINT.take() {
                window_adapter.close();
            }
            clear_open_state();
            return editor_handle;
        }
        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "view.window().show() returned successfully",
            true,
        );

        let Some(window_adapter) = WINDOW_ADAPTER_FROM_SLINT.take() else {
            emit_stage_log(
                logger.as_ref(),
                EditorStageLogLevel::Error,
                "EditorOpen",
                "Slint window adapter was not created during editor open",
                true,
            );
            clear_open_state();
            return editor_handle;
        };
        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "Window adapter retrieved from thread-local",
            true,
        );
        window_adapter.set_view(Box::new(view));

        editor_handle.set_window_adapter(window_adapter);
        emit_stage_log(
            logger.as_ref(),
            EditorStageLogLevel::Info,
            "EditorOpen",
            "plugin-canvas-slint open completed",
            true,
        );
        clear_open_state();
        editor_handle
    }
}

fn clear_open_state() {
    WINDOW_TO_SLINT.set(None);
    WINDOW_ADAPTER_FROM_SLINT.set(None);
    clear_current_editor_stage_logger();
    plugin_canvas::trace_logging::clear_current_trace_logger();
}

pub struct EditorHandle {
    window_adapter: OnceCell<Rc<PluginCanvasWindowAdapter>>,
}

impl EditorHandle {
    pub fn on_frame(&self) {
        self.on_event(&Event::Draw);
    }

    pub fn set_window_size(&self, width: f64, height: f64) {
        let size = slint::LogicalSize {
            width: width as _,
            height: height as _,
        };

        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.set_size(size.into());
        }
    }

    pub fn set_scale(&self, scale: f64) {
        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.set_scale(scale);
        }
    }

    fn new() -> Self {
        Self {
            window_adapter: Default::default(),
        }
    }

    fn window_adapter(&self) -> Option<&PluginCanvasWindowAdapter> {
        self.window_adapter.get().map(|adapter| &**adapter)
    }

    fn set_window_adapter(&self, window_adapter: Rc<PluginCanvasWindowAdapter>) {
        self.window_adapter.set(window_adapter).unwrap();
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.on_event(event)
        } else {
            EventResponse::Ignored
        }
    }
}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        if let Some(window_adapter) = self.window_adapter.get() {
            window_adapter.close();
        }
    }
}
