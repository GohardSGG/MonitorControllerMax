use std::cell::RefCell;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum EditorStageLogLevel {
    Info,
    Warn,
    Error,
}

pub type EditorStageLogger =
    Arc<dyn Fn(EditorStageLogLevel, &'static str, String, bool) + Send + Sync>;

thread_local! {
    static CURRENT_EDITOR_STAGE_LOGGER: RefCell<Option<EditorStageLogger>> = Default::default();
}

pub(crate) fn set_current_editor_stage_logger(logger: Option<EditorStageLogger>) {
    CURRENT_EDITOR_STAGE_LOGGER.set(logger);
}

pub(crate) fn clear_current_editor_stage_logger() {
    CURRENT_EDITOR_STAGE_LOGGER.set(None);
}

pub(crate) fn current_editor_stage_logger() -> Option<EditorStageLogger> {
    CURRENT_EDITOR_STAGE_LOGGER.take()
}

pub(crate) fn emit_stage_log(
    logger: Option<&EditorStageLogger>,
    level: EditorStageLogLevel,
    module: &'static str,
    message: impl Into<String>,
    blocking: bool,
) {
    let message = message.into();

    if let Some(logger) = logger {
        logger(level, module, message, blocking);
        return;
    }

    match level {
        EditorStageLogLevel::Info => log::info!("[{}] {}", module, message),
        EditorStageLogLevel::Warn => log::warn!("[{}] {}", module, message),
        EditorStageLogLevel::Error => log::error!("[{}] {}", module, message),
    }
}
