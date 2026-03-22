# clap-mml-render-tui

### Usage

- For playing around with MML to generate sounds.
- For casual installation. Just having Rust is enough.

### Technical Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Install

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui --package clap-mml-render-tui
```

### Execution

```
cmrt
```

You can input MML in the TUI screen and play around.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Disable automatic updates via TOML. In that case, after quitting, display the command with a message like "An update is available. The update command is...". Further automation has been verified to be more complex with more drawbacks than benefits, thus it's out of scope.

# Out of Scope
- Effects are likely essential to edit, so for now, they are out of scope and will be deferred to a much later stage.