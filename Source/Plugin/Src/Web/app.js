/**
 * MCM Web Controller - JavaScript Client
 */

class MCMController {
    constructor() {
        this.ws = null;
        this.reconnectDelay = 1000;
        this.currentPage = 0;
        this.state = null;
        this.volumeStartY = null;
        this.volumeStartValue = 0.5;

        // Group Dial 状态
        this.groupDialStartY = null;
        this.groupDialGroup = null;
        this.groupDialDragged = false;
        this.groupDialThreshold = 15;  // 触发阈值（像素）

        this.init();
    }

    init() {
        this.connect();
        this.bindEvents();
    }

    // ============================================================================
    // WebSocket Connection
    // ============================================================================

    connect() {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        this.ws = new WebSocket(`${protocol}//${location.host}/ws`);

        this.ws.onopen = () => {
            console.log('[MCM] Connected');
            this.reconnectDelay = 1000;
            this.setStatus('connected', 'Connected');
        };

        this.ws.onmessage = (event) => {
            try {
                const state = JSON.parse(event.data);
                this.state = state;
                this.updateUI(state);
            } catch (e) {
                console.error('[MCM] Parse error:', e);
            }
        };

        this.ws.onclose = () => {
            console.log('[MCM] Disconnected');
            this.setStatus('error', 'Reconnecting...');
            setTimeout(() => this.connect(), this.reconnectDelay);
            this.reconnectDelay = Math.min(this.reconnectDelay * 1.5, 10000);
        };

        this.ws.onerror = (e) => {
            console.error('[MCM] Error:', e);
            this.setStatus('error', 'Connection Error');
        };
    }

