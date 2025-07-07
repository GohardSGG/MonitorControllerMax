# GUI Framework

> **Relevant source files**
> * [extras/Projucer/Source/Utility/UI/jucer_ProjucerLookAndFeel.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/extras/Projucer/Source/Utility/UI/jucer_ProjucerLookAndFeel.cpp)
> * [extras/Projucer/Source/Utility/UI/jucer_ProjucerLookAndFeel.h](https://github.com/juce-framework/JUCE/blob/d6181bde/extras/Projucer/Source/Utility/UI/jucer_ProjucerLookAndFeel.h)
> * [modules/juce_gui_basics/components/juce_Component.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp)
> * [modules/juce_gui_basics/components/juce_Component.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.h)
> * [modules/juce_gui_basics/detail/juce_ComponentHelpers.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/detail/juce_ComponentHelpers.h)
> * [modules/juce_gui_basics/detail/juce_MouseInputSourceImpl.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/detail/juce_MouseInputSourceImpl.h)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V1.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V1.cpp)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V1.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V1.h)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.cpp)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.h)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V3.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V3.cpp)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V3.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V3.h)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.cpp)
> * [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.h)
> * [modules/juce_gui_basics/menus/juce_PopupMenu.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/menus/juce_PopupMenu.cpp)
> * [modules/juce_gui_basics/menus/juce_PopupMenu.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/menus/juce_PopupMenu.h)
> * [modules/juce_gui_basics/mouse/juce_MouseInputSource.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/mouse/juce_MouseInputSource.cpp)
> * [modules/juce_gui_basics/mouse/juce_MouseInputSource.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/mouse/juce_MouseInputSource.h)
> * [modules/juce_gui_basics/widgets/juce_ComboBox.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_ComboBox.cpp)
> * [modules/juce_gui_basics/widgets/juce_ComboBox.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_ComboBox.h)
> * [modules/juce_gui_basics/widgets/juce_Label.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Label.cpp)
> * [modules/juce_gui_basics/widgets/juce_Label.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Label.h)
> * [modules/juce_gui_basics/widgets/juce_ProgressBar.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_ProgressBar.cpp)
> * [modules/juce_gui_basics/widgets/juce_ProgressBar.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_ProgressBar.h)
> * [modules/juce_gui_basics/widgets/juce_Slider.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Slider.cpp)
> * [modules/juce_gui_basics/widgets/juce_Slider.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Slider.h)
> * [modules/juce_gui_basics/windows/juce_ComponentPeer.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/windows/juce_ComponentPeer.cpp)
> * [modules/juce_gui_basics/windows/juce_ComponentPeer.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/windows/juce_ComponentPeer.h)

The JUCE GUI Framework is a cross-platform C++ system for building graphical user interfaces. It provides a unified component model, event propagation, and a set of standard widgets, all abstracted from the underlying operating system. The framework is designed to work consistently across Windows, macOS, Linux, iOS, and Android.

This page provides a technical overview of the GUI system, its architecture, and the main code entities involved. For details on graphics rendering, see page [3.3]. For OpenGL integration, see page [3.4].

Sources:

