---
title: BG (Backgrounds)
slug: docs-sb3-manual-bg-backgrounds
system: SmileBASIC 3
type: guide
topic: 34
source: e-manual.pdf
scraped: 2026-06-21
---

# BG (Backgrounds)

Petit Computer 3's SmileBASIC provides structures called BG and sprites that make it easy to create game screens with movement.

## About BG

BG is an abbreviation for background, and refers to a common structure used for creating background scenery in console games. Tiles of 16x16 dots in size are arranged together in order to create one larger image (the background). By arranging identical tiles together, you can easily create areas of uniform graphics, such as oceans or fields.

In this product, various BG tiles commonly used in games are predefined. These can be used immediately by simply placing them on the BG screen using the `BGPUT` instruction.

**BG Tiles**

The BG tiles are preloaded in the GRP5 graphic page. Each tile is 16x16 dots in size and has a tile number in the range 0-1023. You can check which number corresponds to which tile from the touch screen by using the SMILE tool.

**BG Screen/Layers**

BG tiles are displayed on-screen only after they are placed on the BG screen.

The same BG tile can be placed repeatedly on the screen. The BG screen has coordinates in tile units. One screen can contain 25 tiles horizontally and 15 tiles vertically. Please note that by using the `BGSCREEN` instruction, you can also use a BG screen that is larger than the display screen.

The BG screen consists of 4 superimposed layers, and BG tiles can be placed on each one. These layers can be used to, for example, distinguish between the background and the foreground, or to enable multi-scrolling. When placing BG tiles, a layer number (0-3) must be specified.

You can use the `BGOFS` instruction to change the display location of each layer in dot units. This makes scrolling possible. You can also change the order in which the layers are superimposed by specifying the Z-coordinate in the `BGOFS` instruction. This also affects depth in 3D display.

## Place a BG Tile - BGPUT Instruction

This instruction is used to place a BG tile on the specified BG screen layer.

**Format**

```sb3
BGPUT Layer, X, Y, Tile number
```

- Layer: Number of the layer on which to place the tile (0-3)
- X, Y: Coordinates at which to place the tile
- Tile number: Number of the tile to place (0-1023)

**Usage example**

```sb3
BGPUT 0,12,7,1
```

This places BG tile 1 near the center of the screen on layer 0.

## Fill an Area with Repeated BG Tiles - BGFILL Instruction

This instruction is used to fill a specified rectangular area with repeated instances of one BG tile.

**Format**

```sb3
BGFILL Layer, Start point X, Start point Y, End point X, End point Y, Tile number
```

- Layer: Number of the layer that includes the area to fill with the tile (0-3)
- Start point X, Y: Coordinates for the top left of the target area
- End point X, Y: Coordinates for the bottom right of the target area
- Tile number: Number of the tile to fill the area with (0-1023)

**Usage example**

```sb3
BGFILL 0, 1, 1, 23,13, 2
```

This will fill an area of one tile around the inside of the circumference of layer 0 with BG tile 2.

## Change the Display Location of the BG Screen - BGOFS Instruction

This instruction is used to change the display location and depth of a specified layer.

**Format**

```sb3
BGOFS Layer, X, Y, Z
```

- Layer: Number of the layer to move (0-3)
- X, Y: Amount (in dots) by which to move the display location. Positive values move the layer left or upwards
- Z: Coordinates in the depth direction (Rear: 1024 - Screen surface: 0 - Front: -256)

**Optional**

**Usage example**

```sb3
BGOFS 0, -3, -4
```

This moves the display location of layer 0 by 3 dots to the right and 4 dots downwards.

## Clear the BG Screen - BGCLR Instruction

This instruction is used to clear a specified BG screen layer.

**Format**

```sb3
BGCLR Layer
```

- Layer: Number of the layer to clear (0-3)

**Usage example**

```sb3
BGCLR 0
```

This clears the display of layer 0.

## Other BG Instructions

There are lots of other BG instructions. Please refer to HELP and the sample programs for information on how to use them. The following is a list of the main instructions.

- `BGSCREEN` instruction — Changes the maximum size of the BG screen
- `BGSCALE` instruction — Scales the BG screen up/down
- `BGROT` instruction — Rotates the BG screen
- `BGCOPY` instruction — Copies a specified range from the BG screen to another location
- `BGANIM` instruction — Performs animation using the BG
