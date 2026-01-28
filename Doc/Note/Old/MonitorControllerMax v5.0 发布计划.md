# MonitorControllerMax v5.0 发布计划

## 当前状态总结

### 已完成 ✅

- GUI 系统 (egui + wgpu)
- 交互状态机 (Solo/Mute/比较模式)
- VST3 参数同步 (Idle has_sound=true 已修复)
- 配置系统 (Speaker_Config.json)
- ZeroMQ 网络框架 (本机可用)

### 待完成 (按优先级排序)

1. **P0 - OSC 硬件集成** ← 最高优先级
2. P1 - 网络层配置化
3. P2 - 音频处理完善

---

## 一、OSC 硬件集成 (P0 最高优先级)

### 1.1 旧版 C++ OSC 配置 (参考来源)

来源: `Library/Old/MonitorControllerMax/Source/OSCCommunicator.h/cpp`

 

**端口配置**:

```cpp
static constexpr const char* TARGET_IP = "127.0.0.1";
static constexpr int TARGET_PORT = 7444;   // 发送端口 (插件 → 控制器)
static constexpr int RECEIVE_PORT = 7445;  // 接收端口 (控制器 → 插件)
```

### 1.2 完整 OSC 地址映射表

#### 接收消息 (硬件 → 插件)

|OSC 地址|类型|值|说明|
|---|---|---|---|
|`/Monitor/Mode/Solo`|float|1.0|点击 Solo 模式按钮 (进入/退出 Solo 选择状态)|
|`/Monitor/Mode/Mute`|float|1.0|点击 Mute 模式按钮 (进入/退出 Mute 选择状态)|
|`/Monitor/Solo/{Channel}`|float|1.0|选中通道进行 Solo|
|`/Monitor/Mute/{Channel}`|float|1.0|选中通道进行 Mute|
|`/Monitor/Master/Volume`|float|0.0-1.0|Master 音量|
|`/Monitor/Master/Dim`|float|1.0|Dim 效果开关|
|`/Monitor/Master/Mute`|float|1.0|Master 静音 (Cut)|

#### 发送消息 (插件 → 硬件 LED)

|OSC 地址|值|LED 效果|
|---|---|---|
|`/Monitor/Mode/Solo`|1.0|Solo 模式按钮亮起 (表示处于 Solo 选择状态)|
|`/Monitor/Mode/Solo`|0.0|Solo 模式按钮熄灭|
|`/Monitor/Mode/Mute`|1.0|Mute 模式按钮亮起 (表示处于 Mute 选择状态)|
|`/Monitor/Mode/Mute`|0.0|Mute 模式按钮熄灭|
|`/Monitor/Solo/{Channel}`|1.0|通道 **绿色** LED 亮起|
|`/Monitor/Solo/{Channel}`|0.0|通道绿色 LED 熄灭|
|`/Monitor/Mute/{Channel}`|1.0|通道 **红色** LED 亮起|
|`/Monitor/Mute/{Channel}`|0.0|通道红色 LED 熄灭|

#### 闪烁实现 (比较模式)

```
闪烁 = 每 500ms 交替发送 1.0 和 0.0
需要一个独立的闪烁定时器线程
```

**Channel 名称** (与 Speaker_Config.json 一致):

```
主声道: L, R, C, LFE, LR, RR
环绕: LSS, RSS, LRS, RRS
天花板: LTF, RTF, LTB, RTB
SUB通道: SUB_F, SUB_B, SUB_L, SUB_R (空格转下划线)
```

**参数判断逻辑**:

```rust
let state = value > 0.5;  // 任何大于 0.5 的值视为 ON
```

### 1.3 线程架构设计 (关键)

**核心原则**: 音频线程绝对不能被阻塞，所有 OSC 操作必须在独立线程中完成。

