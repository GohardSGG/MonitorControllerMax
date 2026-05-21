#![allow(non_snake_case)]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use mcm_core::config_manager::ConfigManager;
use mcm_core::interaction::InteractionManager;
use mcm_core::osc_state::OscSharedState;
use mcm_core::params::MonitorParams;
use mcm_infra::logger::InstanceLogger;
use mcm_protocol::config::AppConfig;
use mcm_protocol::web_structs::WebSharedState;
use nih_plug::prelude::Editor;

#[cfg(feature = "gui-slint")]
pub mod slint_host;

#[cfg(feature = "gui-slint")]
pub mod slint_ui {
    slint::include_modules!();
}

pub fn create_editor(
    params: Arc<MonitorParams>,
    interaction: Arc<InteractionManager>,
    osc_state: Arc<OscSharedState>,
    network_connected: Arc<AtomicBool>,
    logger: Arc<InstanceLogger>,
    app_config: AppConfig,
    layout_config: Arc<ConfigManager>,
    web_state: Arc<WebSharedState>,
) -> Option<Box<dyn Editor>> {
    #[cfg(feature = "gui-slint")]
    {
        let editor = slint_host::SlintHostEditor::new(
            (980, 700),
            params,
            interaction,
            osc_state,
            network_connected,
            logger,
            app_config,
            layout_config,
            web_state,
        );
        return Some(Box::new(editor));
    }

    #[allow(unreachable_code)]
    {
        let _ = (
            params,
            interaction,
            osc_state,
            network_connected,
            logger,
            app_config,
            layout_config,
            web_state,
        );
        None
    }
}
