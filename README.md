# clap-mml-render-tui

### Usage

- For playing around with MML sounds
- For casual installation. Only Rust is required.

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

Install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui clap-mml-render-tui
```

### Running

```
cmrt
```

You can enter MML and enjoy playing in the TUI screen.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - This allows playing MML found in Bluesky posts with Surge XT.

# Breaking Changes
- Expect frequent, daily breaking changes.

# Future Plans
- Disable automatic updates via TOML. In this case, after quitting, the message "An update is available. The update command is ~" will be displayed along with the command. Further automation has been verified to be too complex with more disadvantages than advantages, and is therefore out of scope.

# Out of Scope
- Effects will likely require manual editing, so we'll accept that and keep them out of scope for now, deferring them to a much later stage.