* [modules/juce_gui_basics/components/juce_Component.h L44-L1148](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.h#L44-L1148)
* [modules/juce_gui_basics/components/juce_Component.cpp L42-L1148](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L42-L1148)

## Component System

The core of the JUCE GUI framework is the `juce::Component` class. All visible UI elements inherit from `Component`. Components are arranged in a tree structure, where each component can have multiple children and a single parent.

**Component Class Hierarchy and Key Methods**

```

```

Sources:

* [modules/juce_gui_basics/components/juce_Component.h L44-L1148](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.h#L44-L1148)
* [modules/juce_gui_basics/components/juce_Component.cpp L42-L1148](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L42-L1148)

### Component Hierarchy

Components are organized in a tree. Each `juce::Component` can have zero or more children and at most one parent. The hierarchy is manipulated using methods such as `addChildComponent()`, `removeChildComponent()`, and `getParentComponent()`.

**Component Hierarchy Example**

```

```

Key methods:

* `addChildComponent(Component*)`
* `removeChildComponent(int)`
* `getParentComponent()`
* `getChildComponent(int)`
* `getNumChildComponents()`

Sources:

* [modules/juce_gui_basics/components/juce_Component.cpp L258-L273](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L258-L273)
* [modules/juce_gui_basics/components/juce_Component.cpp L603-L616](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L603-L616)

### Component Lifecycle

The lifecycle of a `juce::Component` is as follows:

1. Construction (`Component()` or derived constructor)
2. Addition to a parent or to the desktop (`addChildComponent()`, `addToDesktop()`)
3. Becoming visible (`setVisible(true)`)
4. Event handling and painting
5. Removal from parent or desktop
6. Destruction (`~Component()`)

Note: Child components are not automatically deleted when a parent is destroyed. Ownership must be managed explicitly.

Sources:

* [modules/juce_gui_basics/components/juce_Component.cpp L242-L273](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L242-L273)

### Positioning and Sizing

Components use a local coordinate system with (0,0) at the top-left. All positions and sizes are relative to the parent component.

Key methods:

* `setBounds(int x, int y, int width, int height)`
* `setBoundsRelative(float x, float y, float w, float h)`
* `setSize(int width, int height)`
* `setTopLeftPosition(int x, int y)`

Sources:

* [modules/juce_gui_basics/components/juce_Component.cpp L811-L863](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L811-L863)
* [modules/juce_gui_basics/components/juce_Component.h L268-L436](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.h#L268-L436)

## Event Handling System

JUCE components handle input events such as mouse and keyboard actions. Events are delivered from the operating system to the appropriate `juce::Component` via a chain of abstractions.

**Event Propagation Flow**

```

```

Sources:

* [modules/juce_gui_basics/windows/juce_ComponentPeer.cpp L94-L112](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/windows/juce_ComponentPeer.cpp#L94-L112)
* [modules/juce_gui_basics/components/juce_Component.cpp L1076-L1147](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L1076-L1147)
* [modules/juce_gui_basics/mouse/juce_MouseInputSource.cpp L37-L108](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/mouse/juce_MouseInputSource.cpp#L37-L108)

### Mouse Events

Mouse events are delivered to components via virtual methods such as:

* `mouseDown(const MouseEvent&)`
* `mouseUp(const MouseEvent&)`
* `mouseDrag(const MouseEvent&)`
* `mouseMove(const MouseEvent&)`

The `juce::MouseEvent` object provides:

* Position (relative to component)
* Button state
* Modifier keys
* Click count
* Drag state

Sources:

* [modules/juce_gui_basics/components/juce_Component.cpp L723-L744](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L723-L744)
* [modules/juce_gui_basics/mouse/juce_MouseInputSource.cpp L56-L79](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/mouse/juce_MouseInputSource.cpp#L56-L79)

### Keyboard Events

Keyboard events are delivered to the component with keyboard focus. If not handled, they propagate up the parent chain.

Key methods:

* `keyPressed(const KeyPress&)`
* `keyStateChanged(bool isKeyDown)`
* `focusGained()`, `focusLost()`

Sources:

* [modules/juce_gui_basics/windows/juce_ComponentPeer.cpp L189-L263](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/windows/juce_ComponentPeer.cpp#L189-L263)

## Standard Widgets

JUCE provides a set of standard widgets, all derived from `juce::Component`. These include buttons, sliders, labels, combo boxes, and more.

**Widget Class Relationships**

```

```

**Button Features**

* Toggle state
* Radio groups
* Command manager integration
* Click callbacks

Sources:

* [modules/juce_gui_basics/buttons/juce_Button.h L50-L65](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/buttons/juce_Button.h#L50-L65)
* [modules/juce_gui_basics/buttons/juce_Button.cpp L85-L102](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/buttons/juce_Button.cpp#L85-L102)

**Slider Features**

* Multiple styles (horizontal, vertical, rotary, etc.)
* Range/value management
* Text box for value display/edit
* Min/max/thumbs for range selection

Sources:

* [modules/juce_gui_basics/widgets/juce_Slider.h L108-L707](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Slider.h#L108-L707)
* [modules/juce_gui_basics/widgets/juce_Slider.cpp L51-L498](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Slider.cpp#L51-L498)

**Label Features**

* Text display
* Optional editing
* Font and justification
* Border and attachment

Sources:

* [modules/juce_gui_basics/widgets/juce_Label.h L45-L248](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Label.h#L45-L248)
* [modules/juce_gui_basics/widgets/juce_Label.cpp L38-L202](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_Label.cpp#L38-L202)

**ComboBox Features**

* Item management (add/remove/enable/disable)
* Selection and editable text
* ID-based system
* Change callbacks

Sources:

* [modules/juce_gui_basics/widgets/juce_ComboBox.h L54-L575](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_ComboBox.h#L54-L575)
* [modules/juce_gui_basics/widgets/juce_ComboBox.cpp L38-L269](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/widgets/juce_ComboBox.cpp#L38-L269)

## Popup Menus

The `juce::PopupMenu` class provides a flexible system for context menus and drop-downs.

**PopupMenu Class Structure**

```

```

Features:

* Hierarchical submenus
* Custom component items
* Separators and section headers
* Callbacks for selection
* Keyboard navigation

Sources:

* [modules/juce_gui_basics/menus/juce_PopupMenu.h L88-L597](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/menus/juce_PopupMenu.h#L88-L597)
* [modules/juce_gui_basics/menus/juce_PopupMenu.cpp L65-L379](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/menus/juce_PopupMenu.cpp#L65-L379)

## LookAndFeel System

The `juce::LookAndFeel` system separates component appearance from behavior. Each component uses a `LookAndFeel` instance to draw itself. This can be set globally or per-component.

**LookAndFeel Class Structure**

```

```

Key points:

* All components use a `LookAndFeel` for drawing
* Can be set globally or per-component
* Multiple versions (V2, V3, V4) exist
* V4 supports color schemes
* Custom LookAndFeel classes can be created

Sources:

* [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.h L46-L591](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.h#L46-L591)
* [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.cpp L39-L598](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.cpp#L39-L598)
* [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V3.cpp L38-L50](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V3.cpp#L38-L50)
* [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.h L1-L200](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.h#L1-L200)
* [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.cpp L38-L116](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V4.cpp#L38-L116)

### Colour IDs

Components use integer colour IDs to identify styleable regions. These IDs are used with `setColour()` and `findColour()`.

| Widget Type | Colour ID Symbol | Description |
| --- | --- | --- |
| Button | `TextButton::buttonColourId` | Background color |
| Button | `TextButton::textColourOffId` | Text color (off) |
| Button | `TextButton::textColourOnId` | Text color (on) |
| Slider | `Slider::backgroundColourId` | Background |
| Slider | `Slider::thumbColourId` | Thumb |
| Slider | `Slider::trackColourId` | Track |
| Slider | `Slider::rotarySliderFillColourId` | Rotary fill |

Sources:

* [modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.cpp L42-L143](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/lookandfeel/juce_LookAndFeel_V2.cpp#L42-L143)

## Component Rendering

Rendering in JUCE is performed using the `juce::Graphics` class. The process is initiated by the OS and flows through several code entities.

**Component Rendering Flow**

```

```

Steps:

1. `ComponentPeer::handlePaint()` is called by the OS
2. `Component::paintEntireComponent()` is invoked
3. `Component::paint(Graphics&)` is called (user override)
4. Drawing is typically delegated to `LookAndFeel`
5. `LookAndFeel` uses `Graphics` for primitive operations

Sources:

* [modules/juce_gui_basics/windows/juce_ComponentPeer.cpp L115-L171](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/windows/juce_ComponentPeer.cpp#L115-L171)
* [modules/juce_gui_basics/components/juce_Component.cpp L1335-L1390](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L1335-L1390)

### Desktop Integration

A `juce::Component` can be made a top-level window using `addToDesktop()`. This creates a native window via `juce::ComponentPeer`.

Key methods:

* `void Component::addToDesktop(int styleFlags, void* nativeWindowToAttachTo = nullptr)`
* `void Component::removeFromDesktop()`
* `bool Component::isOnDesktop() const`

The `styleFlags` parameter controls window features (title bar, resizable, minimize/maximize, etc).

Sources:

* [modules/juce_gui_basics/components/juce_Component.cpp L383-L496](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/components/juce_Component.cpp#L383-L496)
* [modules/juce_gui_basics/windows/juce_ComponentPeer.h L57-L86](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/windows/juce_ComponentPeer.h#L57-L86)

## Summary

The JUCE GUI Framework provides a powerful, flexible system for creating cross-platform user interfaces with a consistent appearance and behavior. By understanding the component model, event handling, and look-and-feel system, developers can create sophisticated UIs that work across different platforms with minimal platform-specific code.

The layered architecture separates component behavior from appearance, allowing for extensive customization without changing core functionality. The wide range of provided widgets covers most common UI needs, while the custom component system allows for creating specialized interfaces as needed.