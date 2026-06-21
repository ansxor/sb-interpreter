---
title: Computer Colors (RGB)
slug: docs-sb3-manual-computer-colors-rgb
system: SmileBASIC 3
type: guide
topic: 28
source: e-manual.pdf
scraped: 2026-06-21
---

# Computer Colors (RGB)

The colors used in programs (RGB) are different from those used for printed materials such as books (CMY). The following is a brief explanation of the colors used in computers.

## Subtractive Primary Colors (for printing)

Just like paint colors, the colors used in printing become murky when blended. In the above figure, the point where all three colors meet is B (black). Blending these colors completely will produce jet black. Ink cartridges used in printers also use these colors. (However, since printers cannot produce completely jet black by simply blending the colors, black ink is often supplied separately.)

## Additive Primary Colors (for TVs)

Video game screens use liquid crystal displays, which create color by emitting different colored lights. Unlike the colors used for printing, blending the three additive primary colors (RED (red component), GREEN (green component) and BLUE (blue component)) will make the light stronger, producing a color that is closer to WHITE. These colors are called RGB, an acronym for Red, Green and Blue. In programming, RGB is used when specifying colors.

## RGB Values of Commonly Used Colors

For example, if you want to draw a red line on the screen, write the following:

```sb3
GLINE 0,0,399,239,RGB( 255,0,0 )
```

At the end of this line is the instruction `RGB()`, which takes the value of the red component R (0-255), the value of the green component G and the value of the blue component B, in this order. If you want to draw a green line, write `RGB(0,255,0)`. For a blue line, write `RGB(0,0,255)`.
