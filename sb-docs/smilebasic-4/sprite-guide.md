---
title: Sprite Guide
slug: docs-sb4-sprite-guide
system: SmileBASIC 4
type: guide
source: https://smilebasicsource.com/forum/thread/docs-sb4-sprite-guide
content_id: 19542
created: 2021-12-07
scraped: 2026-06-21
---

# Sprite Guide

The *sprite system* is, without a doubt, the most powerful graphics feature in SmileBASIC, and likely the most important feature in general for creating games. Initially, using sprites is simple, but it can become deeply complex the further you go. This guide aims to introduce you to the core concepts of sprites and direct you to related reference pages.

## What are Sprites?

Think of a classic Mario game, and all of the different /things/ in it. Not the ground or the background objects, but the enemies, items, Mario himself. These are all *sprites*.

A sprite is a display element that shows an individual graphic on-screen. You can move these little pictures around, change their appearance, animate them, and do all sorts of other things.

![https://smilebasicsource.com/api/file/raw/23995](https://smilebasicsource.com/api/file/raw/23995)

Count the sprites.

In these classic 2D games, the active objects are sprites and the backgrounds are character layers (the SmileBASIC equivalent of which is text screens.) Sprites, in this sense, aren't just pictures. They're real, actual /things/ with a presence. In SmileBASIC, it's the same way. Sprites are created and assigned a /management ID/, after which they are present on-screen and can be freely controlled.

## Anatomy of a Sprite

To "create" a sprite, you have to initialize a sprite ID with its properties first. Any of these properties can be changed at any time, but the sprite has to be initialized to begin with. First, let's take a look at what a sprite is made of.

![https://smilebasicsource.com/api/file/raw/23998](https://smilebasicsource.com/api/file/raw/23998)
