---
title: SPLINK
slug: docs-sb4-splink
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-splink
content_id: 19540
created: 2020-12-14
scraped: 2026-06-21
---

# SPLINK

Link a sprite to a parent, set link flags, and check link relationships.

A sprite can be *linked* to any sprite with an ID less than its own. This sprite is now the "child" of the other sprite (which makes the other sprite the parent.) A child can only belong to one parent, but a parent can have multiple children.

The coordinate system of the children (including the Z coordinate) is now relative to the parent's (specifically the parent sprite's `SPHOME`.) For example, given the below code, sprite 1 is actually at `20,20` on-screen, because sprite 0 is at `10,10`.

```sb4
'create sprites
SPSET 0,0
SPSET 1,1
'make 1 a child of 0
SPLINK 1,0
'move the sprites
SPOFS 0,10,10
SPOFS 1,10,10
```

The child coordinate system is also affected by the parent's `SPSCALE` and `SPROT`. For example, `SPSCALE 0,2,2` causes the child to be twice as far away from the parent, and `SPSCALE 0,45` causes the child to rotate 45 degrees around the parent.

By default, these coordinate system transformations do not affect the actual display properties of the children, just their position. By setting link flags, children can inherit their show/hide state, rotation, scale, color, and layer from their parent.

## Link

Link a child to a parent, optionally setting link flags.

```sbsyntax
SPLINK childID%, parentID% {, linkFlags% }
```

| Input | Description |
| --- | --- |
| `childID%` | ID of the child sprite. 0-4095. |
| `parentID%` | ID of the parent sprite. 0-4095.<br>`parentID%` must be less than `childID%`. |
| `linkFlags%` | Optional bitset specifying properties to inherit from the parent.<br>Bit — Description<br>1 — `SPSHOW`/`SPHIDE`/`SPCLR`<br>2 — `SPROT`<br>4 — `SPSCALE`<br>8 — `SPCOLOR`<br>16 — `SPLAYER`<br>If the `SPSHOW` flag is set, then the child sprite will be cleared the frame after the parent is cleared with `SPCLR`. |

## Check

Check the link relationships of a sprite, or link flags.

```sbsyntax
SPLINK spriteID%, linkType% OUT linkValue%
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. |
| `linkType%` | Type of link information to get. Optional, default 0.<br>Type — Description<br>0 — ID of parent sprite.<br>1 — ID of first child sprite.<br>2 — ID of next sibling sprite.<br>3 — Link flags of this sprite. |

| Output | Description |
| --- | --- |
| `linkValue%` | Link information. If parent, child, or sibling were checked, the appropriate sprite ID (or -1 if there is no link relationship) is returned. If link flags were checked, the link flags of the target sprite (or 0 if unset) are returned. |

## Examples

```sb4
'link sprite 1 to sprite 0
SPLINK 1,0
```

```sb4
'get the parent of sprite 1, and the first child of sprite 0
PRINT SPLINK(1)
PRINT SPLINK(0,0)
```
