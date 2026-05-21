use std::any::Any;
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mcm_core::config_manager::{ConfigManager, Layout};
use mcm_core::interaction::{ChannelMarker, InteractionManager, SubClickType};
use mcm_core::osc_state::OscSharedState;
use mcm_core::params::{MonitorParams, PluginRole, SoloMode, MAX_CHANNELS};
use mcm_infra::logger::InstanceLogger;
use mcm_protocol::config::AppConfig;
use mcm_protocol::web_structs::{WebRestartAction, WebSharedState};
use nih_plug::editor::ParentWindowHandle;
use nih_plug::prelude::{Editor, GuiContext, ParamSetter};
use nih_plug::util;
use nih_plug_slint::editor::SlintEditor;
use nih_plug_slint::handle::SlintHostHandle;
use nih_plug_slint::plugin_canvas::event::EventResponse;
use nih_plug_slint::plugin_canvas::window::WindowAttributes;
use nih_plug_slint::plugin_canvas::{Event, LogicalSize};
use nih_plug_slint::resize::{HostResizeCoordinator, ResizeDebounce, ResizePolicy};
use nih_plug_slint::view::PluginView;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

const SLINT_DEFAULT_WIDTH: u32 = 980;
const SLINT_DEFAULT_HEIGHT: u32 = 700;
const SLINT_ASPECT_RATIO: f32 = SLINT_DEFAULT_WIDTH as f32 / SLINT_DEFAULT_HEIGHT as f32;
const SLINT_MIN_WIDTH: u32 = 760;
const SLINT_MIN_HEIGHT: u32 = 560;

enum UiMessage {
    SetWindowSize(u32, u32),
}

struct SlintBridgeState {
    ui_tx: Option<mpsc::Sender<UiMessage>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChannelUiSnapshot {
    name: String,
    state: i32,
    blinking: bool,
    grid_pos: i32,
    is_sub: bool,
    channel_index: i32,
    sub_relative_index: i32,
}

impl ChannelUiSnapshot {
    fn to_slint(&self) -> crate::editor::slint_ui::ChannelInfoUI {
        crate::editor::slint_ui::ChannelInfoUI {
            name: self.name.clone().into(),
            state: self.state,
            blinking: self.blinking,
            grid_pos: self.grid_pos,
            is_sub: self.is_sub,
            channel_index: self.channel_index,
            sub_relative_index: self.sub_relative_index,
        }
    }
}

struct PollState {
    prev_layout: i32,
    prev_sub_layout: i32,
    prev_role: i32,
    prev_grid_width: i32,
    prev_grid_height: i32,
    prev_channels: Vec<ChannelUiSnapshot>,
    speaker_names: Vec<String>,
    sub_names: Vec<String>,
}

pub struct SlintHostEditor {
    params: Arc<MonitorParams>,
    interaction: Arc<InteractionManager>,
    osc_state: Arc<OscSharedState>,
    network_connected: Arc<AtomicBool>,
    logger: Arc<InstanceLogger>,
    app_config: Arc<Mutex<AppConfig>>,
    layout_config: Arc<ConfigManager>,
    web_state: Arc<WebSharedState>,
    state: Mutex<SlintBridgeState>,
    resize_coordinator: HostResizeCoordinator,
}

impl SlintHostEditor {
    pub fn new(
        size: (u32, u32),
        params: Arc<MonitorParams>,
        interaction: Arc<InteractionManager>,
        osc_state: Arc<OscSharedState>,
        network_connected: Arc<AtomicBool>,
        logger: Arc<InstanceLogger>,
        app_config: AppConfig,
        layout_config: Arc<ConfigManager>,
        web_state: Arc<WebSharedState>,
    ) -> Self {
        let resize_coordinator = HostResizeCoordinator::new(
            size,
            ResizePolicy {
                min_width: SLINT_MIN_WIDTH,
                min_height: SLINT_MIN_HEIGHT,
                aspect_ratio: SLINT_ASPECT_RATIO,
            },
            ResizeDebounce::default(),
        );

        Self {
            params,
            interaction,
            osc_state,
            network_connected,
            logger,
            app_config: Arc::new(Mutex::new(app_config)),
            layout_config,
            web_state,
            state: Mutex::new(SlintBridgeState { ui_tx: None }),
            resize_coordinator,
        }
    }
}

impl Editor for SlintHostEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        gui_context: Arc<dyn GuiContext>,
    ) -> Box<dyn Any + Send> {
        let (ui_tx, ui_rx) = mpsc::channel::<UiMessage>();
        {
            let mut guard = self.state.lock().expect("slint state lock poisoned");
            guard.ui_tx = Some(ui_tx);
        }

        let initial_size = self.size();
        let window_attributes = WindowAttributes::new(
            LogicalSize::new(initial_size.0 as f64, initial_size.1 as f64),
            1.0,
        );

        let params = Arc::clone(&self.params);
        let interaction = Arc::clone(&self.interaction);
        let osc_state = Arc::clone(&self.osc_state);
        let network_connected = Arc::clone(&self.network_connected);
        let logger = Arc::clone(&self.logger);
        let app_config = Arc::clone(&self.app_config);
        let layout_config = Arc::clone(&self.layout_config);
        let web_state = Arc::clone(&self.web_state);
        let resize_coordinator = self.resize_coordinator.clone();
        let ui_rx_cell = Arc::new(Mutex::new(Some(ui_rx)));

        let editor_handle = SlintEditor::open(parent, window_attributes, move |_window| {
            let rx = {
                let mut guard = ui_rx_cell.lock().expect("slint rx lock poisoned");
                guard.take().expect("slint receiver already consumed")
            };
            Ok(SlintHostView::new(
                params.clone(),
                interaction.clone(),
                osc_state.clone(),
                network_connected.clone(),
                logger.clone(),
                app_config.clone(),
                layout_config.clone(),
                web_state.clone(),
                gui_context.clone(),
                resize_coordinator.clone(),
                rx,
            ))
        });

        Box::new(SlintHostHandle::new(editor_handle))
    }

    fn size(&self) -> (u32, u32) {
        self.resize_coordinator.size()
    }

    fn set_scale_factor(&self, _factor: f32) -> bool {
        false
    }

    fn min_size(&self) -> Option<(u32, u32)> {
        Some(self.resize_coordinator.min_size())
    }

    fn aspect_ratio(&self) -> Option<f32> {
        Some(self.resize_coordinator.aspect_ratio())
    }

    fn host_resized(&self, width: u32, height: u32) {
        let constrained = self.resize_coordinator.on_host_resized(width, height);
        self.logger.info(
            "slint-resize",
            &format!(
                "Host resized {}x{} -> constrained {}x{}",
                width, height, constrained.0, constrained.1
            ),
        );
        let maybe_tx = {
            let guard = self.state.lock().expect("slint state lock poisoned");
            guard.ui_tx.clone()
        };

        if let Some(tx) = maybe_tx {
            let _ = tx.send(UiMessage::SetWindowSize(constrained.0, constrained.1));
        }
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        // Polled in SlintHostView timer.
    }

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        // Polled in SlintHostView timer.
    }

    fn param_values_changed(&self) {
        // Polled in SlintHostView timer.
    }
}