    send(cmd) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(cmd));
        }
    }

    setStatus(type, text) {
        const el = document.getElementById('status');
        el.className = 'status ' + type;
        el.textContent = text;
    }

    // ============================================================================
    // UI Update
    // ============================================================================

    updateUI(state) {
        // Update Mode Buttons - 区分常亮（Primary）和闪烁（Compare）
        const soloSteady = state.primary === 1;     // Solo 常亮
        const soloBlinking = state.compare === 1;   // Solo 闪烁
        const muteSteady = state.primary === 2;     // Mute 常亮
        const muteBlinking = state.compare === 2;   // Mute 闪烁

        document.querySelectorAll('[data-cmd="ToggleSolo"]').forEach(btn => {
            btn.classList.remove('active', 'solo-steady', 'solo-blinking', 'mute-steady', 'mute-blinking');
            if (soloSteady) btn.classList.add('solo-steady');
            if (soloBlinking) btn.classList.add('solo-blinking');
        });

        document.querySelectorAll('[data-cmd="ToggleMute"]').forEach(btn => {
            btn.classList.remove('active', 'solo-steady', 'solo-blinking', 'mute-steady', 'mute-blinking');
            if (muteSteady) btn.classList.add('mute-steady');
            if (muteBlinking) btn.classList.add('mute-blinking');
        });

        // Update Channel Buttons
        if (state.channels) {
            state.channels.forEach(ch => {
                const btn = document.querySelector(`[data-channel="${ch.name}"]`);
                if (btn) {
                    btn.classList.remove('solo', 'mute');
                    if (ch.state === 2) btn.classList.add('solo');
                    else if (ch.state === 1) btn.classList.add('mute');
                }
            });
        }

        // Update Volume
        const volumePct = Math.round(state.master_volume * 100);
        document.getElementById('volume-display').textContent = volumePct + '%';
        document.getElementById('volume-knob').style.setProperty('--progress', (volumePct * 270 / 100) + 'deg');

        // Update Action Buttons
        const dimBtn = document.getElementById('btn-dim');
        if (dimBtn) dimBtn.classList.toggle('active', state.dim);

        const cutBtn = document.getElementById('btn-cut');
        if (cutBtn) cutBtn.classList.toggle('active', state.cut);

        const monoBtn = document.getElementById('btn-mono');
        if (monoBtn) monoBtn.classList.toggle('active', state.mono);

        const lowBtn = document.getElementById('btn-low');
        if (lowBtn) lowBtn.classList.toggle('active', state.low_boost);

        const highBtn = document.getElementById('btn-high');
        if (highBtn) highBtn.classList.toggle('active', state.high_boost);

        const lfe10Btn = document.getElementById('btn-lfe10');
        if (lfe10Btn) lfe10Btn.classList.toggle('active', state.lfe_add_10db);

        // Update all DIM buttons across pages
        document.querySelectorAll('[data-cmd="ToggleDim"]').forEach(btn => {
            btn.classList.toggle('active', state.dim);
        });

        document.querySelectorAll('[data-cmd="ToggleCut"]').forEach(btn => {
            btn.classList.toggle('active', state.cut);
        });

        document.querySelectorAll('[data-cmd="ToggleLfeAdd10dB"]').forEach(btn => {
            btn.classList.toggle('active', state.lfe_add_10db);
        });
    }

    // ============================================================================
    // Event Binding
    // ============================================================================

    bindEvents() {
        // Button Clicks
        document.querySelectorAll('.btn[data-cmd]').forEach(btn => {
            btn.addEventListener('click', () => {
                const cmd = btn.dataset.cmd;
                if (cmd) {
                    this.send({ type: cmd });
                    // Haptic feedback
                    if (navigator.vibrate) navigator.vibrate(10);
                }
            });
        });

        // Channel Button Clicks
        document.querySelectorAll('.btn.channel[data-channel]').forEach(btn => {
            btn.addEventListener('click', () => {
                const channel = btn.dataset.channel;
                if (channel) {
                    this.send({ type: 'ChannelClick', channel: channel });
                    if (navigator.vibrate) navigator.vibrate(10);
                }
            });
        });

        // Volume Knob
        const volumeKnob = document.getElementById('volume-knob');
        if (volumeKnob) {
            volumeKnob.addEventListener('touchstart', (e) => this.onVolumeStart(e), { passive: false });
            volumeKnob.addEventListener('mousedown', (e) => this.onVolumeStart(e));
        }

        // Group Dial Encoders (用于控制通道组 Solo/Mute)
        document.querySelectorAll('.group-dial[data-group]').forEach(dial => {
            // 拖动开始
            dial.addEventListener('touchstart', (e) => this.onGroupDialStart(e, dial.dataset.group), { passive: false });
            dial.addEventListener('mousedown', (e) => this.onGroupDialStart(e, dial.dataset.group));
            // 点击（切换 Mute）
            dial.addEventListener('click', (e) => {
                // 只有短按才触发点击（长按拖动不触发）
                if (!this.groupDialDragged) {
                    this.send({ type: 'GroupClick', group: dial.dataset.group });
                    if (navigator.vibrate) navigator.vibrate(10);
                }
            });
        });

        // Page Swipe
        const screen = document.querySelector('.screen');
        let touchStartX = null;

        screen.addEventListener('touchstart', (e) => {
            touchStartX = e.touches[0].clientX;
        }, { passive: true });

        screen.addEventListener('touchend', (e) => {
            if (touchStartX === null) return;
            const touchEndX = e.changedTouches[0].clientX;
            const diff = touchStartX - touchEndX;

            if (Math.abs(diff) > 50) {
                if (diff > 0 && this.currentPage < 2) {
                    this.goToPage(this.currentPage + 1);
                } else if (diff < 0 && this.currentPage > 0) {
                    this.goToPage(this.currentPage - 1);
                }
            }
            touchStartX = null;
        }, { passive: true });

        // Page Indicators
        document.querySelectorAll('.dot').forEach(dot => {
            dot.addEventListener('click', () => {
                const page = parseInt(dot.dataset.page);
                this.goToPage(page);
            });
        });
    }

    // ============================================================================
    // Volume Control
    // ============================================================================

    onVolumeStart(e) {
        e.preventDefault();
        this.volumeStartY = e.clientY || (e.touches && e.touches[0].clientY);
        this.volumeStartValue = this.state ? this.state.master_volume : 0.5;

        const onMove = (ev) => this.onVolumeMove(ev);
        const onEnd = () => {
            document.removeEventListener('mousemove', onMove);
            document.removeEventListener('mouseup', onEnd);
            document.removeEventListener('touchmove', onMove);
            document.removeEventListener('touchend', onEnd);
        };

        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onEnd);
        document.addEventListener('touchmove', onMove, { passive: false });
        document.addEventListener('touchend', onEnd);
    }

    onVolumeMove(e) {
        if (this.volumeStartY === null) return;

        const clientY = e.clientY || (e.touches && e.touches[0].clientY);
        const deltaY = this.volumeStartY - clientY;
        const sensitivity = 0.005; // 0.5% per pixel

        let newValue = this.volumeStartValue + deltaY * sensitivity;
        newValue = Math.max(0, Math.min(1, newValue));

        // Round to 1%
        newValue = Math.round(newValue * 100) / 100;

        this.send({ type: 'SetVolume', value: newValue });

        if (e.preventDefault) e.preventDefault();
    }

    // ============================================================================
    // Group Dial Control (通道组编码器)
    // ============================================================================

    onGroupDialStart(e, group) {
        e.preventDefault();
        this.groupDialStartY = e.clientY || (e.touches && e.touches[0].clientY);
        this.groupDialGroup = group;
        this.groupDialDragged = false;

        const onMove = (ev) => this.onGroupDialMove(ev);
        const onEnd = () => {
            this.groupDialStartY = null;
            this.groupDialGroup = null;
            // 延迟重置 dragged 标志，让 click 事件能检测
            setTimeout(() => { this.groupDialDragged = false; }, 50);
            document.removeEventListener('mousemove', onMove);
            document.removeEventListener('mouseup', onEnd);
            document.removeEventListener('touchmove', onMove);
            document.removeEventListener('touchend', onEnd);
        };

        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onEnd);
        document.addEventListener('touchmove', onMove, { passive: false });
        document.addEventListener('touchend', onEnd);
    }

    onGroupDialMove(e) {
        if (this.groupDialStartY === null || !this.groupDialGroup) return;

        const clientY = e.clientY || (e.touches && e.touches[0].clientY);
        const deltaY = this.groupDialStartY - clientY;

        // 超过阈值才触发
        if (Math.abs(deltaY) > this.groupDialThreshold) {
            const direction = deltaY > 0 ? 1 : -1;  // 向上(右转)=1, 向下(左转)=-1
            console.log(`[MCM] GroupDial ${this.groupDialGroup} direction=${direction}`);

            this.send({
                type: 'GroupDial',
                group: this.groupDialGroup,
                direction: direction
            });

            // 重置起点，允许连续拖动
            this.groupDialStartY = clientY;
            this.groupDialDragged = true;

            // Haptic feedback
            if (navigator.vibrate) navigator.vibrate(5);
        }

        if (e.preventDefault) e.preventDefault();
    }

    // ============================================================================
    // Page Navigation
    // ============================================================================

    goToPage(page) {
        if (page < 0 || page > 2) return;

        this.currentPage = page;

        // Update pages visibility
        document.querySelectorAll('.page').forEach((p, i) => {
            p.classList.toggle('active', i === page);
        });

        // Update dots
        document.querySelectorAll('.dot').forEach((d, i) => {
            d.classList.toggle('active', i === page);
        });

        if (navigator.vibrate) navigator.vibrate(5);
    }
}

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    window.mcm = new MCMController();
});
