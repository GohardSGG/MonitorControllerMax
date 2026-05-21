use std::cell::RefCell;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum TraceLogLevel {
    Info,
    Warn,
    Error,
}

pub type TraceLogger = Arc<dyn Fn(TraceLogLevel, &'static str, String, bool) + Send + Sync>;

thread_local! {
    static CURRENT_TRACE_LOGGER: RefCell<Option<TraceLogger>> = Default::default();
}

pub fn set_current_trace_logger(logger: Option<TraceLogger>) {
    CURRENT_TRACE_LOGGER.set(logger);
}

pub fn clear_current_trace_logger() {
    CURRENT_TRACE_LOGGER.set(None);
}

pub fn current_trace_logger() -> Option<TraceLogger> {
    CURRENT_TRACE_LOGGER.take()
}

pub fn emit_trace_log(
    logger: Option<&TraceLogger>,
    level: TraceLogLevel,
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
        TraceLogLevel::Info => log::info!("[{}] {}", module, message),
        TraceLogLevel::Warn => log::warn!("[{}] {}", module, message),
        TraceLogLevel::Error => log::error!("[{}] {}", module, message),
    }
}