struct SlintHostView {
    app: crate::editor::slint_ui::AppWindow,
    ui_rx: mpsc::Receiver<UiMessage>,
    _poll_timer: Timer,
}

impl SlintHostView {
    #[allow(clippy::too_many_arguments)]
    fn new(
        params: Arc<MonitorParams>,
        interaction: Arc<InteractionManager>,
        osc_state: Arc<OscSharedState>,
        network_connected: Arc<AtomicBool>,
        logger: Arc<InstanceLogger>,
        app_config: Arc<Mutex<AppConfig>>,
        layout_config: Arc<ConfigManager>,
        web_state: Arc<WebSharedState>,
        gui_context: Arc<dyn GuiContext>,
        resize_coordinator: HostResizeCoordinator,
        ui_rx: mpsc::Receiver<UiMessage>,
    ) -> Self {
        let app = crate::editor::slint_ui::AppWindow::new().expect("Failed to create AppWindow");
        logger.info("slint-click", "Slint click diagnostics active");

        let speaker_names = layout_config.get_speaker_layouts();
        let sub_names = layout_config.get_sub_layouts();

        let poll_state = Arc::new(Mutex::new(PollState {
            prev_layout: -1,
            prev_sub_layout: -1,
            prev_role: -1,
            prev_grid_width: -1,
            prev_grid_height: -1,
            prev_channels: Vec::new(),
            speaker_names,
            sub_names,
        }));

        if let Ok(cfg) = app_config.lock() {
            app.set_osc_send_port_text(cfg.osc_send_port.to_string().into());
            app.set_osc_receive_port_text(cfg.osc_receive_port.to_string().into());
            app.set_network_port_text(cfg.network_port.to_string().into());
            app.set_master_ip_text(cfg.master_ip.clone().into());
        }

        app.set_build_version_text(format!("v{}", env!("CARGO_PKG_VERSION")).into());
        app.set_grid_width(5);
        app.set_grid_height(5);
        app.set_channels(make_channel_model(Vec::new()));
        app.set_role_options(make_string_model(vec![
            role_to_text(PluginRole::Standalone).to_string(),
            role_to_text(PluginRole::Master).to_string(),
            role_to_text(PluginRole::Slave).to_string(),
        ]));
        if let Ok(state) = poll_state.lock() {
            app.set_layout_options(make_string_model(state.speaker_names.clone()));
            app.set_sub_layout_options(make_string_model(state.sub_names.clone()));
        } else {
            app.set_layout_options(make_string_model(Vec::new()));
            app.set_sub_layout_options(make_string_model(Vec::new()));
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            app.on_master_gain_changed(move |next| {
                let value = next.clamp(0.0, 1.0);
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.set_parameter(&params.master_gain, value);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_cycle_role(move |direction| {
                let current = params.role.value();
                let current_idx = match current {
                    PluginRole::Standalone => 0,
                    PluginRole::Master => 1,
                    PluginRole::Slave => 2,
                };
                let next_idx = (current_idx + direction).clamp(0, 2);
                if next_idx == current_idx {
                    return;
                }

                let next = match next_idx {
                    0 => PluginRole::Standalone,
                    1 => PluginRole::Master,
                    _ => PluginRole::Slave,
                };

                let setter = ParamSetter::new(gui_context.as_ref());
                apply_role_change(params.as_ref(), interaction.as_ref(), &setter, next, logger.as_ref());
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            app.on_cycle_solo_mode(move |direction| {
                let current = params.solo_mode.value();
                let current_idx = match current {
                    SoloMode::SIP => 0,
                    SoloMode::PFL => 1,
                };
                let next_idx = (current_idx + direction).clamp(0, 1);
                if next_idx == current_idx {
                    return;
                }

                let next = match next_idx {
                    0 => SoloMode::SIP,
                    _ => SoloMode::PFL,
                };

                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.solo_mode);
                setter.set_parameter(&params.solo_mode, next);
                setter.end_set_parameter(&params.solo_mode);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let gui_context = Arc::clone(&gui_context);
            let poll_state = Arc::clone(&poll_state);
            app.on_cycle_layout(move |direction| {
                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    return;
                }
                let max = poll_state
                    .lock()
                    .ok()
                    .map(|s| s.speaker_names.len() as i32 - 1)
                    .unwrap_or(-1);
                if max < 0 {
                    return;
                }

                let current = params.layout.value();
                let next = (current + direction).clamp(0, max);
                if next == current {
                    return;
                }

                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.layout);
                setter.set_parameter(&params.layout, next);
                setter.end_set_parameter(&params.layout);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let gui_context = Arc::clone(&gui_context);
            let poll_state = Arc::clone(&poll_state);
            app.on_cycle_sub_layout(move |direction| {
                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    return;
                }
                let max = poll_state
                    .lock()
                    .ok()
                    .map(|s| s.sub_names.len() as i32 - 1)
                    .unwrap_or(-1);
                if max < 0 {
                    return;
                }

                let current = params.sub_layout.value();
                let next = (current + direction).clamp(0, max);
                if next == current {
                    return;
                }

                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.sub_layout);
                setter.set_parameter(&params.sub_layout, next);
                setter.end_set_parameter(&params.sub_layout);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_select_role(move |index| {
                let next = match index {
                    0 => PluginRole::Standalone,
                    1 => PluginRole::Master,
                    2 => PluginRole::Slave,
                    _ => return,
                };

                if next == params.role.value() {
                    return;
                }

                let setter = ParamSetter::new(gui_context.as_ref());
                apply_role_change(
                    params.as_ref(),
                    interaction.as_ref(),
                    &setter,
                    next,
                    logger.as_ref(),
                );
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let gui_context = Arc::clone(&gui_context);
            app.on_select_layout(move |index| {
                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    return;
                }

                let next = index.max(0);
                if next == params.layout.value() {
                    return;
                }

                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.layout);
                setter.set_parameter(&params.layout, next);
                setter.end_set_parameter(&params.layout);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let gui_context = Arc::clone(&gui_context);
            app.on_select_sub_layout(move |index| {
                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    return;
                }

                let next = index.max(0);
                if next == params.sub_layout.value() {
                    return;
                }

                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.sub_layout);
                setter.set_parameter(&params.sub_layout, next);
                setter.end_set_parameter(&params.sub_layout);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let layout_config = Arc::clone(&layout_config);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_toggle_automation_mode(move |enabled| {
                if params.role.value() != PluginRole::Standalone {
                    return;
                }

                if enabled {
                    interaction.enter_automation_mode();
                } else {
                    interaction.exit_automation_mode();
                    if let Some(layout) = current_layout(layout_config.as_ref(), params.as_ref()) {
                        let setter = ParamSetter::new(gui_context.as_ref());
                        sync_all_channel_params(
                            params.as_ref(),
                            &setter,
                            interaction.as_ref(),
                            layout,
                            logger.as_ref(),
                        );
                    }
                }

                osc_state.broadcast_channel_states(interaction.as_ref());
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            let osc_state = Arc::clone(&osc_state);
            app.on_toggle_dim(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                let next = !params.dim.value();
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.dim);
                setter.set_parameter(&params.dim, next);
                setter.end_set_parameter(&params.dim);
                osc_state.send_dim(next);
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            let osc_state = Arc::clone(&osc_state);
            app.on_toggle_cut(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                let next = !params.cut.value();
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.cut);
                setter.set_parameter(&params.cut, next);
                setter.end_set_parameter(&params.cut);
                osc_state.send_cut(next);
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            let osc_state = Arc::clone(&osc_state);
            app.on_toggle_mono(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                let next = !params.mono.value();
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.mono);
                setter.set_parameter(&params.mono, next);
                setter.end_set_parameter(&params.mono);
                osc_state.send_mono(next);
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            let osc_state = Arc::clone(&osc_state);
            app.on_toggle_low_boost(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                let next = !params.low_boost.value();
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.low_boost);
                setter.set_parameter(&params.low_boost, next);
                setter.end_set_parameter(&params.low_boost);
                osc_state.send_low_boost(next);
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            let osc_state = Arc::clone(&osc_state);
            app.on_toggle_high_boost(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                let next = !params.high_boost.value();
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.high_boost);
                setter.set_parameter(&params.high_boost, next);
                setter.end_set_parameter(&params.high_boost);
                osc_state.send_high_boost(next);
            });
        }

        {
            let params = Arc::clone(&params);
            let gui_context = Arc::clone(&gui_context);
            let osc_state = Arc::clone(&osc_state);
            app.on_toggle_lfe_add_10db(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                let next = !params.lfe_add_10db.value();
                let setter = ParamSetter::new(gui_context.as_ref());
                setter.begin_set_parameter(&params.lfe_add_10db);
                setter.set_parameter(&params.lfe_add_10db, next);
                setter.end_set_parameter(&params.lfe_add_10db);
                osc_state.send_lfe_add_10db(next);
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let layout_config = Arc::clone(&layout_config);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_toggle_solo_mode(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                interaction.on_solo_button_click();
                if let Some(layout) = current_layout(layout_config.as_ref(), params.as_ref()) {
                    let setter = ParamSetter::new(gui_context.as_ref());
                    sync_all_channel_params(
                        params.as_ref(),
                        &setter,
                        interaction.as_ref(),
                        layout,
                        logger.as_ref(),
                    );
                }

                osc_state.send_mode_solo(interaction.is_solo_active());
                if !interaction.is_mute_active() {
                    osc_state.send_mode_mute(false);
                }
                osc_state.broadcast_channel_states(interaction.as_ref());
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let layout_config = Arc::clone(&layout_config);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_toggle_mute_mode(move || {
                if params.role.value() == PluginRole::Slave {
                    return;
                }
                interaction.on_mute_button_click();
                if let Some(layout) = current_layout(layout_config.as_ref(), params.as_ref()) {
                    let setter = ParamSetter::new(gui_context.as_ref());
                    sync_all_channel_params(
                        params.as_ref(),
                        &setter,
                        interaction.as_ref(),
                        layout,
                        logger.as_ref(),
                    );
                }

                osc_state.send_mode_mute(interaction.is_mute_active());
                if !interaction.is_solo_active() {
                    osc_state.send_mode_solo(false);
                }
                osc_state.broadcast_channel_states(interaction.as_ref());
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let layout_config = Arc::clone(&layout_config);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_channel_click(move |channel_index| {
                logger.info(
                    "slint-click",
                    &format!(
                        "UI main click received: idx={}, role={:?}, automation={}",
                        channel_index,
                        params.role.value(),
                        interaction.is_automation_mode()
                    ),
                );

                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    logger.info(
                        "slint-click",
                        "UI main click ignored by guard (slave or automation mode)",
                    );
                    return;
                }

                let idx = channel_index.max(0) as usize;
                let Some(layout) = current_layout(layout_config.as_ref(), params.as_ref()) else {
                    logger.warn("slint-click", "UI main click ignored: no current layout");
                    return;
                };

                let Some(ch) = layout.main_channels.iter().find(|c| c.channel_index == idx) else {
                    let available: Vec<String> = layout
                        .main_channels
                        .iter()
                        .map(|c| format!("{}:{}", c.name, c.channel_index))
                        .collect();
                    logger.warn(
                        "slint-click",
                        &format!(
                            "UI main click index not found: idx={}, available=[{}]",
                            idx,
                            available.join(", ")
                        ),
                    );
                    return;
                };

                if interaction.on_channel_click(&ch.name) {
                    let setter = ParamSetter::new(gui_context.as_ref());
                    sync_all_channel_params(
                        params.as_ref(),
                        &setter,
                        interaction.as_ref(),
                        layout,
                        logger.as_ref(),
                    );
                    osc_state.broadcast_channel_states(interaction.as_ref());
                    logger.info(
                        "slint-click",
                        &format!("UI main click applied: {} (idx={})", ch.name, idx),
                    );
                } else {
                    logger.info(
                        "slint-click",
                        &format!("UI main click had no state effect: {} (idx={})", ch.name, idx),
                    );
                }
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let layout_config = Arc::clone(&layout_config);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_sub_channel_click(move |sub_relative_index, channel_index| {
                logger.info(
                    "slint-click",
                    &format!(
                        "UI sub click received: sub_idx={}, ch_idx={}, role={:?}, automation={}",
                        sub_relative_index,
                        channel_index,
                        params.role.value(),
                        interaction.is_automation_mode()
                    ),
                );

                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    logger.info(
                        "slint-click",
                        "UI sub click ignored by guard (slave or automation mode)",
                    );
                    return;
                }

                let Some(layout) = current_layout(layout_config.as_ref(), params.as_ref()) else {
                    logger.warn("slint-click", "UI sub click ignored: no current layout");
                    return;
                };

                let sub_idx = sub_relative_index.max(0) as usize;
                let channel_idx = channel_index.max(0) as usize;
                let sub_channel = layout
                    .sub_channels
                    .iter()
                    .enumerate()
                    .find(|(_, c)| c.channel_index == channel_idx)
                    .or_else(|| layout.sub_channels.get(sub_idx).map(|c| (sub_idx, c)));
                let Some((resolved_sub_idx, ch)) = sub_channel else {
                    let available: Vec<String> = layout
                        .sub_channels
                        .iter()
                        .enumerate()
                        .map(|(i, c)| format!("{}:ch{}:sub{}", c.name, c.channel_index, i))
                        .collect();
                    logger.warn(
                        "slint-click",
                        &format!(
                            "UI sub click index not found: sub_idx={}, ch_idx={}, available=[{}]",
                            sub_idx,
                            channel_idx,
                            available.join(", ")
                        ),
                    );
                    return;
                };

                let handled = match interaction.detect_sub_click(resolved_sub_idx) {
                    SubClickType::SingleClick => interaction.on_channel_click(&ch.name),
                    SubClickType::DoubleClick => interaction.on_sub_double_click(&ch.name),
                };

                if handled {
                    let setter = ParamSetter::new(gui_context.as_ref());
                    sync_all_channel_params(
                        params.as_ref(),
                        &setter,
                        interaction.as_ref(),
                        layout,
                        logger.as_ref(),
                    );
                    osc_state.broadcast_channel_states(interaction.as_ref());
                    logger.info(
                        "slint-click",
                        &format!(
                            "UI sub click applied: {} (sub_idx={}, ch_idx={})",
                            ch.name, resolved_sub_idx, channel_idx
                        ),
                    );
                } else {
                    logger.info(
                        "slint-click",
                        &format!(
                            "UI sub click had no state effect: {} (sub_idx={}, ch_idx={})",
                            ch.name, resolved_sub_idx, channel_idx
                        ),
                    );
                }
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let layout_config = Arc::clone(&layout_config);
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            app.on_sub_channel_secondary_click(move |channel_index| {
                logger.info(
                    "slint-click",
                    &format!(
                        "UI sub secondary click received: ch_idx={}, role={:?}, automation={}",
                        channel_index,
                        params.role.value(),
                        interaction.is_automation_mode()
                    ),
                );

                if params.role.value() == PluginRole::Slave || interaction.is_automation_mode() {
                    logger.info(
                        "slint-click",
                        "UI sub secondary click ignored by guard (slave or automation mode)",
                    );
                    return;
                }

                let Some(layout) = current_layout(layout_config.as_ref(), params.as_ref()) else {
                    logger.warn(
                        "slint-click",
                        "UI sub secondary click ignored: no current layout",
                    );
                    return;
                };

                let channel_idx = channel_index.max(0) as usize;
                let Some(ch) = layout.sub_channels.iter().find(|c| c.channel_index == channel_idx) else {
                    let available: Vec<String> = layout
                        .sub_channels
                        .iter()
                        .map(|c| format!("{}:{}", c.name, c.channel_index))
                        .collect();
                    logger.warn(
                        "slint-click",
                        &format!(
                            "UI sub secondary click index not found: ch_idx={}, available=[{}]",
                            channel_idx,
                            available.join(", ")
                        ),
                    );
                    return;
                };

                if interaction.on_sub_double_click(&ch.name) {
                    let setter = ParamSetter::new(gui_context.as_ref());
                    sync_all_channel_params(
                        params.as_ref(),
                        &setter,
                        interaction.as_ref(),
                        layout,
                        logger.as_ref(),
                    );
                    osc_state.broadcast_channel_states(interaction.as_ref());
                    logger.info(
                        "slint-click",
                        &format!("UI sub secondary click applied: {} (ch_idx={})", ch.name, channel_idx),
                    );
                } else {
                    logger.info(
                        "slint-click",
                        &format!(
                            "UI sub secondary click had no state effect: {} (ch_idx={})",
                            ch.name, channel_idx
                        ),
                    );
                }
            });
        }

        {
            let interaction = Arc::clone(&interaction);
            let web_state = Arc::clone(&web_state);
            app.on_toggle_web(move || {
                if web_state.is_running.load(Ordering::Relaxed) {
                    interaction.request_web_restart(WebRestartAction::Stop);
                } else {
                    interaction.request_web_restart(WebRestartAction::Start);
                }
            });
        }

        {
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let app_config = Arc::clone(&app_config);
            let logger = Arc::clone(&logger);
            let app_weak = app.as_weak();
            app.on_save_settings(move || {
                let Some(app) = app_weak.upgrade() else {
                    return;
                };

                let mut new_config = if let Ok(cfg) = app_config.lock() {
                    cfg.clone()
                } else {
                    logger.error("slint", "Settings lock poisoned, cannot save");
                    return;
                };

                new_config.osc_send_port = parse_port(
                    &app.get_osc_send_port_text().to_string(),
                    new_config.osc_send_port,
                );
                new_config.osc_receive_port = parse_port(
                    &app.get_osc_receive_port_text().to_string(),
                    new_config.osc_receive_port,
                );
                new_config.network_port = parse_port(
                    &app.get_network_port_text().to_string(),
                    new_config.network_port,
                );
                new_config.master_ip = app.get_master_ip_text().to_string();

                match mcm_infra::config_loader::save_to_disk(&new_config) {
                    Ok(_) => {
                        if let Ok(mut cfg) = app_config.lock() {
                            *cfg = new_config.clone();
                        }
                        interaction.request_osc_restart(new_config.clone());
                        if params.role.value() != PluginRole::Standalone {
                            interaction.request_network_restart(new_config.clone());
                        }
                        interaction.request_web_restart(WebRestartAction::Start);
                        app.set_status_line("Settings saved. Restart requests queued.".into());
                    }
                    Err(err) => {
                        logger.error("slint", &format!("Save settings failed: {err}"));
                        app.set_status_line(format!("Save failed: {err}").into());
                    }
                }
            });
        }

        {
            let logger = Arc::clone(&logger);
            app.on_open_config_folder(move || {
                let config_path = mcm_infra::config_loader::config_path();
                if let Some(parent) = config_path.parent() {
                    if let Err(err) = open::that(parent) {
                        logger.warn("slint", &format!("Failed to open config folder: {err}"));
                    }
                }
            });
        }

        {
            let gui_context = Arc::clone(&gui_context);
            let logger = Arc::clone(&logger);
            let resize_coordinator = resize_coordinator.clone();
            app.on_handle_resize_request(move |width_px, height_px| {
                let requested_w = width_px.max(1.0).round() as u32;
                let requested_h = height_px.max(1.0).round() as u32;
                let now = Instant::now();
                let Some(constrained) =
                    resize_coordinator.begin_request_from_ui(requested_w, requested_h, now)
                else {
                    return;
                };
                logger.info(
                    "slint-resize",
                    &format!(
                        "UI requested resize {}x{} -> constrained {}x{}",
                        requested_w, requested_h, constrained.0, constrained.1
                    ),
                );

                if !gui_context.request_resize() {
                    resize_coordinator.reject_pending_request();
                    logger.warn(
                        "slint-resize",
                        &format!(
                            "Host denied resize request to {}x{}",
                            constrained.0, constrained.1
                        ),
                    );
                }
            });
        }

        let poll_timer = Timer::default();
        {
            let app_weak = app.as_weak();
            let params = Arc::clone(&params);
            let interaction = Arc::clone(&interaction);
            let osc_state = Arc::clone(&osc_state);
            let network_connected = Arc::clone(&network_connected);
            let logger = Arc::clone(&logger);
            let app_config = Arc::clone(&app_config);
            let layout_config = Arc::clone(&layout_config);
            let web_state = Arc::clone(&web_state);
            let gui_context = Arc::clone(&gui_context);
            let poll_state = Arc::clone(&poll_state);

            poll_timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
                let Some(app) = app_weak.upgrade() else {
                    return;
                };
                refresh_runtime_ui(
                    &app,
                    params.as_ref(),
                    interaction.as_ref(),
                    osc_state.as_ref(),
                    network_connected.as_ref(),
                    logger.as_ref(),
                    app_config.as_ref(),
                    layout_config.as_ref(),
                    web_state.as_ref(),
                    gui_context.as_ref(),
                    poll_state.as_ref(),
                );
            });
        }

        Self {
            app,
            ui_rx,
            _poll_timer: poll_timer,
        }
    }

    fn flush_ui_messages(&self) {
        while let Ok(message) = self.ui_rx.try_recv() {
            match message {
                UiMessage::SetWindowSize(width, height) => {
                    self.app
                        .window()
                        .set_size(slint::LogicalSize::new(width as f32, height as f32));
                }
            }
        }
    }
}

impl PluginView for SlintHostView {
    fn window(&self) -> &slint::Window {
        self.app.window()
    }

    fn on_event(&self, _event: &Event) -> EventResponse {
        self.flush_ui_messages();
        EventResponse::Ignored
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_runtime_ui(
    app: &crate::editor::slint_ui::AppWindow,
    params: &MonitorParams,
    interaction: &InteractionManager,
    osc_state: &OscSharedState,
    network_connected: &AtomicBool,
    logger: &InstanceLogger,
    app_config: &Mutex<AppConfig>,
    layout_config: &ConfigManager,
    web_state: &WebSharedState,
    gui_context: &dyn GuiContext,
    poll_state: &Mutex<PollState>,
) {
    let setter = ParamSetter::new(gui_context);

    if let Some(volume) = osc_state.take_pending_volume() {
        setter.begin_set_parameter(&params.master_gain);
        setter.set_parameter(&params.master_gain, volume);
        setter.end_set_parameter(&params.master_gain);
    }

    if let Some(dim) = osc_state.take_pending_dim() {
        setter.begin_set_parameter(&params.dim);
        setter.set_parameter(&params.dim, dim);
        setter.end_set_parameter(&params.dim);
    }

    if let Some(cut) = osc_state.take_pending_cut() {
        setter.begin_set_parameter(&params.cut);
        setter.set_parameter(&params.cut, cut);
        setter.end_set_parameter(&params.cut);
        osc_state.sync_cut_state(cut);
    }

    // 效果器开关：仅当 OSC 有新变化时同步到 DAW 参数（pending 标志模式）
    if let Some(low) = osc_state.take_pending_low_boost() {
        setter.begin_set_parameter(&params.low_boost);
        setter.set_parameter(&params.low_boost, low);
        setter.end_set_parameter(&params.low_boost);
    }
    if let Some(high) = osc_state.take_pending_high_boost() {
        setter.begin_set_parameter(&params.high_boost);
        setter.set_parameter(&params.high_boost, high);
        setter.end_set_parameter(&params.high_boost);
    }
    if let Some(lfe) = osc_state.take_pending_lfe_add_10db() {
        setter.begin_set_parameter(&params.lfe_add_10db);
        setter.set_parameter(&params.lfe_add_10db, lfe);
        setter.end_set_parameter(&params.lfe_add_10db);
    }
    if let Some(mono) = osc_state.take_pending_mono() {
        setter.begin_set_parameter(&params.mono);
        setter.set_parameter(&params.mono, mono);
        setter.end_set_parameter(&params.mono);
    }

    if params.role.value() == PluginRole::Slave {
        if let Some(gain) = interaction.take_network_master_gain() {
            setter.begin_set_parameter(&params.master_gain);
            setter.set_parameter(&params.master_gain, gain);
            setter.end_set_parameter(&params.master_gain);
        }
        if let Some(dim) = interaction.take_network_dim() {
            setter.begin_set_parameter(&params.dim);
            setter.set_parameter(&params.dim, dim);
            setter.end_set_parameter(&params.dim);
        }
        if let Some(cut) = interaction.take_network_cut() {
            setter.begin_set_parameter(&params.cut);
            setter.set_parameter(&params.cut, cut);
            setter.end_set_parameter(&params.cut);
        }
        if let Some(layout_idx) = interaction.take_network_layout() {
            if layout_idx != params.layout.value() {
                interaction.clear_on_layout_change();
                setter.begin_set_parameter(&params.layout);
                setter.set_parameter(&params.layout, layout_idx);
                setter.end_set_parameter(&params.layout);
            }
        }
        if let Some(sub_idx) = interaction.take_network_sub_layout() {
            if sub_idx != params.sub_layout.value() {
                interaction.clear_on_layout_change();
                setter.begin_set_parameter(&params.sub_layout);
                setter.set_parameter(&params.sub_layout, sub_idx);
                setter.end_set_parameter(&params.sub_layout);
            }
        }
    }

    let mut state = match poll_state.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let current_layout_idx = params.layout.value();
    let current_sub_layout = params.sub_layout.value();
    let current_role_index = match params.role.value() {
        PluginRole::Standalone => 0,
        PluginRole::Master => 1,
        PluginRole::Slave => 2,
    };

    let first_load = state.prev_layout < 0 || state.prev_sub_layout < 0;
    let layout_changed =
        state.prev_layout != current_layout_idx || state.prev_sub_layout != current_sub_layout;

    if first_load || layout_changed {
        if layout_changed && !interaction.is_automation_mode() {
            interaction.clear_on_layout_change();
        }

        if let Some(layout) = current_layout(layout_config, params) {
            if !interaction.is_automation_mode() {
                sync_all_channel_params(params, &setter, interaction, layout, logger);
            }
            osc_state.update_layout_channels(layout);
            osc_state.broadcast_channel_states(interaction);
        }
    }

    if state.prev_role >= 0 && state.prev_role != current_role_index {
        if let Ok(cfg) = app_config.lock() {
            interaction.request_network_restart(cfg.clone());
        }
    }

    state.prev_layout = current_layout_idx;
    state.prev_sub_layout = current_sub_layout;
    state.prev_role = current_role_index;

    let layout_name = state
        .speaker_names
        .get(current_layout_idx.max(0) as usize)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let sub_layout_name = state
        .sub_names
        .get(current_sub_layout.max(0) as usize)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());

    app.set_role_text(role_to_text(params.role.value()).into());
    app.set_solo_mode_text(solo_mode_to_text(params.solo_mode.value()).into());
    app.set_layout_text(layout_name.into());
    app.set_sub_layout_text(sub_layout_name.into());
    app.set_role_index(current_role_index);

    let max_layout = (state.speaker_names.len() as i32 - 1).max(0);
    let max_sub_layout = (state.sub_names.len() as i32 - 1).max(0);
    app.set_layout_index(current_layout_idx.clamp(0, max_layout));
    app.set_sub_layout_index(current_sub_layout.clamp(0, max_sub_layout));

    let is_slave = params.role.value() == PluginRole::Slave;
    let is_automation = interaction.is_automation_mode();
    app.set_can_change_role(true);
    app.set_can_change_layout(!is_automation && !is_slave);
    app.set_controls_enabled(!is_slave);
    app.set_channel_interaction_enabled(!is_automation && !is_slave);
    app.set_automation_toggle_enabled(params.role.value() == PluginRole::Standalone);

    let gain = params.master_gain.value();
    app.set_master_gain_slider(gain);
    app.set_master_gain_text(format!("{}%", (gain * 100.0).round() as i32).into());

    app.set_dim_on(params.dim.value());
    app.set_cut_on(params.cut.value());
    app.set_mono_on(params.mono.value());

    let lb = params.low_boost.value();
    let hb = params.high_boost.value();
    let lfe = params.lfe_add_10db.value();
    app.set_low_boost_on(lb);
    app.set_high_boost_on(hb);
    app.set_lfe_add_10db_on(lfe);

    // 推送 Event Log 到 UI
    let logs = logger.get_recent_logs();
    if logs.is_empty() {
        app.set_event_log_text("No events recorded.".into());
    } else {
        app.set_event_log_text(logs.join("\n").into());
    }

    app.set_solo_active(interaction.is_solo_active());
    app.set_mute_active(interaction.is_mute_active());
    app.set_automation_mode(is_automation);

    app.set_web_running(web_state.is_running.load(Ordering::Relaxed));
    app.set_web_address(
        web_state
            .get_address()
            .unwrap_or_else(|| "not running".to_string())
            .into(),
    );
    app.set_osc_recv_bound(osc_state.recv_port_bound.load(Ordering::Relaxed));
    app.set_network_connected(network_connected.load(Ordering::Relaxed));

    interaction.tick_blink();
    let blink_show = interaction.should_blink_show();
    let solo_visible = if interaction.is_solo_blinking() {
        blink_show
    } else {
        interaction.is_solo_steady()
    };
    let mute_visible = if interaction.is_mute_blinking() {
        blink_show
    } else {
        interaction.is_mute_steady()
    };
    app.set_solo_visible(solo_visible);
    app.set_mute_visible(mute_visible);

    if let Some(layout) = current_layout(layout_config, params) {
        let grid_width = layout.width as i32;
        let grid_height = layout.height as i32;
        if state.prev_grid_width != grid_width {
            app.set_grid_width(grid_width);
            state.prev_grid_width = grid_width;
        }
        if state.prev_grid_height != grid_height {
            app.set_grid_height(grid_height);
            state.prev_grid_height = grid_height;
        }

        let mut channel_snapshots: Vec<ChannelUiSnapshot> = Vec::with_capacity(layout.total_channels);

        for ch in layout.main_channels.iter() {
            let display = interaction.get_channel_display(&ch.name);
            let mut state_value = match display.marker {
                Some(ChannelMarker::Mute) => 1,
                Some(ChannelMarker::Solo) => 2,
                None => 0,
            };

            if display.is_blinking && !blink_show {
                state_value = 0;
            }

            channel_snapshots.push(ChannelUiSnapshot {
                name: ch.name.clone(),
                state: state_value,
                blinking: display.is_blinking,
                grid_pos: ch.grid_pos as i32,
                is_sub: false,
                channel_index: ch.channel_index as i32,
                sub_relative_index: -1,
            });
        }

        for (sub_idx, ch) in layout.sub_channels.iter().enumerate() {
            let display = interaction.get_channel_display(&ch.name);
            let mut state_value = match display.marker {
                Some(ChannelMarker::Mute) => 1,
                Some(ChannelMarker::Solo) => 2,
                None => 0,
            };

            if display.is_blinking && !blink_show {
                state_value = 0;
            }

            channel_snapshots.push(ChannelUiSnapshot {
                name: ch.name.clone(),
                state: state_value,
                blinking: display.is_blinking,
                grid_pos: ch.grid_pos as i32,
                is_sub: true,
                channel_index: ch.channel_index as i32,
                sub_relative_index: sub_idx as i32,
            });
        }

        if state.prev_channels != channel_snapshots {
            let channels: Vec<crate::editor::slint_ui::ChannelInfoUI> =
                channel_snapshots.iter().map(ChannelUiSnapshot::to_slint).collect();
            app.set_channels(make_channel_model(channels));
            state.prev_channels = channel_snapshots;
        }
    } else {
        if state.prev_grid_width != 5 {
            app.set_grid_width(5);
            state.prev_grid_width = 5;
        }
        if state.prev_grid_height != 5 {
            app.set_grid_height(5);
            state.prev_grid_height = 5;
        }
        if !state.prev_channels.is_empty() {
            app.set_channels(make_channel_model(Vec::new()));
            state.prev_channels.clear();
        }
    }
}

fn current_layout<'a>(
    layout_config: &'a ConfigManager,
    params: &MonitorParams,
) -> Option<&'a Layout> {
    layout_config.get_layout_by_indices(
        params.layout.value().max(0) as usize,
        params.sub_layout.value().max(0) as usize,
    )
}

fn make_channel_model(values: Vec<crate::editor::slint_ui::ChannelInfoUI>) -> ModelRc<crate::editor::slint_ui::ChannelInfoUI> {
    ModelRc::new(VecModel::from(values))
}

fn make_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    let labels: Vec<SharedString> = values.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(labels))
}

fn apply_role_change(
    params: &MonitorParams,
    interaction: &InteractionManager,
    setter: &ParamSetter,
    next_role: PluginRole,
    logger: &InstanceLogger,
) {
    if next_role != PluginRole::Standalone && interaction.is_automation_mode() {
        interaction.exit_automation_mode();
        logger.info(
            "slint",
            &format!(
                "[Editor] Auto-exited automation mode (switched to {:?})",
                next_role
            ),
        );
    }

    setter.begin_set_parameter(&params.role);
    setter.set_parameter(&params.role, next_role);
    setter.end_set_parameter(&params.role);
}

fn sync_all_channel_params(
    params: &MonitorParams,
    setter: &ParamSetter,
    interaction: &InteractionManager,
    layout: &Layout,
    logger: &InstanceLogger,
) {
    let mut on_mask: u32 = 0;
    let mut synced_channels: u32 = 0;

    for ch in layout.main_channels.iter().chain(layout.sub_channels.iter()) {
        let idx = ch.channel_index;
        if idx >= MAX_CHANNELS {
            continue;
        }

        let display = interaction.get_channel_display(&ch.name);
        if display.has_sound && idx < u32::BITS as usize {
            on_mask |= 1u32 << (idx as u32);
        }

        setter.begin_set_parameter(&params.channels[idx].enable);
        setter.set_parameter(&params.channels[idx].enable, display.has_sound);
        setter.end_set_parameter(&params.channels[idx].enable);
        synced_channels += 1;
    }

    let on_count = on_mask.count_ones();
    let off_count = synced_channels.saturating_sub(on_count);
    logger.info(
        "slint",
        &format!(
            "[SYNC] {}ch: {}on/{}off mask=0x{:x}",
            synced_channels, on_count, off_count, on_mask
        ),
    );
}

fn role_to_text(role: PluginRole) -> &'static str {
    match role {
        PluginRole::Standalone => "Standalone",
        PluginRole::Master => "Master (Source)",
        PluginRole::Slave => "Slave (Monitor)",
    }
}

fn solo_mode_to_text(mode: SoloMode) -> &'static str {
    match mode {
        SoloMode::SIP => "SIP (Solo In Place)",
        SoloMode::PFL => "PFL (Pre Fader Listen)",
    }
}

fn parse_port(input: &str, fallback: u16) -> u16 {
    input.trim().parse::<u16>().unwrap_or(fallback)
}

fn gain_to_slider(gain: f32) -> f32 {
    let db = util::gain_to_db(gain).clamp(-80.0, 0.0);
    (db + 80.0) / 80.0
}

fn slider_to_gain(value: f32) -> f32 {
    let db = value.clamp(0.0, 1.0) * 80.0 - 80.0;
    util::db_to_gain(db)
}

fn format_gain_db(gain: f32) -> String {
    let db = util::gain_to_db(gain);
    if db <= -79.9 {
        "-inf dB".to_string()
    } else {
        format!("{db:.2} dB")
    }
}
