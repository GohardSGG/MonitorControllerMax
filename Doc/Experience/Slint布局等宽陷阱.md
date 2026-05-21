# Slint 布局等宽陷阱：HorizontalLayout 内子元素宽度不一致

## 问题现象

在 `HorizontalLayout` 中放置多个按钮组件，即使设置了 `horizontal-stretch: 1`，按钮宽度仍然**不等**。文字内容较长的按钮比短的更宽。

```
┌──────────┐ ┌────────────┐
│   DIM    │ │    CUT     │   ← DIM 比 CUT 窄
└──────────┘ └────────────┘
┌──────────┐ ┌────────────┐
│   MONO   │ │   +10dB    │   ← MONO 和 +10dB 宽度不同
└──────────┘ └────────────┘
```

## 根因

Slint 的 `horizontal-stretch` **不是**按比例分配**总空间**，而是按比例分配**剩余空间**。

分配逻辑：
1. 先让每个子元素获得各自的**最小宽度**（min-width）
2. 然后将**剩余空间**按 stretch 比例均分

如果按钮组件内部使用了 `HorizontalLayout` 或 `VerticalLayout` 作为内容容器，这些布局会根据子文字内容向上传播最小宽度约束。因此：

- "MONO" 的最小宽度 ≈ 文字渲染宽度 + 2×padding
- "+10dB" 的最小宽度 ≈ 文字渲染宽度 + 2×padding（不同）
- 两者最小宽度不同，即使 stretch 相同，最终宽度也不等

## 错误写法

```slint
// ❌ 组件内部用 HorizontalLayout → 文字内容决定最小宽度
export component MyButton inherits Rectangle {
    horizontal-stretch: 1;  // 这不够！

    HorizontalLayout {              // ← 这会向上传播最小宽度
        padding: 6px;
        Text {
            text: root.text;        // 不同文字 = 不同最小宽度
            horizontal-alignment: center;
        }
    }
}
```

## 正确写法

```slint
// ✅ 用绝对定位的 Text，不影响父级最小宽度
export component MyButton inherits Rectangle {
    horizontal-stretch: 1;

    Text {
        x: 0;
        y: 0;
        width: 100%;               // ← 填满父级，不向上报告最小宽度
        height: 100%;
        text: root.text;
        horizontal-alignment: center;
        vertical-alignment: center;
    }
}
```

## 核心原则

| 方式 | 是否影响父级 min-width | 等宽效果 |
|:--|:--|:--|
| `HorizontalLayout { Text {} }` | ✅ 会传播 | ❌ 不等宽 |
| `Text { width: 100%; height: 100% }` | ❌ 不传播 | ✅ 等宽 |

## 适用场景

- 按钮组（toolbar / button group）需要等宽排列
- 任何需要 `horizontal-stretch` 实现均分的场景
- Tab 标签页等宽切换

## 额外注意

如果组件有两行文字（主标题 + 副标题），改用绝对定位的 `VerticalLayout`：

```slint
if root.sub-text != "": VerticalLayout {
    x: 0;
    y: 0;
    width: 100%;
    height: 100%;
    alignment: center;
    spacing: 1px;
    Text { text: root.text; horizontal-alignment: center; }
    Text { text: root.sub-text; horizontal-alignment: center; }
}
```

---

*发现于 2026-05-21，MonitorControllerMax 左侧面板按钮组等宽对齐调试中。*
