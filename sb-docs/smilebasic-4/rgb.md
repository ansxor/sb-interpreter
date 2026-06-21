---
title: RGB
slug: docs-sb4-rgb
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-rgb
content_id: 19525
created: 2020-05-03
scraped: 2026-06-21
---

# RGB

Create, split, or modify color codes.

## Create Color Code

Create a color code value given its channel values.

```sbsyntax
RGB { alpha%, } red%, green%, blue% OUT color%
```

| Input | Description |
| --- | --- |
| `alpha%` | The alpha (transparency) channel (0-255). Optional, defaults to 255 |
| `red%` | The color channels (0-255). |
| `green%` | The color channels (0-255). |
| `blue%` | The color channels (0-255). |

| Output | Description |
| --- | --- |
| `color%` | Integer value encoding a 32-bit ARGB color. |

## Split Color Code

Split a color code into its channels.

```sbsyntax
RGB color% OUT { alpha%, } red%, green%, blue%
```

| Input | Description |
| --- | --- |
| `color%` | Integer value encoding a 32-bit ARGB color. |

| Output | Description |
| --- | --- |
| `alpha%` | The alpha (transparency) channel, 0-255 (optional.) |
| `red%` | The color channels (0-255). |
| `green%` | The color channels (0-255). |
| `blue%` | The color channels (0-255). |

## Replace Channels

Replace channels in color

```sbsyntax
RGB oldColor%, { alpha% }, { red% }, { green% }, { blue% } OUT newColor%
```

| Input | Description |
| --- | --- |
| `oldColor%` | The color code value to modify. |
| `alpha%` | Optional, will replace channels in `oldColor%` if specified, (0-255). |
| `red%` | Optional, will replace channels in `oldColor%` if specified, (0-255). |
| `green%` | Optional, will replace channels in `oldColor%` if specified, (0-255). |
| `blue%` | Optional, will replace channels in `oldColor%` if specified, (0-255). |

| Output | Description |
| --- | --- |
| `newColor%` | `oldColor%`, with channels replaced by `alpha%`, `red%`, `green%`, and `blue%` if specified. |

## Examples

```sb4
BACKCOLOR RGB(255,128,0) 'change background color to orange
```

```sb4
RGB #C_MAGENTA OUT R,G,B
?R,G,B '255, 0, 255
```

```sb4
C = RGB(0,224,255)
C2 = RGB(C,,128,,) 'replace red channel with 128
RGB C2 OUT R,G,B
?R,G,B '128,224,255
```
