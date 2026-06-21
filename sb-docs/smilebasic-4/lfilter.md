---
title: LFILTER
slug: docs-sb4-lfilter
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-lfilter
content_id: 19550
created: 2020-11-02
scraped: 2026-06-21
---

# LFILTER

Apply a filter to a layer.

## Syntax

```sbsyntax
LFILTER id% {, filterType%, filterParams... }
LFILTER id% OUT filterType%
```

| Argument | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `filterType%` | The type of filter to apply to the layer.<br>Value — Description<br>0 — No filter<br>1 — Mosaic<br>2 — Blur<br>3 — Horizontal Raster Deformation<br>4 — Vertical Raster Deformation<br>5 — HSV Color Shift<br>Optional. 0, if not specified. |
| `filterParams...` | Parameters to the specified filter.<br>The number and type of parameters depends on `filterType%`. See following sections for details on filters. |

If `filterType%` and `filterParams...` are not specified, the filter is cleared.

## Filters

The following sections specify how each layer filter is used.

### No Filter

The filter settings for the layer are reset.

```sbsyntax
LFILTER id%, 0
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `0` | Specify 0 for `filterType%` to clear the filter. |

### Mosaic

Apply a "pixelated" effect to the layer by downsampling the image.

```sbsyntax
LFILTER id%, 1, factor%
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `1` | Specify 1 for `filterType%` to use the mosaic filter. |
| `factor%` | The amount of downsampling to apply.<br>1 = no change. |

### Blur

Blur the layer.

```sbsyntax
LFILTER id%, 2, amount%
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `2` | Specify 2 for `filterType%` to use the blur filter. |
| `amount%` | The amount of blur to apply.<br>0 = no change. |

### Horizontal/Vertical Raster Deformation

Apply offset and scaling to each row or column of pixels.

```sbsyntax
LFILTER id%, 3, deformArray#[], repeatFlag%
LFILTER id%, 4, deformArray#[], repeatFlag%
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `3`, `4` | Specify 3 for `filterType%` to use the horizontal deformation filter.<br>Specify 4 for `filterType%` to use the vertical deformation filter. |
| `deformArray#[]` | A 2D real number array containing deformation information. |
| `repeatFlag%` | When true, the layer's contents will be repeated if the deformation samples outside the layer. |

The first dimension of `deformArray#[]` must be the display's height (in pixels) for horizontal deformation and the display's width for vertical deformation. These correspond to each row or column of the display, respectively. The second dimension must be 2 or 4. If it is 2, then offset and scale are specified only for the X direction (horizontal deform) or Y direction (vertical deform) of each row/column. If it is 4, then offset and scale can be specified in both the X and Y directions.

To sum up, here is a table of what each index refers to in the four possible modes.

| Filter/Index | Horizontal 2 | Vertical 2 | Horizontal 4 | Vertical 4 |
| --- | --- | --- | --- | --- |
| 0 | Row X offset | Column Y offset | Row X offset | Column X offset |
| 1 | Row X scale | Column Y offset | Row X scale | Column X scale |
| 2 | /unused/ | /unused/ | Row Y offset | Column Y offset |
| 3 | /unused/ | /unused/ | Row Y scale | Column Y scale |

Offset values are specified as real numbers, where 1.0 is the width/height of the display, depending on axis.

### HSV Color Shift

Change the colors of the image by adjusting the HSV values.

```sbsyntax
LFILTER id%, 5, hue#, saturation#, value#
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `5` | Specify 2 for `filterType%` to use the blur filter. |
| `hue#` | Amount to adjust the hue by. |
| `saturation#` | Amount to adjust the saturation by. |
| `value#` | Amount to adjust the value (brightness) by. |

## Examples

Each example will demonstrate a different filter individually, using a given sample image. The sample image, and the code that generates it, is below.

```sb4
ACLS
FOR I%=0 TO 239
 GLINE 0,I%,399,I%,RGBF(0,I%/239,1)
NEXT I%
GTRI 150,170,150,70,250,120,#C_RED
GTRI 250,170,250,70,150,120,#C_BLUE,#G_ADD
TFILL 0,18,8,22,12,&hE801
SPSET 0,0:SPOFS 0,32,64:SPHOME 0,8,8
SPSET 1,1:SPOFS 1,64,64:SPHOME 1,8,8:SPSCALE 1,2:SPROT 1,45
SPSET 2,2:SPOFS 2,112,64:SPHOME 2,8,8:SPSCALE 2,3,3:SPROT 2,90
LOCATE 18,2
PRINT "Hello, ";
COLOR #C_BLACK
PRINT" World!"
```

!https://smilebasicsource.com/api/file/raw/3346#img

### Mosaic

```sb4
'apply a mosaic with 5x downsampling
LFILTER 0,1,5
```

!https://smilebasicsource.com/api/file/raw/3347#img

### Blur

```sb4
'apply a blur with a strength of 50
LFILTER 0,2,50
```

!https://smilebasicsource.com/api/file/raw/3348#img

### Horizontal Raster Deformation

```sb4
'horizontal sine distortion
'declare filter params array [height of screen,2]
DIM FP#[240,2]
'set params for each row of screen
FOR I%=0 TO 239
 'shift row horizontally by sine function
 FP#[I%,0]=SIN(I%/120*#PI)/8
 'keep scale the same
 FP#[I%,1]=1
NEXT I%
'apply filter, enable wrapping
LFILTER 0,3,FP#,#TRUE
```

!https://smilebasicsource.com/api/file/raw/3349#img

### Vertical Raster Deformation

```sb4
'vertical sine distortion
'declare filter params array [width of screen,2]
DIM FP#[400,2]
'set params for each column of screen
FOR I%=0 TO 399
 'shift row horizontally by sine function
 FP#[I%,0]=SIN(I%/200*#PI)/8
 'keep scale the same
 FP#[I%,1]=1
NEXT I%
'apply filter, enable wrapping
LFILTER 0,4,FP#,#TRUE
```

!https://smilebasicsource.com/api/file/raw/3350#img

### HSV Color Shift

```sb4
'shift hue by 180 degrees
'shift saturation by -50
'shift value by 86
LFILTER 0,5,180,-50,86
```

!https://smilebasicsource.com/api/file/raw/3351#img
