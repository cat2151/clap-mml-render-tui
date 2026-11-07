# clap-mml-render-tui

## Overview
An MML TUI DAW (or similar). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) with MML. Written in Rust.

## Purpose

- For playing around and making sounds with MML
- For personal use. An experimental project.

## Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

※Under consideration: Automation of the above (already implemented in `cat-plugin-player`).

## Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

※Under consideration: Automatic release.

## Running

```
cmrt
```

You can input MML and play around in the TUI screen.

## Patch Catalog

You can create a patch catalog for Dexed and other plugins, and use it for filtering, etc.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- ※Limited to those available for free on Windows, without account registration, and supporting CLAP.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effects (can be inserted in series into a track in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (included with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- SFZ (those with a dedicated screen for trying out key switches and control changes)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Individual download pages (for those who get lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install DAW and audio plugins even for cats (up to making sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download Page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub Releases Page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

## Update

```
cmrt update
```

## Play SNS Posts as Music

```
cmrt --server
```

For details, please refer to [bluesky-text-to-audio](https://github.com/cat2151/bluesky-text-to-audio/blob/main/README.ja.md).

## Breaking Changes
- Breaking changes are made frequently, on a daily basis. It's common for something that worked yesterday to not work today.

## Concept Notes
- アトミック小節
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing an "offline rendering of one bar,"
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and quickly iterating through editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - ※If translated as 'atomic measure', it might be confused with a term from physics, so for now, it is kept as 'アトミック小節' without a full English translation.
- Experimentation
    - Various experiments will be implemented.
- Personal Use Only
    - Breaking changes are frequent.
        - It's common for files saved yesterday to not load today.
            - Therefore, please use it at a 'toy' level.

## Out of Scope
- Practicality

## License
- MIT