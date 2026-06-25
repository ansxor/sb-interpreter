# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

### Fixed
- MAINCNT (frame counter) not updating in sb-player wasm build (#94)
- XSCREEN 4 (combined) BG only renders on top screen, not both (#93)
- MAINCNT not advancing in sb-player: wasm host loop now fires the VBlank heartbeat every frame (tick_frame before run_frame), and VSYNC/WAIT arm a pending target driven one frame at a time instead of jumping instantly (#94)
- sb-player VSYNC runs at display refresh rate, not 60 fps: pace the rAF loop to wall-clock 60 Hz so high-refresh displays (120/144 Hz) don't run programs too fast (#92)
- XSCREEN 2 not dual-screen: wasm compose() + player render only one screen (#81)
- DISPLAY/GPAGE: VM keeps one global graphics state — screens aren't separate contexts (#80)
- Per-screen BG layers (BG* target active DISPLAY; XSCREEN bg-alloc split) (#84)
- Per-screen sprite tables (SPSET/SPCLR/... target active DISPLAY; XSCREEN sprite-alloc split) (#83)
- GRP per-screen draw context + DISPLAY routing (showPage default [0,1]) (#82)

### Changed
- S-T4f VSYNC/WAIT timing semantics (#46)
- sb-player: make errors visually obvious instead of silently doing nothing (#88)
- RPG GAME console textbox render bug: red box unclosed, game overlaps textbox (#87)
- Per-screen CONSOLE data (top/bottom share console state, should be independent) (#86)
- Compositor/runner/player render correct per-screen content for both screens (#85)
