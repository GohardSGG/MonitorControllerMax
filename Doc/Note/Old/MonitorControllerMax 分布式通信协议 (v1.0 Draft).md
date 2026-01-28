好的，这是基于 **ZeroMQ + Bincode** 的跨机通信详细设计方案，专为 **MonitorControllerMax v4.0** 的分布式需求定制。

---

# MonitorControllerMax 分布式通信协议 (v1.0 Draft)

## 1. 核心技术栈 (Tech Stack)

*   **传输层**: **ZeroMQ (ZMQ)**
    *   **模式**: **PUB/SUB (发布/订阅)**。
    *   **理由**: 
        *   支持 1-to-N 广播 (1 Master -> N Slaves)。
        *   自动重连，无需业务层干预。
        *   TCP 协议保证消息顺序和完整性（相比 UDP）。
*   **序列化**: **Bincode**
    *   **理由**: Rust 原生，零拷贝反序列化，二进制体积极小（比 JSON 快 ~10x，小 ~5x）。
*   **拓扑**:
    *   **Master**: Bind `tcp://0.0.0.0:9123` (固定端口，或可配置)。
    *   **Slave**: Connect `tcp://<Master_IP>:9123`。

---

## 2. 数据结构 (Payload Definition)

为了保证高效和向前兼容，我们定义一个紧凑的 `NetworkPacket`。

```rust
use serde::{Serialize, Deserialize};

// 扁平化的音频状态，直接对应 RenderState
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)] // 保证内存布局稳定
pub struct NetworkRenderState {
    // 1. 全局增益 (Master Fader + Dim + Cut 后的最终值)
    // 范围: 0.0 (Silence) - 1.0 (Unity) - >1.0 (Boost)
    pub master_gain: f32,

    // 2. 通道状态位图 (Bitmask)
    // 使用位图极其节省带宽。u32 可以存 32 个通道的状态。
    // Bit = 1 代表通道被 Mute (或 Auto-Mute)，Bit = 0 代表 Open。
    // 我们不需要区分 Mute 和 Solo，Slave 只需要知道“这个通道该不该响”。
    pub channel_mute_mask: u32,

    // 3. 通道增益微调 (可选)
    // 如果以后支持每个通道独立的 Trim，可以用数组。目前版本可能不需要。
    // pub channel_trims: [f32; 18], 

    // 4. 安全/校验
    pub timestamp: u64, // 毫秒级时间戳，用于调试延迟或丢弃乱序包(虽然TCP保序)
    pub magic: u16,     // 0xMC，用于简单的协议识别
}
```

### 数据量估算
*   `f32` (4 bytes) + `u32` (4 bytes) + `u64` (8 bytes) + `u16` (2 bytes) = **18 bytes**。
*   即使每秒发送 100 次 (100Hz)，带宽占用也仅为 1.8 KB/s。这在任何局域网（甚至 Wi-Fi）上都是微不足道的。**极度轻量**。

---

## 3. Master 端逻辑 (Publisher)

Master 运行一个独立的 **Network Task**（不阻塞 GUI 或 DSP）。

### 3.1 触发机制
我们采用 **"Push on Change + Keep-alive"** 策略：
1.  **Change Trigger**: 当 `RenderState` 发生任何变化（用户推拉杆、按按钮），立即序列化并发送。
2.  **Keep-alive (可选)**: 每 1秒发送一次当前状态。
    *   *目的*: 方便刚加入网络的 Slave 立即获得状态，而不需要等用户下次操作。这也起到了隐式的心跳作用，虽然 Slave 并不强制依赖它。

### 3.2 伪代码
```rust
// 在 Master 的构造函数或后台线程中
let context = zmq::Context::new();
let publisher = context.socket(zmq::PUB).unwrap();
publisher.bind("tcp://0.0.0.0:9123").expect("Failed to bind port 9123");

// 广播函数
fn broadcast_state(state: &RenderState) {
    let packet = NetworkRenderState::from(state); // 转换
    let encoded = bincode::serialize(&packet).unwrap();
    publisher.send(&encoded, 0).unwrap(); // 非阻塞发送
}
```

---

## 4. Slave 端逻辑 (Subscriber)

Slave 的设计核心是 **"无锁缓存 (Lock-Free Cache)"**。

### 4.1 接收线程 (Background Thread)
Slave 启动一个独立的线程，专门负责死循环 `recv`。

```rust
// 静态全局缓存 (或者放在 Arc<Plugin> 里)
// AtomicCell 支持无锁读写较大的结构体（只要 CPU 支持）
// 或者使用 arc_swap::ArcSwap 
static LAST_RECEIVED_STATE: ArcSwap<NetworkRenderState> = ...;

fn slave_network_thread() {
    let context = zmq::Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber.connect("tcp://192.168.1.100:9123").unwrap(); // IP 可配置
    subscriber.set_subscribe(b"").unwrap(); // 订阅所有消息

    loop {
        // 阻塞等待，直到收到数据
        let data = subscriber.recv_bytes(0).unwrap();
        
        if let Ok(packet) = bincode::deserialize::<NetworkRenderState>(&data) {
            // 原子替换缓存
            LAST_RECEIVED_STATE.store(Arc::new(packet));
        }
    }
}
```

### 4.2 音频线程 (DSP Process)
DSP 线程只做一件事：**读缓存，应用**。

```rust
fn process(buffer) {
    // 1. 获取最新快照 (极快，纳秒级)
    let state = LAST_RECEIVED_STATE.load(); 

    // 2. 应用全局增益
    let gain = state.master_gain;
    
    // 3. 应用通道 Mute
    for ch in 0..channels {
        // 检查位图
        let is_muted = (state.channel_mute_mask >> ch) & 1;
        if is_muted == 1 {
            buffer[ch].clear(); // 静音
        } else {
            buffer[ch].apply_gain(gain); // 应用增益
        }
    }
}
```

---

## 5. 局域网发现 (Service Discovery) - 进阶

*   **痛点**: Slave 怎么知道 Master 的 IP 是多少？
*   **解决方案 (Phase 1)**: 手动输入。在 Slave 的 `config.json` 或简陋界面里填入 `Master IP`。
*   **解决方案 (Phase 2 - 自动发现)**: 使用 UDP Broadcast 或 mDNS。
    *   Master 每秒发一个 UDP 广播包: "I am Master at 192.168.1.X:9123"。
    *   Slave 监听 UDP 广播，自动找到 Master 并建立 ZMQ 连接。
    *   *建议*: 暂时先做 Phase 1 (手动配置)，这是最稳的。

---

## 6. 审核总结

这份方案：
1.  **解耦**: Master 和 Slave 哪怕在地球两端，只要网络通，逻辑就通。
2.  **鲁棒**: ZMQ 解决了断线重连。`ArcSwap` 解决了线程安全。
3.  **极简**: 没有握手，没有 ACK，没有状态机同步。只有“广播”和“缓存”。
4.  **状态保持**: `LAST_RECEIVED_STATE` 只要不被覆盖，就永远保持最后的值。完美符合您的要求。

您觉得这个**“基于 ZMQ 的分布式无锁快照”**方案是否通过审核？