---
title: 3D Effects
slug: docs-sb3-manual-3d-effects
system: SmileBASIC 3
type: guide
topic: 32
source: e-manual.pdf
scraped: 2026-06-21
---

# 3D Effects

Petit Computer allows you to set depth values for graphics, characters, sprites, and so forth, making them appear to "pop out" (or "sink in") through the effect of parallax between the left and right eyes. While 3D effects can help heighten the expressiveness of your works, they can also cause eye strain if overused. Please follow the precautions given on this page and be careful to avoid using extreme parallax in your programs. If you ever find the parallax setting too intense or experience symptoms including eye strain when viewing someone else's work, please take a break to rest your eyes.

## WARNING - 3D FEATURE ONLY FOR CHILDREN 7 AND OVER

Viewing of 3D images by children 6 and under may cause vision damage. Use the Parental Control feature to restrict the display of 3D images for children 6 and under. See the Parental Controls section in the operations manual of your Nintendo 3DS system for more information.

## About 3D Effects

3D effects are only supported on the upper screen. Various different display objects can be given perspective by setting the depth coordinate (Z-coordinate) for them. (Use `LOCATE` for the console screen, `GPRIO` for the graphic screen, `SPOFS` for sprites, and `BGOFS` for BG to specify the Z-coordinate)

## Precautions Regarding 3D Effects

When implementing 3D effects in your work, please pay attention to the following points and adjust your work accordingly in order to prevent eye strain.

1. Do not use white too much for areas which will serve as the background
2. Do not construct depth with only two levels: the background and the foreground.

White is a color that makes it difficult to perceive depth. When only using a two-level depth setting (rear and front) it is difficult for the eyes to perceive depth because there are few objects to use for comparison.

3. Decrease the parallax of images that "pop out" from the screen

Since images that pop out from the screen surface (those with a Z setting value of less than 0) tend to cause eye strain, please adjust the setting so that they are within the screen. If you set 3D effects near the edge of the screen, the parallax may not appear on the screen, which makes it difficult for the eyes to perceive depth.

If you find that any work created by others uses very intensive 3D effects, please lower the 3D setting when using it. Furthermore, if you experience eye strain during the creation process, please take a break to rest your eyes.

Please also refer to "Health and Safety Information" from the HOME Menu.
