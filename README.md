# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Enjoy the rich sound of Surge XT easily with MML. Written in Rust.

### Usage

- For playing around and making sounds with MML
- For casual installation. Only Rust is required.

### Technical Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Preparation

Install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Install

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Execution

```
cmrt
```

You can input MML and play around in the TUI screen.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C, D, E, F, G, A, B.

### Configuration

`config.toml` is automatically created on first launch. It's located in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. Restart the application after closing the editor.

Current configuration example:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing to /render of a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# Real-time playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 62154
realtime_play_server_command = ""

# Whether to autoplay on startup
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for Surge XT patches
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]

# List of directories to search for WAV loops in the loop browser
loop_dirs = []

# List of categories that can be assigned to loop directories.
# The key for the category overlay is determined from unused English letters within the category name.
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in_process renders. |
| `offline_render_backend` | `in_process` | Target for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server instances. |
| `offline_render_server_port` | `62153` | Localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for play_server. |
| `realtime_play_server_command` | Empty string | Command to launch play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for when selecting patches. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from unused English letters within the category name. |

OS-specific default `plugin_path` values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default `patches_dirs` values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

If `offline_render_backend = "render_server"`, the TUI side does not directly load the CLAP plugin but instead sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, will restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
  - If a Bluesky post contains MML, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C-D-E (Do-Re-Mi).

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh.
- It also supports various chord progression notations (some are not yet supported).

# Breaking Changes
- Expect frequent breaking changes daily.

# Future Plans
- It's logical to obtain Surge XT patches via an API, so that will be implemented (currently, they are searched from TOML specified paths, which is inefficient. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measures
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and quickly iterating through editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.

# Out of Scope
- Effects are considered out of scope and highly deferred, as they require dedicated editing. One reason for this is that Surge XT's patches already encapsulate effects (effects are derived from patches).