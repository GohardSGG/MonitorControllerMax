# MonitorControllerMax 核心逻辑与交互设计规范 (v4.0 Final Gold)

## 1. 系统架构与职责 (Architecture)

### 1.1 物理拓扑

DAW Output -> Master Plugin (Source) -> Calibration Software -> Slave Plugin (Monitor) -> Speakers

### 1.2 角色职责 (Role Responsibilities)

- Master Plugin (Source / Control Plane):

- 职责: 全局逻辑大脑、用户交互入口 (GUI/OSC)。

- 音频处理: 仅执行 Source Mute (为了控制进入校准软件的信号源)。不处理 Gain/Dim。

- 通信: 计算出最终的 RenderState，广播给 Slave。

- Slave Plugin (Monitor / Audio Plane):

- 职责: 最终执行者、音量守门员。

- 音频处理: 执行 Monitor Mute + 执行 Master Gain/Dim。

- 故障保护: State Retention (状态保持)。无论 Master 是否在线、断连或崩溃，Slave 永远保持上一次接收到的有效状态。重启 DAW 后读取本地缓存的最后状态（如果有），否则默认静音或 -inf dB 以防爆音。

---

## 2. 交互逻辑 (Interaction Logic)

### 2.1 状态定义

- Context A (Solo Context): SoloSet_Main + SoloSet_Sub。

- Context B (Mute Context): MuteSet_Main + MuteSet_Sub。

- Global Mode: Idle, SoloActive (Green), MuteActive (Red), SoloCompare (Blink Green), MuteCompare (Blink Red).

### 2.2 操作规则

1. A/B 切换: 点击 Solo/Mute 按钮在 Active 和 Compare 模式间切换。记忆各自独立，互不干扰（除非在 A 中修改了 Solo，脏化 B 的自动反转）。

2. 退出: 点击常亮按钮 -> 彻底退出到 Idle。

3. SUB 操作:

- 单击: 加入当前 Context。

- 双击/长按: 在当前 Context 中反转其状态 (User Mute)。

1. 自动化: 默认忽略，手动开启后强制覆盖所有交互状态。

---

## 3. 音频处理逻辑 (Audio Processing Logic)

这是每帧 process() 中执行的纯函数逻辑。

### 3.1 输入变量

- SoloSet_Main: 被 Solo 的主声道集合。

- SoloSet_Sub: 被 Solo 的 SUB 集合。

- UserMuteSet: 被手动 Mute 的通道集合。

### 3.2 Master 逻辑 (Source Control)

目标: 决定给校准软件喂什么。

对于主声道 i (Main Channel):

- Pass = 1.0 IF:

1. i 未被 UserMuteSet 包含。

2. AND (

(SoloSet_Main 不为空 AND i 在 SoloSet_Main 中) OR (显式 Solo，必须通)

(SoloSet_Main 为空 AND SoloSet_Sub 不为空) OR (没 Solo 主，但 Solo SUB，全通喂饱)

(SoloSet_Main 为空 AND SoloSet_Sub 为空) (都没 Solo，全通)

)

对于 SUB 通道:

- 全通。 (Master 不负责 SUB 的源头控制，因为 SUB 信号是在 Master 之后生成的)

### 3.3 Slave 逻辑 (Monitor Control)

目标: 决定最终谁响。应用 Gain/Dim。

对于主声道 i (Main Channel):

- Pass = 1.0 IF:

1. (前提: Master 端已放行)

2. AND (

(SoloSet_Main 不为空 AND i 在 SoloSet_Main 中) OR (跟随 Main Solo)

(SoloSet_Main 为空 AND SoloSet_Sub 为空) (常态全通)

)

- 解释: 如果 SoloSet_Main 为空但 SoloSet_Sub 不为空 (Solo Only SUB)，这里会返回 0 (Auto-Mute)。正确。

对于 SUB 通道 j (SUB Channel):

- Pass = 1.0 IF:

1. j 未被 UserMuteSet 包含 (双击操作)。

2. AND (SoloSet_Sub 为空 OR j 在 SoloSet_Sub 中)。

3. AND (Master 端有任意主声道放行)。(联动豁免: 有源才有声)

---

## 4. 最后的自检 (Final Self-Check)

1. Q: Solo L + Solo SUB1。Master 放行 R 吗？

- A: 不放行。SoloSet_Main 非空，执行 Main Solo 逻辑。R 被切。

- 结果: SUB1 只能播放 L 的低频。符合预期。

1. Q: 拔掉网线，Master 断连。

- A: Slave 保持上一帧状态（比如 Gain -20dB）。不会突然变成 0dB 爆音。符合预期。

1. Q: 只有 SUB1，没有 SUB2。Solo SUB1。

- A: Master 全通。Slave Mute 所有主声道。Slave 放行 SUB1。

- 结果: 只听 SUB1。符合预期。