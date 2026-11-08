# clap-mml-render-tui

## Overview
An MML TUI DAW (or similar). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) using MML. Written in Rust.

## Usage

- For playing around with sounds using MML (Music Macro Language)
- For personal use. An experimental project.

## Technology Stack
- Plugin Host Library
  - https://github.com/prokopyl/clack

## Download

[Download Windows Version](https://github.com/cat2151/clap-mml-render-tui/releases/download/v0.1.0/clap-mml-render-tui-0.1.0-windows-x64.zip)

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

※ Under consideration: Automation of the above (already implemented in `cat-plugin-player`)

## Running

```
cmrt
```

You can input MML in the TUI screen and play around.

## Patch Catalog

You can create a patch catalog for Dexed and other plugins, and use it for filtering, etc.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- ※ Limited to CLAP plugins available for free on Windows, without account registration.
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
- SFZ (those that allow trying key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Download Pages for Each (for those who got lost)
  - Surge XT: Easy to get with winget: [Fastest! How to Install a DAW and Audio Plugins (even for cats) (up to playing sounds with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download page](https://floe.audio/download/)
  - [Sforzando Download page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)
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
- Frequent breaking changes are made daily. It's common for something that worked yesterday not to work today.

## Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in one-measure units,"
    - while being constrained,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - ※ While "atomic measure" might sound like a term from physics, I've chosen to use this specific terminology for now, to avoid confusion.
- Experimentation
    - Various experiments will be implemented.
- Personal Use Only
    - Frequent breaking changes will be made.
        - It will be common for files saved yesterday not to load today.
            - Therefore, please limit its use to toy-level projects.

## Out of Scope
- Practicality

## License
- MIT