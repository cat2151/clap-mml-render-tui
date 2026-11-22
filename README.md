# clap-mml-render-tui

## Overview
An MML TUI DAW (or similar) application. Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [Zebralette3](https://u-he.com/products/zebralette3/) [JC-303](https://midilab.co/jc303/) / [OB-Xf](https://surge-synth-team.org/ob-xf/) / [Apricot](https://nakst.itch.io/apricot) [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) using MML. Written in Rust.

## Download

[Download Windows version](https://github.com/cat2151/clap-mml-render-tui/releases/download/v0.1.0/clap-mml-render-tui-0.1.0-windows-x64.zip)

## Usage

- For playing around with sounds using MML (Music Macro Language)
- For personal use. An experimental project.

## Technical Stack
- Plugin host library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/)

```
winget install "Surge XT"
```

※Under consideration: Automation of the above (already implemented in cat-plugin-player)

## Run

```
cmrt
```

You can input MML in the TUI screen and play around with it.

## Patch Catalog

Creates a patch catalog for Dexed and other plugins. This allows you to easily select patches from supported audio plugins.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- ※Limited to CLAP, Windows, and freely available plugins without account registration.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- [Zebralette3](https://u-he.com/products/zebralette3/)
- [JC-303](https://midilab.co/jc303/)
- [OB-Xf](https://surge-synth-team.org/ob-xf/)
- [Apricot](https://nakst.itch.io/apricot)
- Effects (can be inserted in series into tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- sfz (those with a dedicated screen to try key switches and control changes)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- sfz (Drum kits)
  - I believe all sfz drum kits are automatically supported.
- Individual Download Pages (for reference)
  - Surge XT : Easy to get with winget : [Fastest! DAW and Audio Plugins that even a cat can install (up to playing sounds with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6](https://u-he.com/products/tyrelln6/)
  - [Zebralette3](https://u-he.com/products/zebralette3/)
  - [JC-303](https://midilab.co/jc303/)
  - [OB-Xf](https://surge-synth-team.org/ob-xf/)
  - [Apricot](https://nakst.itch.io/apricot)
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
- Breaking changes are made frequently on a daily basis. It is common for something that worked yesterday to not work today.

## Concept Notes
- Atomic Measures
    - Inspired by Obsidian's Atomic Notes.
    - By making the unit of all processing "offline rendering in one-measure increments",
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching purposes and for quickly iterating through editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
- Experiments
    - Various experiments will be implemented.
- Modular
    - Various functionalities are implemented in separate library crates.
        - This is intended to facilitate use from other projects.
            - However, the benefits feel thin compared to the increased test build time. Well, you could say it's valuable if it yielded experimental results that visualize this.
- For Personal Use
    - Breaking changes are made frequently.
        - It is common for files saved yesterday to not be loadable today.
            - Therefore, please limit its use to toy-level projects.

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