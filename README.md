# clap-mml-render-tui

## Overview
An MML TUI DAW (or similar). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [Zebralette3](https://u-he.com/products/zebralette3/) [JC-303](https://midilab.co/jc303/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) with MML. Written in Rust.

## Download

[Download Windows version](https://github.com/cat2151/clap-mml-render-tui/releases/download/v0.1.0/clap-mml-render-tui-0.1.0-windows-x64.zip)

## Purpose

- For playing around with sounds using MML (Music Macro Language)
- For personal use. Experimental project.

## Technical Stack
- Plugin host library
  - https://github.com/prokopyl/clack

## Setup

Please install [Surge XT](https://surge-synthesizer.github.io/)

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

Creates a patch catalog for Dexed and other plugins. This makes it easier to select patches for supported audio plugins.

```
cmrt build-patch-catalog-cache
```

## Supported Audio Plugins
- ※ Limited to plugins that are CLAP, run on Windows, and are available for free without account registration.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- [Zebralette3](https://u-he.com/products/zebralette3/)
- [JC-303](https://midilab.co/jc303/)
- effect (can be inserted in series on tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (included with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- sfz (those with a dedicated screen to try key switches and control changes)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- sfz (Drum kit)
  - Any sfz drum kit should be automatically supported.
- Download pages for each (for those who got lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install DAW and audio plugins even for cats (up to playing sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6](https://u-he.com/products/tyrelln6/)
  - [Zebralette3](https://u-he.com/products/zebralette3/)
  - [JC-303](https://midilab.co/jc303/)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub Releases Page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

## Playing SNS posts as music

```
cmrt --server
```

For details, please refer to [bluesky-text-to-audio](https://github.com/cat2151/bluesky-text-to-audio/blob/main/README.ja.md)

## Breaking Changes
- Frequent breaking changes occur daily. It is common for something that worked yesterday not to work today.

## Concept Notes
- `アトミック小節` (Atomic Measure/Bar)
  - `アトミック小節` is a concept inspired by Obsidian's atomic notes.
  - By making the unit of all processing 'offline rendering in 1-measure units',
  - while imposing constraints,
  - various benefits can be gained.
  - This is suitable for sketching purposes and for quickly iterating on edits.
  - For more serious editing, existing feature-rich DAWs would be more appropriate.
  - *Note: The term `アトミック小節` is kept in Japanese as a direct translation to "atomic measure" might sound like a physics term.*
- Experimentation
  - Various experiments will be implemented.
- Modular
  - Various features are implemented as separate library crates.
  - This is intended to make it smoother for other projects to use them.
  - However, the benefits feel minimal compared to the increased test build time. Well, consider it a worthwhile experiment to visualize this outcome.
- Personal Use Only
  - Frequent breaking changes are made.
  - It is common for files saved yesterday not to load today.
  - Therefore, please limit its use to an experimental/toy level.

## Notes for Rust Users

If you install with cargo, you can easily get the latest version via self-update.

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
cmrt update
```

## Out of Scope
- Practicality

## License
- MIT