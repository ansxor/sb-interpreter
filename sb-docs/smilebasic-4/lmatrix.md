---
title: LMATRIX
slug: docs-sb4-lmatrix
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-lmatrix
content_id: 19552
created: 2020-11-02
scraped: 2026-06-21
---

# LMATRIX

Apply a transformation to a layer.

## Simple Transformation

The 2D affine transform according to the parameters is applied to the layer.

```sbsyntax
LMATRIX id%, originX%, originY% {, offsetX%, offsetY% {, scaleX#, scaleY# {, angle% }}}
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `originX%`, `originY%` | The coordinates of the layer's origin (aka its "home coordinates.")<br>The layer's transformation is centered on this point. |
| `offsetX%`, `offsetY%` | The amount of offset applied to the layer (or, the location of its origin on-screen.)<br>Optional. (0,0) if not specified. |
| `scaleX#`, `scaleY#` | The scale factor in the X and Y direction.<br>Optional. (1, 1) if not specified. |
| `angle%` | The angle to rotate the layer by, in degrees.<br>Optional. 0 if not specified. |

## Transformation Matrix

The transformation according to a transformation matrix is applied.

```sbsyntax
LMATRIX id%, matrix#[]
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `matrix#[]` | The transformation matrix to be used. |

`matrix#[]` is a 4x4 array based on the OpenGL transformation matrix, meaning any arbitrary transformation or projection that can be expressed in OpenGL can be applied.

## Clear Transformation

The layer's transformation is cleared.

```sbsyntax
LMATRIX id%
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
