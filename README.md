# clap-mml-render-tui

### Usage

- For playing sounds using MML.
- For casual installation. Requires only Rust.

### Technology Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Prerequisites

Install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui clap-mml-render-tui
```

### Run

```
cmrt
```

You can enter MML in the TUI screen and play.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - If an MML is present in a Bluesky post, it can be played using Surge XT.

# Breaking Changes
- Frequent breaking changes will occur daily.

# Future Plans
- Disable automatic updates via `toml`. When automatic updates are off, after quitting, the message "Update available. Update command: ~" will be displayed. Further automation has been determined to be too complex with more disadvantages than advantages, and is thus out of scope.
- Fetching Surge XT patches via API is the correct approach and will be implemented (currently, it searches for paths specified in `toml`, which is inefficient. Implementation is currently deprioritized in favor of other features).

# Out of Scope
- Effects are likely to require mandatory editing, so they are currently considered out of scope and heavily deprioritized.