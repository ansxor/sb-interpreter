---
title: Layer Guide
slug: docs-sb4-layer-guide
system: SmileBASIC 4
type: guide
source: https://smilebasicsource.com/forum/thread/docs-sb4-layer-guide
content_id: 19548
created: 2020-05-15
scraped: 2026-06-21
---

# Layer Guide

As part of its display system, SB4 contains eight *layers*, numbered 0 to 7. A layer is a grouping of sprites and text screens. They can simply be used to organize different display objects for your convenience, or their filtering and transformation functions can be used to create complex graphical effects.

## Default Setup

By default, all sprites and text screens are assigned to layer 0. All layers have no filters, transforms, or composition modes applied.

## Assigning Layers

To assign a sprite or text screen to a layer, use the `SPLAYER` or `TLAYER` functions:

```sb4
SPLAYER 0,1  'assign sprite 0 to layer 1
TLAYER 0,1   'assign text screen 0 to layer 1
```

For sprites, you can also change the default layer they appear in when they are set up with `SPSET`:

```sb4
SPLAYER 1  'all new sprites will be assigned to layer 1
```

## Draw Order

Layers are drawn in reverse order, back to front; e.g. 7, then 6, then 5 etc. up to 0. This makes 0 the "top" layer and 7 the "bottom" layer. The contents of each layer are drawn all at once, meaning that a sprite with a higher z-offset in layer 1 will always draw under a sprite in layer 0. This can be used in addition to z-offset to ensure some elements never overlap or draw out of order.

## Layer Composition

As part of the rendering step, each layer is drawn on-screen using a specified *composition mode*. By default, the composition mode is *None*, meaning the layer is simply rendered on top of everything else. There are five composition modes:

| Number | Name | Description |
| --- | --- | --- |
| 0 | None | Overwrite; layer affects display contents below it |
| 1 | Simple | Composite this layer as normal |
| 2 | Add | Composite this layer using additive blending |
| 3 | Multiply | Composite this layer using multiply blending |
| 4 | Screen | Composite this layer using screen blending |

To change the composition mode, use the `LAYER` function:

```
LAYER 0,1  'set layer 0 to Simple composition
```

### Composition Color

`LAYER` additionally allows you to use a *composition color*. The contents of this layer will be color multiplied by this color, in the same method as `SPCOLOR` and `TCOLOR`, before rendering.

```sb4
LAYER 0,0,#C_RED  'use red as the composition color
```

### None vs Simple

At first, None and Simple might look exactly the same. The difference is that when using None, the layer is rendered to the screen *before* `LFILTER` is applied, so everything below the layer is also affected by the filter. Additionally, the composition color has no effect in None mode.

## Layer Filters

Graphical filters can be applied to layers using `LFILTER`. Only one filter mode can be used at a time per layer. There are six layer modes:

| Number | Name | Description |
| --- | --- | --- |
| 0 | None | Disables filters on this layer |
| 1 | Mosaic | Applies a "pixelated" effect to the layer |
| 2 | Blur | Blurs the layer contents |
| 3 | Horizontal Raster Deformation | Apply offset and scaling transformations to each row of pixels |
| 4 | Vertical Raster Deformation | Apply offset and scaling transformations to each column of pixels |
| 5 | Color | Adjust the hue, saturation, and value of this layer |

The usage of each filter varies by type; for more detail, see the `LFILTER` page.

### Filter Stacking

When a layer's composition mode is set to None (the default setting), all display contents below this layer are affected by the filter setting. This allows you to effectively apply multiple filters to one layer, but requires using an empty layer per additional filter. For example, if you wanted to apply both Blur and Color filters to the entire screen, all of the display contents could be placed onto layer 1. Then, you apply the Color filter to layer 1, and the Blur filter to layer 0. You must ensure the additional layers have composition disabled for this to work!

```sb4
'test message
COLOR #C_RED
PRINT "BLURRY AND YELLOW"
'disable composition on layer 0
LAYER 0,0
'move console to layer 1
TLAYER #TCONSOLE,1
'apply color filter to layer 1 to turn the text yellow
LFILTER 1,5,60,1,1
'apply blur filter to layer 0
LFILTER 0,2,50
```

## Coordinate System

Each layer has its own coordinate system. Sprites and text screens are actually positioned within the layer's coordinate system, not the display's. By default, layer coordinates and display coordinates are the same, but its origin point and offset relative to the screen can be changed, among other things, using `LMATRIX`.

TODO:
- LFILTER
- LCLIP
