# clap-mml-render-tui

### Purpose

- For experimenting with sounds using MML.
- For casual installation. Having Rust installed is sufficient.

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Run

```
cmrt
```

You can input MML and play it in the TUI screen.

### Configuration

`config.toml` is automatically created on first launch. It's located in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode for TUI / DAW, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here's an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering workers for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing to /render in a render-server child process.
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
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | List of editor candidates to try, in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in-process rendering workers. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` workers. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to start `render_server`. |
| `realtime_audio_backend` | `in_process` | Target for real-time audio playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to start `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for patches. |

The default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share` is used)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

If `offline_render_backend` is set to `"render_server"`, the TUI side does not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives a WAV. If the connection to the render-server fails, `cmrt` will start a child process and, in case of a communication error, retry after restarting once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
- When there is MML in a Bluesky post, it enables playing it with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C-D-E.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is more logical to obtain Surge XT patches via an API, so that will be implemented (currently, they are inefficiently explored from paths specified in `toml`. Implementation will be deferred to prioritize other features).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes concept.
    - By making the unit of all processing 'offline rendering in single-measure units',
    - while accepting certain constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - ※ The term 'atomic measure' is used here; while it might evoke a physics term, it refers to a single musical measure as the smallest processing unit.

# Out of Scope
- Effects require editing, so they are explicitly deemed out of scope and postponed to much later. One reason for this is that in Surge XT, patches inherently include effects (effects are derived from patches).