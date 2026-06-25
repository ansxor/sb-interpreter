# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

### Fixed
- XSCREEN 2 not dual-screen: wasm compose() + player render only one screen (#81)
- DISPLAY/GPAGE: VM keeps one global graphics state — screens aren't separate contexts (#80)
- Per-screen BG layers (BG* target active DISPLAY; XSCREEN bg-alloc split) (#84)
- Per-screen sprite tables (SPSET/SPCLR/... target active DISPLAY; XSCREEN sprite-alloc split) (#83)
- GRP per-screen draw context + DISPLAY routing (showPage default [0,1]) (#82)

### Changed
- Compositor/runner/player render correct per-screen content for both screens (#85)
