# clap-mml-render-tui

## Overview
An MML TUI DAW (or similar). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [Zebralette3](https://u-he.com/products/zebralette3/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) using MML. Written in Rust.

## Download

[Download Windows version](https://github.com/cat2151/clap-mml-render-tui/releases/download/v0.1.0/clap-mml-render-tui-0.1.0-windows-x64.zip)

## Purpose

- For playing and experimenting with sounds using MML (Music Macro Language)
- For personal use. An experimental project.

## Technology Stack
- Plugin host library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

*Under consideration*: Automation of the above (already implemented in `cat-plugin-player`).

## Run

```
cmrt
```

You can input MML in the TUI screen and play around.

## Patch Catalog

Creates a patch catalog for Dexed and other plugins. This makes it easier to select patches for supported audio plugins.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- *Limited to*: CLAP, Windows, free, and available without account registration.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- [Zebralette3](https://u-he.com/products/zebralette3/)
- effect (can be inserted in series into a DAW mode track)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- sfz (those that allow trying key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- sfz (Drum kits)
  - Any sfz drum kit should be automatically supported.
- Download pages for each (for those who got lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install a DAW and audio plugins (up to playing sound with a virtual MIDI keyboard) even for cats](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe download page](https://floe.audio/download/)
  - [Sforzando download page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 download page](https://u-he.com/products/tyrelln6/)
  - [Zebralette3 download page](https://u-he.com/products/zebralette3/)
  - [TONE3000 download page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

## Playing SNS Posts as Music

```
cmrt --server
```

For more details, please refer to [bluesky-text-to-audio](https://github.com/cat2151/bluesky-text-to-audio/blob/main/README.ja.md).

## Breaking Changes
- Frequent breaking changes are made daily. It's common for something that worked yesterday to not work today.

## Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per measure,"
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note*: This concept treats a single musical measure as an atomic unit for processing.
- Experimentation
    - Implements various experiments.
- Modular
    - Various features are implemented as individual library crates.
        - This is intended to facilitate use from other projects.
            - However, the perceived benefits are slim compared to the increased test build time. Well, consider it a worthwhile experiment to visualize that outcome.
- Personal Project
    - Frequent breaking changes will be made.
        - It will be common for files saved yesterday to not be loadable today.
            - Therefore, please limit usage to toy-level experimentation.

## For Rust Users

Installing with cargo allows for easy updates to the latest version via self-update.

```
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
cmrt update
```

## Out of Scope
- Practicality

## License
- MIT