```
┌─────────────────────────────────────────────────────────────────┐
│                        线程架构图                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     │
│  │  音频线程     │     │  OSC 接收线程 │     │  闪烁定时器   │     │
│  │  (实时优先)   │     │  (独立)       │     │  线程 (独立)  │     │
│  └──────┬───────┘     └──────┬───────┘     └──────┬───────┘     │
│         │                    │                    │              │
│         │ 只读               │ 写入               │ 读取         │
│         ▼                    ▼                    ▼              │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              InteractionManager (全局单例)               │    │
│  │                                                          │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │
│  │  │ channel_    │  │ primary_    │  │ blink_      │      │    │
│  │  │ states[]    │  │ mode        │  │ channels[]  │      │    │
│  │  │ (AtomicU8)  │  │ (Atomic)    │  │ (AtomicBool)│      │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │
│  └─────────────────────────────────────────────────────────┘    │
│         │                    │                    │              │
│         │                    │                    │              │
│         ▼                    ▼                    ▼              │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    OSC 发送队列                          │    │
│  │            (crossbeam channel, 无锁)                     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│                    ┌──────────────┐                              │
│                    │  OSC 发送线程 │                              │
│                    │  (独立)       │                              │
│                    └──────────────┘                              │
│                              │                                   │
│                              ▼                                   │
│                       UDP 7444 发送                              │
└─────────────────────────────────────────────────────────────────┘
```

### 1.4 Rust 实现计划

**1. 添加依赖** - Cargo.toml:

```toml
rosc = "0.10"           # OSC 协议库
crossbeam-channel = "*" # 无锁消息队列 (已有)
```

**2. 创建 Osc.rs** - 多线程架构:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::net::UdpSocket;
use crossbeam::channel::{unbounded, Sender, Receiver};

/// OSC 发送消息类型
pub enum OscOutMessage {
    SoloLed { channel: String, on: bool },
    MuteLed { channel: String, on: bool },
    ModeSolo { on: bool },
    ModeMute { on: bool },
    MasterVolume { value: f32 },
}

pub struct OscManager {
    // 发送队列 (非阻塞)
    send_tx: Option<Sender<OscOutMessage>>,

    // 控制标志
    is_running: Arc<AtomicBool>,

    // 闪烁状态
    blink_phase: Arc<AtomicBool>,  // true=亮, false=灭
}

impl OscManager {
    pub fn new() -> Self {
        Self {
            send_tx: None,
            is_running: Arc::new(AtomicBool::new(false)),
            blink_phase: Arc::new(AtomicBool::new(true)),
        }
    }

    /// 初始化 (仅 Master/Standalone)
    pub fn initialize(&mut self, send_port: u16, recv_port: u16) {
        self.is_running.store(true, Ordering::SeqCst);

        // 1. 启动发送线程
        let (tx, rx) = unbounded::<OscOutMessage>();
        self.send_tx = Some(tx);
        self.spawn_send_thread(rx, send_port);

        // 2. 启动接收线程
        self.spawn_recv_thread(recv_port);

        // 3. 启动闪烁定时器线程
        self.spawn_blink_thread();
    }

    /// 非阻塞发送 (可从任何线程调用)
    pub fn send(&self, msg: OscOutMessage) {
        if let Some(tx) = &self.send_tx {
            let _ = tx.try_send(msg);  // 非阻塞，队列满则丢弃
        }
    }

