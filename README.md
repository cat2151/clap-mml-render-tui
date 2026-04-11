# clap-mml-render-tui

### Purpose

- For playing with sounds using MML.
- For casual installation. Only Rust is required.

### Technology Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

Install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Running

```
cmrt
```

You can input MML and play with it on the TUI screen.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - When MML is found in a Bluesky post, it can be played with Surge XT.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It's logical to obtain Surge XT patches via an API, and this will be implemented (currently, it inefficiently searches for patches specified in a TOML file. Implementation is deferred, prioritizing other tasks).

# Concept Notes
- アトミック小節 (Atomic Measure)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in single-measure increments,"
    - in exchange for accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - (Note: If translated as 'atomic measure', it might be confused with a term from physics, so for now, it is kept as 'アトミック小節' without English translation.)

# Out of Scope
- Effects are deliberately deemed out of scope and a low priority, despite being essential for editing. One reason is that Surge XT patches inherently include effects (effects are essentially extracted from patches).