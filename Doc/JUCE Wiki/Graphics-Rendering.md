# Graphics Rendering

> **Relevant source files**
> * [examples/GUI/FontsDemo.h](https://github.com/juce-framework/JUCE/blob/d6181bde/examples/GUI/FontsDemo.h)
> * [modules/juce_audio_formats/codecs/juce_MP3AudioFormat.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_audio_formats/codecs/juce_MP3AudioFormat.cpp)
> * [modules/juce_core/containers/juce_ElementComparator.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_core/containers/juce_ElementComparator.h)
> * [modules/juce_core/maths/juce_Expression.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_core/maths/juce_Expression.cpp)
> * [modules/juce_core/maths/juce_Range.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_core/maths/juce_Range.h)
> * [modules/juce_core/unit_tests/juce_UnitTestCategories.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_core/unit_tests/juce_UnitTestCategories.h)
> * [modules/juce_graphics/colour/juce_Colour.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/colour/juce_Colour.cpp)
> * [modules/juce_graphics/colour/juce_Colour.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/colour/juce_Colour.h)
> * [modules/juce_graphics/colour/juce_PixelFormats.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/colour/juce_PixelFormats.h)
> * [modules/juce_graphics/contexts/juce_GraphicsContext.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_GraphicsContext.cpp)
> * [modules/juce_graphics/contexts/juce_GraphicsContext.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_GraphicsContext.h)
> * [modules/juce_graphics/contexts/juce_LowLevelGraphicsContext.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_LowLevelGraphicsContext.h)
> * [modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.cpp)
> * [modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.h)
> * [modules/juce_graphics/detail/juce_JustifiedText.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_JustifiedText.cpp)
> * [modules/juce_graphics/detail/juce_JustifiedText.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_JustifiedText.h)
> * [modules/juce_graphics/detail/juce_Ranges.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_Ranges.cpp)
> * [modules/juce_graphics/detail/juce_Ranges.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_Ranges.h)
> * [modules/juce_graphics/detail/juce_ShapedText.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_ShapedText.cpp)
> * [modules/juce_graphics/detail/juce_ShapedText.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_ShapedText.h)
> * [modules/juce_graphics/detail/juce_SimpleShapedText.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_SimpleShapedText.cpp)
> * [modules/juce_graphics/detail/juce_SimpleShapedText.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/detail/juce_SimpleShapedText.h)
> * [modules/juce_graphics/fonts/juce_AttributedString.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_AttributedString.cpp)
> * [modules/juce_graphics/fonts/juce_AttributedString.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_AttributedString.h)
> * [modules/juce_graphics/fonts/juce_Font.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_Font.cpp)
> * [modules/juce_graphics/fonts/juce_Font.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_Font.h)
> * [modules/juce_graphics/fonts/juce_FontOptions.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_FontOptions.cpp)
> * [modules/juce_graphics/fonts/juce_FontOptions.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_FontOptions.h)
> * [modules/juce_graphics/fonts/juce_GlyphArrangement.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_GlyphArrangement.cpp)
> * [modules/juce_graphics/fonts/juce_GlyphArrangement.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_GlyphArrangement.h)
> * [modules/juce_graphics/fonts/juce_TextLayout.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_TextLayout.cpp)
> * [modules/juce_graphics/fonts/juce_TextLayout.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_TextLayout.h)
> * [modules/juce_graphics/fonts/juce_Typeface.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_Typeface.cpp)
> * [modules/juce_graphics/fonts/juce_Typeface.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/fonts/juce_Typeface.h)
> * [modules/juce_graphics/geometry/juce_EdgeTable.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/geometry/juce_EdgeTable.cpp)
> * [modules/juce_graphics/geometry/juce_EdgeTable.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/geometry/juce_EdgeTable.h)
> * [modules/juce_graphics/juce_graphics_Harfbuzz.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/juce_graphics_Harfbuzz.cpp)
> * [modules/juce_graphics/native/juce_DirectWriteTypeface_windows.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_DirectWriteTypeface_windows.cpp)
> * [modules/juce_graphics/native/juce_Fonts_android.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_Fonts_android.cpp)
> * [modules/juce_graphics/native/juce_Fonts_freetype.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_Fonts_freetype.cpp)
> * [modules/juce_graphics/native/juce_Fonts_linux.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_Fonts_linux.cpp)
> * [modules/juce_graphics/native/juce_Fonts_mac.mm](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_Fonts_mac.mm)
> * [modules/juce_graphics/native/juce_RenderingHelpers.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_RenderingHelpers.h)
> * [modules/juce_gui_basics/drawables/juce_SVGParser.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/drawables/juce_SVGParser.cpp)
> * [modules/juce_gui_basics/keyboard/juce_KeyPress.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/keyboard/juce_KeyPress.cpp)
> * [modules/juce_gui_basics/keyboard/juce_KeyPress.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/keyboard/juce_KeyPress.h)
> * [modules/juce_gui_basics/keyboard/juce_ModifierKeys.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/keyboard/juce_ModifierKeys.cpp)
> * [modules/juce_gui_basics/keyboard/juce_ModifierKeys.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_gui_basics/keyboard/juce_ModifierKeys.h)
> * [modules/juce_javascript/javascript/juce_Javascript_test.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_javascript/javascript/juce_Javascript_test.cpp)

This page documents the JUCE graphics rendering system, which is responsible for converting high-level drawing commands into pixel data for display or image output. It covers the architecture, rendering pipeline, core primitives, and the main code entities involved in 2D graphics rendering.

For information about the component system that uses this rendering pipeline, see page [3.1]. For details on specific UI widgets, see page [3.2]. For hardware-accelerated OpenGL rendering, see page [3.4].

## Rendering Pipeline Architecture

JUCE's graphics rendering is organized as a layered system, separating high-level drawing APIs from low-level, platform-specific implementations. The main entry point is the `juce::Graphics` class, which delegates drawing operations to a `juce::LowLevelGraphicsContext` interface. Concrete implementations of this interface handle the actual pixel manipulation, either in software or using platform APIs.

**Diagram: JUCE Graphics Rendering Class Relationships**

```

```

* `juce::Graphics`: High-level API for drawing.
* `juce::LowLevelGraphicsContext`: Abstract interface for rendering.
* `juce::LowLevelGraphicsSoftwareRenderer`: Software fallback renderer.
* `Direct2DGraphicsContext` and `CoreGraphicsContext`: Platform-specific hardware-accelerated renderers.

Sources:

* [modules/juce_graphics/contexts/juce_GraphicsContext.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_GraphicsContext.h)

## The Drawing Pipeline

The JUCE drawing pipeline translates high-level drawing commands into pixel data through a series of well-defined steps. The process is as follows:

**Diagram: Drawing Command Flow in Code Entities**

```mermaid
sequenceDiagram
  participant Application Code
  participant juce::Graphics
  participant juce::LowLevelGraphicsContext
  participant juce::EdgeTable
  participant Renderer

  Application Code->>juce::Graphics: g.fillPath(myPath)
  juce::Graphics->>juce::Graphics: saveStateIfPending()
  juce::Graphics->>juce::LowLevelGraphicsContext: fillPath(myPath, transform)
  juce::LowLevelGraphicsContext->>juce::EdgeTable: Create EdgeTable from path
  juce::EdgeTable->>juce::EdgeTable: Rasterize path into scan lines
  juce::LowLevelGraphicsContext->>Renderer: Render EdgeTable with current fill
  Renderer-->>juce::LowLevelGraphicsContext: Rendering complete
  juce::LowLevelGraphicsContext-->>juce::Graphics: Operation complete
  juce::Graphics-->>Application Code: Return control
```

**Pipeline Steps:**

1. Application code creates a `juce::Graphics` object for the target (component or image).
2. Drawing commands are issued to the `Graphics` object.
3. `Graphics` forwards commands to the selected `LowLevelGraphicsContext`.
4. For vector shapes, an `EdgeTable` is constructed to represent the shape as scan-line segments.
5. The `EdgeTable` is rasterized using the current fill (solid color, gradient, or image).
6. Platform-specific renderers may delegate to native APIs (e.g., Direct2D, CoreGraphics).

Sources:

* [modules/juce_graphics/contexts/juce_GraphicsContext.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_GraphicsContext.cpp)
* [modules/juce_graphics/geometry/juce_EdgeTable.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/geometry/juce_EdgeTable.cpp)

## Core Rendering Primitives

### EdgeTable

The `juce::EdgeTable` class is central to JUCE's 2D rasterization. It represents a shape as a set of horizontal scan-line segments, enabling efficient filling and anti-aliasing.

**Diagram: EdgeTable Construction and Rasterization**

```

```

Steps:

1. The path is flattened into line segments (`PathFlatteningIterator`).
2. Each segment is added as an edge in the `EdgeTable`.
3. Edges are sorted and converted to horizontal spans per scan line.
4. Spans are filled using the current fill type (color, gradient, image).

Sources:

* [modules/juce_graphics/geometry/juce_EdgeTable.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/geometry/juce_EdgeTable.h)
* [modules/juce_graphics/geometry/juce_EdgeTable.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/geometry/juce_EdgeTable.cpp)

### RenderingHelpers

The `juce::RenderingHelpers` namespace provides utility classes and templates for efficient pixel operations, fill types, and transformations.

**Diagram: RenderingHelpers Key Classes and Relationships**

```

```

Key helpers:

* `TranslationOrTransform`: Optimizes affine transformations, using simple translation when possible.
* `FloatRectangleRasterisingInfo`: Handles anti-aliasing for non-integer-aligned rectangles.
* `GradientPixelIterators`: Efficiently computes gradient colors for linear, radial, and transformed gradients.
* `EdgeTableFillers`: Specializations for filling with solid color, gradient, or image.

Sources:

* [modules/juce_graphics/native/juce_RenderingHelpers.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_RenderingHelpers.h)

## Platform-Specific Renderers

### Software Renderer

The `juce::LowLevelGraphicsSoftwareRenderer` is a CPU-based renderer that works on all platforms. It:

* Implements all drawing operations in software.
* Uses optimized pixel blending and rasterization.
* Renders directly to image pixel buffers.
* Serves as a fallback when hardware acceleration is unavailable.

Sources:

* [modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.h)
* [modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/contexts/juce_LowLevelGraphicsSoftwareRenderer.cpp)

### Direct2D (Windows)

On Windows, JUCE can use Direct2D for hardware-accelerated rendering. The main code entities are:

**Diagram: Direct2D Renderer Code Entities**

```

```

* `Direct2DHwndContext`: Renders to windows, manages swap chains and DPI.
* `Direct2DImageContext`: Renders to images using Direct2D bitmaps.
* `Direct2DDeviceResources`: Caches resources for performance.

Sources:

* [modules/juce_graphics/native/juce_Direct2DGraphicsContext_windows.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_Direct2DGraphicsContext_windows.cpp)

### CoreGraphics (macOS/iOS)

On Apple platforms, JUCE uses CoreGraphics (Quartz 2D) for rendering.

**Diagram: CoreGraphics Renderer Code Entities**

```

```

* `CoreGraphicsContext`: Wraps a `CGContextRef` for drawing, manages coordinate conversion and text rendering.
* `CoreGraphicsPixelData`: Manages image pixel data and conversion to/from `CGImage`.

Sources:

* [modules/juce_graphics/native/juce_CoreGraphicsContext_mac.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_CoreGraphicsContext_mac.h)

## Image Rendering

JUCE supports multiple image formats and provides efficient rendering for each. The `juce::Image` class abstracts pixel data and provides access to platform-specific storage.

**Diagram: Image Rendering Code Entities**

```

```

### Pixel Formats

JUCE supports:

* `RGB`: 24-bit RGB
* `ARGB`: 32-bit premultiplied ARGB
* `SingleChannel`: 8-bit alpha/grayscale

### Image Rendering Pipeline

* **Drawing to an Image**: Create a `juce::Image`, then a `juce::Graphics` for that image, and issue drawing commands.
* **Drawing an Image**: Use `Graphics::drawImageAt()`, `drawImage()`, or `drawImageTransformed()`. The renderer converts the image as needed for the platform.
* **Platform-Specific Optimizations**: * `Direct2DPixelData` uses Direct2D bitmaps. * `CoreGraphicsPixelData` uses `CGImage` or `CGBitmapContext`.

Sources:

* [modules/juce_graphics/images/juce_Image.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/images/juce_Image.h)

## Performance Considerations

### Resource Caching

JUCE renderers use caching to improve performance and avoid redundant work.

**Diagram: Resource Caching Flow**

```

```

Caching mechanisms:

* Gradient caches (linear/radial) for brushes.
* Glyph caches for text rendering.
* Image caches for repeated image drawing.
* Path caches for complex paths.

Sources:

* [modules/juce_graphics/native/juce_RenderingHelpers.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_RenderingHelpers.h)

### DPI Awareness and Scaling

JUCE's rendering system supports high-DPI displays and scaling.

**Diagram: Coordinate Scaling in Rendering**

```

```

* On Windows: Per-monitor DPI awareness, device contexts with scaling.
* On macOS/iOS: Native backing scale factor for retina displays.

Sources:

* [modules/juce_graphics/native/juce_RenderingHelpers.h](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/native/juce_RenderingHelpers.h)

## Special Effects

JUCE provides built-in support for effects such as drop shadows and glows.

* `juce::DropShadow`: Applies a blurred shadow to images, paths, or rectangles.
* `juce::GlowEffect`: Similar to drop shadows, but radiates in all directions.

Both effects work by creating a blurred mask from the source shape and compositing it with the desired color and offset.

Sources:

* [modules/juce_graphics/effects/juce_DropShadowEffect.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/effects/juce_DropShadowEffect.cpp)
* [modules/juce_graphics/effects/juce_GlowEffect.cpp](https://github.com/juce-framework/JUCE/blob/d6181bde/modules/juce_graphics/effects/juce_GlowEffect.cpp)

## Conclusion

JUCE's graphics rendering system provides a flexible and powerful abstraction over platform-specific rendering technologies, enabling consistent high-quality drawing across platforms while still leveraging hardware acceleration when available. The layered architecture separates concerns and allows for platform-specific optimizations without affecting the higher-level API.

Understanding this rendering pipeline is essential for creating efficient graphics in JUCE applications, especially when dealing with complex UI or performance-critical rendering.