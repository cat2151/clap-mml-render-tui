# clap-mml-render-tui

## Overview
An MML TUI DAW (or similar). Easily enjoy rich sounds with MML from [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx). Written in Rust.

## Download

[Download Windows version](https://github.com/cat2151/clap-mml-render-tui/releases/download/v0.1.0/clap-mml-render-tui-0.1.0-windows-x64.zip)

## Purpose

- For playing around with MML (Music Macro Language) to generate sounds.
- Personal use. Experimental project.

## Technology Stack
- Plugin host library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

※Under consideration: Automation of the above (already implemented in `cat-plugin-player`)

## Run

```
cmrt
```

You can input MML in the TUI screen and play around with it.

## Patch Catalog

Creates a patch catalog for Dexed and other plugins. This makes it easier to select patches from supported audio plugins.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- ※Limited to those available for free on CLAP, Windows, without account registration.
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
- SFZ (specifically those with a dedicated screen for trying key switches and control changes)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Download pages for each (for those who get lost)
  - Surge XT: Easy to get with winget: [Fastest! How even a cat can install a DAW and audio plugins (up to playing sounds with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (Studiorack-site introduction page)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (Studiorack-site introduction page)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download page](https://floe.audio/download/)
  - [Sforzando Download page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

## Playing SNS Posts as Music

```
cmrt --server
```

For details, please refer to [bluesky-text-to-audio](https://github.com/cat2151/bluesky-text-to-audio/blob/main/README.ja.md).

## Breaking Changes
- Frequent breaking changes occur daily. It's common for something that worked yesterday not to work today.

## Concept Notes
- アトミック小節
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-bar increments",
    - various benefits can be gained despite the constraints.
    - This is suitable for sketching and rapid iteration of the editing cycle.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - ※Since 'atomic measure' could be mistaken for a physics term, 'アトミック小節' will be kept as is for now without direct English translation.
- Experimentation
    - Various experiments will be implemented.
- Personal Use
    - Frequent breaking changes will occur.
        - It will be common for files saved yesterday not to load today.
            - Therefore, please keep usage at a toy level.

## Note for Rust Users

If you install with cargo, you can easily get the latest version via self-update.

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
cmrt update
```

## Out of Scope
- Practicality

## License
- MIT