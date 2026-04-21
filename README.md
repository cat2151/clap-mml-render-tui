# clap-mml-render-tui

### Purpose

- For playing around with sound using MML
- For casual installation. Just having Rust is enough.

### Technology Stack
- Plugin Host Library
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

You can enter MML in the TUI screen and play with it.

### Configuration

On first launch, `config.toml` is automatically created. It is located under the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

Here is an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under the configuration directory's
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering workers for DAW (1-16)
offline_render_workers = 4

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing to /render on the render-server child process.
offline_render_backend = "in_process"
offline_render_server_port = 62153
offline_render_server_command = ""

# List of directories to search for Surge XT patches
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

The configuration items are as follows:

| Item | Default | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. If empty, an error will occur on startup. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. The actual save location is `phrase/` or `daw/` under the configuration directory. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. The actual save location is `phrase/` or `daw/` under the configuration directory. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `4` | Number of concurrent offline rendering workers for the DAW. Set within the range of `1` to `16`. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. Specify `in_process` or `render_server`. |
| `offline_render_server_port` | `62153` | Port number on `127.0.0.1` to use when `offline_render_backend = "render_server"`. Set within the range of `1` to `65535`. |
| `offline_render_server_command` | Empty string | Child process startup command for the `render_server` backend. If empty, it looks for `clap-mml-render-server` / `clap-mml-render-server.exe` in the same directory as `cmrt`, or a command with the same name on `PATH`. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for TUI / DAW patch selection and random patches. If unset or empty, patch selection and random patches cannot be used. |

OS-specific `plugin_path` default values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific `patches_dirs` default values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

When `offline_render_backend = "render_server"` is set, the TUI does not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, `cmrt` launches a child process and retries once upon a communication error.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interacts with the bluesky-text-to-audio Chrome extension
  - When an MML is found in a Bluesky post, it can be played using Surge XT.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is logical to obtain Surge XT patches via an API, so this will be implemented (currently, they are searched from TOML-specified paths, which is inefficient. Implementation timing is deferred; other priorities are higher).

# Concept Notes
- Atomic Measures
  - Inspired by Obsidian's Atomic Notes.
  - By making the unit of all processing "one-bar offline rendering",
  - while accepting constraints,
  - various benefits can be gained.
  - This approach is suitable for sketching and rapid editing cycles.
  - For more serious editing, existing feature-rich DAWs would be more appropriate.

# Out of Scope
- Effects are essential for editing, so they are intentionally deemed out of scope and postponed to much later. One reason for this is that Surge XT patches already encapsulate effects (effects are derived from patches).