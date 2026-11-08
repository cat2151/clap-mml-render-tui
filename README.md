# clap-mml-render-tui

## Overview
An MML TUI DAW (or something similar). Enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) easily with MML. Written in Rust.

## Download

[Download Windows version](https://github.com/cat2151/clap-mml-render-tui/releases/download/v0.1.0/clap-mml-render-tui-0.1.0-windows-x64.zip)

## Purpose

- For playing around with sounds using MML (Music Macro Language).
- Personal project. Experimental project.

## Technical Stack
- Plugin Host Library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

※ Under consideration: Automation of the above (already implemented in cat-plugin-player)

## Run

```
cmrt
```

You can input MML in the TUI screen and play around.

## Patch Catalog

You can create a patch catalog for Dexed and other plugins, and use it for filtering and more.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- ※ Limited to plugins available for free on Windows, supporting CLAP, and requiring no account registration.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effects (can be inserted in series on tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (included with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- SFZ (those that allow testing key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Download pages for each (for those who get lost)
  - Surge XT: Easier to get with winget: [Fastest! How to install DAW and audio plugins even for cats (up to playing sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
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

## Play SNS Posts as Music

```
cmrt --server
```

For details, please refer to [bluesky-text-to-audio](https://github.com/cat2151/bluesky-text-to-audio/blob/main/README.ja.md).

## Breaking Changes
- Frequent breaking changes occur daily. It's common for something that worked yesterday not to work today.

## Concept Notes
### Atomic Bar (アトミック小節)
Inspired by Obsidian's atomic notes.
By making the unit of all processing 'offline rendering in 1-measure units', while accepting constraints, various benefits can be gained. This is suitable for sketching and quickly iterating on edits. For more serious editing, existing feature-rich DAWs would be more suitable.
*Note: The term "atomic measure" was avoided due to potential confusion with physics terminology; the original Japanese term "アトミック小節" is used to convey this concept.*
### Experimentation
- Various experiments will be implemented.
### Personal Use
- Frequent breaking changes will occur.
    - It will be commonplace for files saved yesterday to be unloadable today.
        - Therefore, please limit its use to toy-level experimentation.

## Notes for Rust Users

Installing with cargo allows for easy updates to the latest version via self-update.

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
cmrt update
```

## Out of Scope
- Practicality

## License
- MIT