---
title: VISIBLE
slug: docs-ptc-visible
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-visible
content_id: 19563
created: 2023-03-20
scraped: 2026-06-21
---

# VISIBLE

Control what graphics are displayed or hidden.

## Syntax

```sbsyntax
VISIBLE console,panel,bg0,bg1,sprites,graphics
```

| Input | Description |
| --- | --- |
| console | Controls visibility of the upper screen text foreground. Can be omitted. |
| panel | Controls most but not all of the lower screen. See notes. Can be omitted. |
| bg0 | Controls visibility of the foreground BG layer of both screens. Can be omitted. |
| bg1 | Controls visibility of the background BG layer of both screens. Can be omitted. |
| sprites | Controls visibility of sprites on both screens. Does not apply to the keyboard keys or system icons. See notes for panel interactions. Can be omitted. |
| graphics | Controls visibility of the GRP pages on both screens. This argument must always be provided. See notes for panel interactions. |

`VISIBLE` controls the visibility of the various graphical systems of Petit Computer. Each of these can be toggled separately, but the bottom screen usually requires the panel to be enabled as well for other systems to be visible.

Passing `TRUE` or `1` for the argument enables display of that component - `FALSE` or `0` disable it. Every argument except the last one can be omitted - not passing any value keeps the previous state.

## Examples

```sb
' Make everything visible
VISIBLE 1,1,1,1,1,1
```

```sb
' Disable graphics page and keep everything else's visibility the same
VISIBLE ,,,,,FALSE
```

## Notes

VISIBLE controls what graphical elements are visible. These are grouped into six categories, which can be enabled and disabled separately. Some things are impossible to disable and other things depend on multiple toggles.

All arguments are rounded down to the nearest integer.

### console

This controls whether the text console is visible or not. This only controls the visibility of the upper screen text console, and only the text layer itself - if the background has been set by COLOR, it will still be displayed.

### panel

This controls almost everything on the lower screen, including the panel background, keyboard sprites, system icons, and (partially) the function keys. If `PNLTYPE "PNL"` or `PNLTYPE "OFF"` were used, and then a command like `INPUT` is called, the keyboard will be invisible, but the function key text will be displayed. If previously the panel had already been set to a keyboard such as `PNLTYPE "KYA"`, however, the function key text will be hidden.

Note that this does not seem to affect text printed with `PNLSTR`. There seems to be no way to control this directly with `VISIBLE` - one workaround is to set `PNLTYPE "KYA"` to hide the text layer instead, but this has the side effect of an invisible keyboard on the lower screen.

The keyboard is fully functional when hidden - it is simply not displayed.

Even if `PNLTYPE "OFF"` was set, the sprites and graphics layers will not be visible if the panel setting is `FALSE`.

### sprites

This controls the visibility of sprites. This affects both screens, but does not apply to keyboard sprites or system icons. Additionally, for sprites on the lower screen to display, the panel setting must also be enabled.

### graphics

This controls the visibility of the GRP pages. This affects both screens, but like the sprites requires the panel setting to be enabled for the bottom screen's graphic page to be visible.

## Errors

| Action | Error |
| --- | --- |
| An argument isn't 0 or 1 after rounding (or empty) | Out of range |
| A string argument is provided | Type Mismatch |
| Last argument is skipped | Syntax error |
| More than six arguments are provided | Syntax error |
| Less than six arguments (including empty ones) are provided | Missing operand |