    /// 发送线程 - 消费队列，发送 UDP
    fn spawn_send_thread(&self, rx: Receiver<OscOutMessage>, port: u16) {
        let is_running = self.is_running.clone();

        thread::spawn(move || {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            let target = format!("127.0.0.1:{}", port);

            while is_running.load(Ordering::Relaxed) {
                if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
                    let packet = encode_osc_message(&msg);
                    let _ = socket.send_to(&packet, &target);
                }
            }
        });
    }

    /// 接收线程 - 监听 UDP，处理消息
    fn spawn_recv_thread(&self, port: u16) {
        let is_running = self.is_running.clone();

        thread::spawn(move || {
            let socket = UdpSocket::bind(format!("127.0.0.1:{}", port)).unwrap();
            socket.set_read_timeout(Some(Duration::from_millis(100))).ok();

            let mut buf = [0u8; 1024];
            while is_running.load(Ordering::Relaxed) {
                if let Ok((len, _)) = socket.recv_from(&mut buf) {
                    if let Some((address, value)) = decode_osc_message(&buf[..len]) {
                        handle_osc_input(&address, value);
                    }
                }
            }
        });
    }

    /// 闪烁定时器线程 - 每 500ms 切换状态
    fn spawn_blink_thread(&self) {
        let is_running = self.is_running.clone();
        let blink_phase = self.blink_phase.clone();
        let send_tx = self.send_tx.clone();

        thread::spawn(move || {
            while is_running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(500));

                // 切换闪烁相位
                let new_phase = !blink_phase.load(Ordering::Relaxed);
                blink_phase.store(new_phase, Ordering::Relaxed);

                // 发送闪烁通道的 LED 状态
                if let Some(tx) = &send_tx {
                    let interaction = get_interaction_manager();
                    for (channel, is_blinking) in interaction.get_blinking_channels() {
                        if is_blinking {
                            let _ = tx.try_send(OscOutMessage::SoloLed {
                                channel: channel.clone(),
                                on: new_phase,
                            });
                        }
                    }
                }
            }
        });
    }
}
```

**3. OSC 消息处理**:

```rust
/// 处理接收到的 OSC 消息 (在接收线程中调用)
fn handle_osc_input(address: &str, value: f32) {
    let interaction = get_interaction_manager();

    match address {
        // 模式按钮
        "/Monitor/Mode/Solo" => {
            if value > 0.5 {
                interaction.toggle_solo_mode();
            }
        }
        "/Monitor/Mode/Mute" => {
            if value > 0.5 {
                interaction.toggle_mute_mode();
            }
        }
        // 通道 Solo/Mute
        addr if addr.starts_with("/Monitor/Solo/") => {
            let channel = &addr[14..];
            if value > 0.5 {
                interaction.on_channel_solo_from_osc(channel);
            }
        }
        addr if addr.starts_with("/Monitor/Mute/") => {
            let channel = &addr[14..];
            if value > 0.5 {
                interaction.on_channel_mute_from_osc(channel);
            }
        }
        // Master 控制
        "/Monitor/Master/Volume" => {
            // 更新 master gain 参数
        }
        "/Monitor/Master/Dim" => {
            if value > 0.5 {
                // 切换 dim
            }
        }
        "/Monitor/Master/Mute" => {
            if value > 0.5 {
                // 切换 cut
            }
        }
        _ => {}
    }
}
```

**4. InteractionManager 集成**:

```rust
// Interaction.rs 中添加 OSC 触发方法
impl InteractionManager {
    /// 切换 Solo 模式 (从 OSC /Monitor/Mode/Solo 调用)
    pub fn toggle_solo_mode(&mut self) {
        // 如果当前是 Solo 模式，退出；否则进入
        if self.primary_mode == PrimaryMode::Solo {
            self.primary_mode = PrimaryMode::None;
        } else {
            self.primary_mode = PrimaryMode::Solo;
        }
        self.notify_mode_changed();
    }

    /// 切换 Mute 模式 (从 OSC /Monitor/Mode/Mute 调用)
    pub fn toggle_mute_mode(&mut self) {
        if self.primary_mode == PrimaryMode::Mute {
            self.primary_mode = PrimaryMode::None;
        } else {
            self.primary_mode = PrimaryMode::Mute;
        }
        self.notify_mode_changed();
    }

    /// 通知 OSC 发送模式状态
    fn notify_mode_changed(&self) {
        if let Some(osc) = get_osc_manager() {
            osc.send(OscOutMessage::ModeSolo {
                on: self.primary_mode == PrimaryMode::Solo
            });
            osc.send(OscOutMessage::ModeMute {
                on: self.primary_mode == PrimaryMode::Mute
            });
        }
    }

    /// 获取所有闪烁通道 (供闪烁线程使用)
    pub fn get_blinking_channels(&self) -> Vec<(String, bool)> {
        // 返回 (通道名, 是否闪烁) 列表
    }
}
```

**5. 角色限制**:

- **Standalone**: 完全启用 OSC (发送 + 接收)
- **Master**: 完全启用 OSC (发送 + 接收)
- **Slave**: 完全禁用 OSC

### 1.5 文件修改清单

|文件|修改内容|
|---|---|
|Cargo.toml|添加 `rosc` 依赖|
|Osc.rs|新建，多线程 OSC 通信|
|Lib.rs|添加 OscManager，按角色初始化|
|Interaction.rs|添加 toggle_solo_mode, toggle_mute_mode, OSC 通知|
|Editor.rs|(可选) OSC 连接状态显示|

---

## 二、网络层配置化 (P1)

### 2.1 当前问题

```rust
// Lib.rs:102 - IP 硬编码
Params::PluginRole::Slave => self.network.init_slave("127.0.0.1", 9123),
```

### 2.2 解决方案

**Params.rs** - 添加配置参数:

```rust
#[id = "master_ip"]
pub master_ip: StringParam,  // 默认 "127.0.0.1"

