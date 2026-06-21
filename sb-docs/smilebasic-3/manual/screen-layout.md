---
title: Screen Layout
slug: docs-sb3-manual-screen-layout
system: SmileBASIC 3
type: guide
topic: 33
source: e-manual.pdf
scraped: 2026-06-21
---

# Screen Layout

In Petit Computer, multiple screens with different roles are displayed superimposed over each other, such as the console screen, which displays characters (text), and the graphic screen, which displays graphics.

## Upper Screen (3D Screen)

This upper screen consists of five screens. In order from innermost to foremost, these are: the background screen, graphic screen, BG screen, sprite screen, and console screen.

- **Background Color** — A single-color screen that is displayed behind all the other screens.

- **Graphic Screen** — The screen that displays graphics drawn with graphic instructions.

- **BG Screen** — The screen used to create game maps and so forth by filling different areas with different tiles.
  - Up to 128x127 tiles of 16x16 dots each can be arranged and displayed on the screen
  - There are four layers, allowing for multi-scrolling and other effects

- **Sprite** — Element used for foreground characters, such as the hero of a game, that are displayed in front of the BG screen.
  - Up to 512 sprites can be used in total for the upper and touch screens
  - The basic size is 16x16 dots, which can be changed individually to any required size
  - An instruction to display multiple sprites continuously, producing animation, is available.

- **Console Screen** — Text screen where characters can be written using instructions such as `PRINT`.

The 3D screen has depth, the amount of which is expressed by the Z-coordinate. The reference surface is Z=0, with Z going negative in front of the screen. The range of depth is 1024 to -256. However, the screen has fewer visible graduations than this, and the results will also change according to adjustment of the 3D depth slider.

## Touch Screen

The structure is the same as the upper screen, except the screen size is different. The keyboard is only displayed on the touch screen.

As with the upper screen, the order of superimposed display elements is managed with the Z-coordinate.

### Positional Relationship between the Upper and Touch Screens

The upper and touch screens are arranged vertically so that the horizontal center positions of the two screens are aligned. The coordinate origin (0, 0) for both screens is at the top left.

### Continuous Display of the Upper and Touch Screens

You can use the upper and touch screens as one continuous screen by combining them using the `XSCREEN 4` instruction.

## Graphic Page

SmileBASIC provides a total of six locations for storing source images to be displayed on the screen. These are called graphic pages.

Each page has a name from GRP0 to GRP5 and corresponds to a certain display screen. For example, the graphic page called "GRP0" corresponds to the graphic screen on the upper screen.

If you use a graphic instruction to draw a graphic or place graphic data in GRP0, it will be displayed on the upper screen's graphic screen.

The following indicates which graphic page corresponds to which screen.

**Upper Screen (DISPLAY 0)**
- Graphic Screen — GRP0
- BG Screen — GRP5
- Sprite — GRP4

**Touch Screen (DISPLAY 1)**
- Graphic Drawing — GRP1
- BG Screen — GRP5
- Sprite — GRP4

## Display Page and Drawing Page

Drawing complicated graphics may take a long time, and the graphic may appear on-screen in an unfinished state. In order to prevent this, you can use a second graphic page to display while drawing is being processed on the first page.

In the initial state, the same graphic page is used for the drawing page and the display page: GRP0 (or GRP1 for the touch screen). However, it is possible to specify a different page using the `GPAGE` instruction.

## Color Specification

SmileBASIC allows you to use 65536 colors for the whole screen. The way colors are specified differs between the graphic and the console screens.

| Display Element | # of Colors / Color Specifications |
|---|---|
| Graphic Page | 32768 colors per pixel; Use the `RGB` function to specify colors |
| Console Screen (Text) | Select from 16 colors (including transparent) |

### Specifying colors on a graphic page

Colors are represented internally as 5 bits for each RGB color + 1 bit for transparency (RGBA=5551). However, when specifying colors, the `RGB` function should be used, and an 8-bit value for each RGB component specified.

**Examples of drawing color specification using the RGB function**

```
GCOLOR RGB( R, G, B )
```

- Specifies color tone values in the range 0-255 for R (Red), G (Green) and B (Blue)

```
GCOLOR RGB( A, R, G, B )
```

- For A (transparency), the value 255 specifies "opaque," and any other value specifies "transparent"

### Specifying colors on the console screen (text)

You can set the character and background color for each character. Select a color for each one, out of 16 colors. The color and number mapping is displayed on the upper screen of the SMILE tool.
