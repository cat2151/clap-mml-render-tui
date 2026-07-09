# clap-mml-render-tui

### Purpose

- For playing around with MML (Music Macro Language) to make sounds.
- For casual installation. Rust is all you need.

### Technology Stack
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

### Execution

```
cmrt
```

You can enter MML in the TUI screen and play with it.

### Configuration

A `config.toml` file is automatically created on the first launch. It is located in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In the TUI / DAW NORMAL mode, pressing `e` will open `config.toml` in your editor. After closing the editor, restart the application.

Here is an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Candidate editors to open config.toml (tried in order from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the cmrt main process.
# render_server: Renders by POSTing to /render on a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# Realtime playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 62154
realtime_play_server_command = ""

# Whether to autoplay on startup
# Notepad mode: Immediately plays the current line. DAW mode: Starts playback from the beginning of the song (measure 0).
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
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Candidate editors (tried in order from left to right). |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for `in_process` backend. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent executions for `render_server`. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to start `render_server`. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback execution. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to start `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for when selecting patches. |

The `plugin_path` default values by OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The `patches_dirs` default values by OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

If `offline_render_backend = "render_server"`, the TUI side does not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives a WAV file. If the connection to the render-server fails, cmrt launches a child process and, in case of a communication error, restarts and retries once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension.
  - When MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C-D-E.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Obtaining Surge XT patches via API is the proper way, so that will be implemented (currently searching specified directories in TOML is inefficient. Implementation timing is deferred, prioritizing other tasks).

# Concept Notes
- アトミック小節
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per 1 measure,"
    - while imposing constraints,
    - various benefits can be gained.
    - This approach is suitable for sketching and rapidly iterating editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Note: "atomic measure" sounds like a physics term, so for now, the term "アトミック小節" is kept as is without direct English translation.*

# Out of Scope
- Effects require extensive editing, so they are explicitly out of scope and postponed significantly. One reason for this is that in Surge XT, patches already encompass effects (effects are derived from patches).