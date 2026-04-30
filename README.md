# clap-mml-render-tui

### Usage

- For playing around with MML (Music Macro Language) sounds.
- For casual installation. Only Rust is required.

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Prerequisites

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Install

```
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Run

```
cmrt
```

You can input MML in the TUI screen and play around.

### Configuration

`config.toml` is automatically created on first launch. It's located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here's an example of the current configuration.

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates for opening config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under the
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ subdirectories
# of the configuration directory. The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing to /render on a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# Real-time playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 62154
realtime_play_server_command = ""

# List of directories to search for Surge XT patches
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific default Surge XT CLAP path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for the `in_process` backend. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent workers for `render_server`. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to launch `play_server`. |
| `patches_dirs` | OS-specific default Surge XT patches directories | List of directories to search for patches when selecting sounds. |

The default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, defaults to `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

If `offline_render_backend = "render_server"`, the TUI does not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, `cmrt` launches a child process and, in case of a communication error, retries after restarting once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works with the bluesky-text-to-audio Chrome extension.
- When MML is found in a Bluesky post, it can be played with Surge XT.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It's more appropriate to obtain Surge XT patches via API, so that will be implemented (currently, searching specified toml directories is inefficient. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measure (アトミック小節)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in one-measure increments,"
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note: Since 'atomic measure' tends to refer to a term in physics, for now, I will keep it as 'アトミック小節' without direct English translation.

# Out of Scope
- Effects require editing, so they are intentionally designated as out of scope and pushed far back in priority. One reason for this is that in Surge XT, patches inherently include effects (effects are derived from patches).