#[id = "network_port"]
pub network_port: IntParam,  // 默认 9123
```

**Editor.rs** - 添加 IP 输入框 (仅 Slave 模式显示)

 

**Network.rs** - 添加连接状态反馈

---

## 三、OSC 状态回调机制 (详细设计)

### 3.1 回调触发点

当 InteractionManager 状态变化时，需要通知 OSC 发送状态给硬件控制器：

```rust
// Interaction.rs 中需要添加回调
pub struct InteractionManager {
    // ... 现有字段 ...

    // OSC 状态变化回调
    pub on_state_changed: Option<Box<dyn Fn(StateChangeEvent) + Send + Sync>>,
}

pub enum StateChangeEvent {
    // Solo 状态变化
    SoloChanged {
        channel_name: String,
        channel_index: usize,
        is_solo: bool,
        is_blinking: bool,  // 闪烁状态 (比较模式)
    },
    // Mute 状态变化
    MuteChanged {
        channel_name: String,
        channel_index: usize,
        is_muted: bool,
    },
    // 全局模式变化
    ModeChanged {
        primary_mode: PrimaryMode,  // Solo/Mute/None
        is_comparing: bool,         // 是否在比较模式
    },
}
```

### 3.2 闪烁状态处理

**问题**: 比较模式下通道会闪烁 (500ms ON/OFF)，硬件 LED 需要同步闪烁

 

**方案 A - 发送闪烁标记** (推荐):

```rust
// 发送一次 OSC 消息，告诉控制器"这个通道需要闪烁"
// /Monitor/Solo/L 1.0  → 正常亮起
// /Monitor/Solo/L 0.5  → 闪烁模式 (特殊值)
// /Monitor/Solo/L 0.0  → 熄灭
```

**方案 B - 发送实时状态**:

```rust
// 每 500ms 发送一次当前实际显示状态
// 需要一个定时器线程，每次闪烁切换都发送 OSC
```

**建议**: 方案 A 更简洁，由硬件控制器自己实现闪烁动画

### 3.3 回调集成位置

```rust
// 在 on_solo_button_click / on_mute_button_click 等函数末尾：
fn on_solo_button_click(&mut self, channel_index: usize, is_sub: bool) {
    // ... 现有逻辑 ...

    // 触发 OSC 回调
    if let Some(callback) = &self.on_state_changed {
        let display = self.get_channel_display(channel_index, is_sub);
        callback(StateChangeEvent::SoloChanged {
            channel_name: self.get_channel_name(channel_index),
            channel_index,
            is_solo: display.marker == Some(MarkerType::Solo),
            is_blinking: display.is_blinking,
        });
    }
}
```

---

## 四、设置窗口设计

### 4.1 UI 布局

```
┌─────────────────────────────────────────────────────┐
│  MonitorControllerMax v2.4.0              [⚙️]     │ ← 标题栏右侧齿轮图标
├─────────────────────────────────────────────────────┤
│  ... 现有 GUI ...                                   │
└─────────────────────────────────────────────────────┘

点击齿轮图标后弹出:
┌─────────────────────────────────────────────────────┐
│  ⚙️ 设置                                    [×]    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  【网络设置】                                        │
│  ┌───────────────────────────────────────────┐     │
│  │ 插件角色:  ○ Standalone  ○ Master  ○ Slave │     │
│  │                                           │     │
│  │ Master IP:  [192.168.1.100    ]           │     │
│  │ 端口:       [9123             ]           │     │
│  └───────────────────────────────────────────┘     │
│                                                     │
│  【OSC 设置】                                        │
│  ┌───────────────────────────────────────────┐     │
│  │ 发送端口:   [7444]                         │     │
│  │ 接收端口:   [7445]                         │     │
│  │ 状态:       🟢 已连接 / 🔴 未连接            │     │
│  └───────────────────────────────────────────┘     │
│                                                     │
│              [保存] [取消]                          │
└─────────────────────────────────────────────────────┘
```

### 4.2 Editor.rs 实现

```rust
// 新增状态
pub struct EditorState {
    // ... 现有字段 ...
    show_settings_panel: bool,
    settings_draft: SettingsDraft,  // 编辑中的设置副本
}

pub struct SettingsDraft {
    role: PluginRole,
    master_ip: String,
    network_port: u16,
    osc_send_port: u16,
    osc_recv_port: u16,
}

// 齿轮图标按钮
fn draw_title_bar(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("MonitorControllerMax v2.4.0");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("⚙").clicked() {
                self.show_settings_panel = true;
            }
        });
    });
}

// 设置面板 (使用 egui::Window)
fn draw_settings_panel(&mut self, ctx: &egui::Context) {
    if !self.show_settings_panel { return; }

    egui::Window::new("⚙ 设置")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // ... 绘制设置项 ...

            ui.horizontal(|ui| {
                if ui.button("保存").clicked() {
                    self.save_settings();
                    self.show_settings_panel = false;
                }
                if ui.button("取消").clicked() {
                    self.show_settings_panel = false;
                }
            });
        });
}
```

---

## 五、配置文件持久化

### 5.1 配置文件位置

**确定方案**: 使用用户目录，避免权限问题

```
%APPDATA%\MonitorControllerMax\config.json
即: C:\Users\{用户名}\AppData\Roaming\MonitorControllerMax\config.json
```

### 5.2 配置文件格式

```json
{
    "version": 1,
    "network": {
        "role": "Standalone",
        "master_ip": "192.168.1.100",
        "port": 9123
    },
    "osc": {
        "send_port": 7444,
        "recv_port": 7445
    }
}
```

### 5.3 Rust 实现

**新建 config_file.rs**:

```rust
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

/// 全局配置单例 (支持热重载)
pub static CONFIG: Lazy<Arc<RwLock<AppConfig>>> = Lazy::new(|| {
    Arc::new(RwLock::new(AppConfig::load_from_disk()))
});

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub version: u32,
    pub network: NetworkConfig,
    pub osc: OscConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub role: String,
    pub master_ip: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OscConfig {
    pub send_port: u16,
    pub recv_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            network: NetworkConfig {
                role: "Standalone".to_string(),
                master_ip: "127.0.0.1".to_string(),
                port: 9123,
            },
            osc: OscConfig {
                send_port: 7444,
                recv_port: 7445,
            },
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_default()
            .join("MonitorControllerMax")
            .join("config.json")
    }

    pub fn load_from_disk() -> Self {
        let path = Self::config_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save_to_disk(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    /// 热重载：保存并立即应用新配置
    pub fn apply_and_save(new_config: AppConfig) {
        // 1. 保存到磁盘
        let _ = new_config.save_to_disk();

        // 2. 更新全局配置
        if let Ok(mut config) = CONFIG.write() {
            *config = new_config.clone();
        }

        // 3. 触发网络/OSC 重新初始化
        reinitialize_services(&new_config);
    }
}

/// 热重载时重新初始化服务
fn reinitialize_services(config: &AppConfig) {
    // 1. 重新初始化网络
    // (需要先停止旧连接，再启动新连接)

    // 2. 重新初始化 OSC
    // (需要先停止旧线程，再启动新线程)

    mcm_info!("[Config] 配置已热重载");
}
```

### 5.4 即刻生效机制

```
用户点击"保存"按钮
    ↓
AppConfig::apply_and_save(new_config)
    ↓
┌─────────────────────────────────────┐
│ 1. save_to_disk() - 写入 JSON 文件   │
│ 2. 更新全局 CONFIG 单例              │
│ 3. reinitialize_services()          │
│    ├── 停止旧网络连接                │
│    ├── 启动新网络连接                │
│    ├── 停止旧 OSC 线程               │
│    └── 启动新 OSC 线程               │
└─────────────────────────────────────┘
    ↓
配置立即生效，无需重启插件
```

---

## 六、音频处理 - 增益平滑 (P2)

### 6.1 平滑器实现

```rust
// 新建 gain_smoother.rs 或在 Audio.rs 中
pub struct GainSmoother {
    current: f32,
    target: f32,
    coefficient: f32,  // 平滑系数
}

impl GainSmoother {
    pub fn new(sample_rate: f32) -> Self {
        // 10ms 平滑时间
        let time_constant = 0.01; // 10ms
        let coefficient = 1.0 - (-1.0 / (sample_rate * time_constant)).exp();

        Self {
            current: 1.0,
            target: 1.0,
            coefficient,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn next(&mut self) -> f32 {
        self.current += (self.target - self.current) * self.coefficient;
        self.current
    }

    pub fn is_smoothing(&self) -> bool {
        (self.current - self.target).abs() > 0.0001
    }
}
```

### 6.2 应用到 Audio.rs

```rust
pub struct AudioProcessor {
    smoothers: [GainSmoother; MAX_CHANNELS],
    master_smoother: GainSmoother,
}

pub fn process_audio(...) {
    // 更新目标增益
    for i in 0..num_channels {
        let target = if is_muted { 0.0 } else { channel_gain };
        self.smoothers[i].set_target(target);
    }
    self.master_smoother.set_target(render_state.master_gain);

    // 应用平滑增益
    for (channel_idx, channel_data) in buffer.iter_samples().enumerate() {
        for sample in channel_data {
            let smoothed_gain = self.smoothers[channel_idx].next();
            let master_gain = self.master_smoother.next();
            *sample *= smoothed_gain * master_gain;
        }
    }
}
```

---

## 七、实施顺序 (最终版)

### 阶段 1：OSC 集成 (P0) - 最高优先级

1. [ ]  Cargo.toml 添加 `rosc` 依赖
2. [ ]  创建 Osc.rs - OscManager 多线程架构
3. [ ]  实现 OSC 发送线程 (UDP 7444)
4. [ ]  实现 OSC 接收线程 (UDP 7445)
5. [ ]  实现闪烁定时器线程 (500ms 周期)
6. [ ]  实现地址解析 `/Monitor/Mode/Solo`, `/Monitor/Mode/Mute`
7. [ ]  实现地址解析 `/Monitor/Solo/{Channel}`, `/Monitor/Mute/{Channel}`
8. [ ]  Interaction.rs 添加 `toggle_solo_mode()`, `toggle_mute_mode()`
9. [ ]  Interaction.rs 添加 `get_blinking_channels()` 供闪烁线程使用
10. [ ]  Interaction.rs 状态变化时通知 OSC 发送 LED 状态
11. [ ]  实现 `broadcast_all_states()` 初始同步
12. [ ]  Lib.rs 集成 OscManager，按角色初始化

### 阶段 2：设置窗口 + 配置持久化 (P1)

13. [ ]  Cargo.toml 添加 `dirs` 依赖
14. [ ]  创建 config_file.rs - AppConfig 结构体 + 全局单例
15. [ ]  实现 load_from_disk / save_to_disk
16. [ ]  实现 apply_and_save 热重载机制
17. [ ]  Editor.rs 添加齿轮图标按钮 (标题栏右侧)
18. [ ]  Editor.rs 实现设置弹窗 (egui::Window)
19. [ ]  设置窗口: 网络设置 (Role, IP, Port)
20. [ ]  设置窗口: OSC 设置 (发送端口, 接收端口, 连接状态)
21. [ ]  保存按钮触发 apply_and_save (即刻生效)

### 阶段 3：音频处理 - 增益平滑 (P2)

22. [ ]  Audio.rs 添加 GainSmoother 结构体
23. [ ]  实现 per-channel 平滑器数组 [GainSmoother; MAX_CHANNELS]
24. [ ]  实现 master 平滑器
25. [ ]  替换直接增益为平滑增益 (10ms 平滑时间)

---

## 八、文件修改清单 (完整)

|文件|修改内容|优先级|
|---|---|---|
|**Cargo.toml**|添加 `rosc`, `dirs` 依赖|P0/P1|
|**Osc.rs**|新建, 多线程 OSC 通信 (发送/接收/闪烁)|P0|
|**Interaction.rs**|toggle_solo_mode, toggle_mute_mode, OSC 通知|P0|
|**Lib.rs**|添加 OscManager, 加载配置|P0/P1|
|**config_file.rs**|新建, 配置持久化 + 热重载|P1|
|**Editor.rs**|齿轮图标, 设置弹窗|P1|
|**Audio.rs**|GainSmoother 增益平滑|P2|

---

## 九、已确认的设计决策

|问题|决策|
|---|---|
|LED 颜色|Solo=绿色(1.0), Mute=红色(1.0)|
|闪烁实现|定时器线程每500ms发送1.0/0.0|
|模式按钮|`/Monitor/Mode/Solo`, `/Monitor/Mode/Mute`|
|配置文件位置|%APPDATA%\MonitorControllerMax\config.json|
|设置修改|即刻生效 (热重载)|
|线程安全|音频线程只读，OSC 操作全部异步|

---

## 十、风险评估

|风险|等级|缓解措施|
|---|---|---|
|OSC 线程与音频线程竞争|低|使用 crossbeam 无锁队列|
|闪烁定时精度|低|thread::sleep 足够精确|
|热重载时服务中断|中|先启动新服务再停止旧服务|
|多实例配置冲突|中|配置文件加入实例标识|

Stayed in plan mode