# clap-mml-render-tui

### Overview
An MML TUI DAW (or something similar). Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Usage

- For playing sounds with MML
- For casual installation. Rust is all you need.

### Tech Stack
- Plugin Host Library
  - https://github.com/prokopyl/clack

### Preparation

Please install [Surge XT](https://surge-synthesizer.github.io/).

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

You can play by entering MML in the TUI screen.

### Keyboard Screen

Press `v` to move to the keyboard screen.

- `c d e f g a b` keys: Play C, D, E, F, G, A, B.

### Settings

When launched for the first time, `config.toml` is automatically created. Its location is under the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW's NORMAL mode, pressing `e` opens `config.toml` with an editor. After closing the editor, restart the application.

Here is an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing /render to a render-server child process.
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

# List of categories that can be assigned to loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate during rendering. |
| `buffer_size` | `512` | Buffer size during rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for `in_process` backend. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` instances. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to launch `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for patches (timbres). |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from an unused English letter within the category name. |

Default `plugin_path` values by OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

Default `patches_dirs` values by OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using Multiple Plugins

You can define settings for each plugin under `[plugins.<name>]` and switch between them with a single `active_plugin` line.

```toml
active_plugin = 'dexed'

[plugins.surge_xt]
plugin_path  = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'
plugin_id    = 'org.surge-synth-team.surge-xt'
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]

[plugins.dexed]
plugin_path = 'C:\Program Files\Common Files\CLAP\Dexed.clap'
plugin_id   = 'com.digital-suburban.dexed'
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the `[plugins.*]` to use. If not specified, the top-level `plugin_path` / `patches_dirs` will be used. |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Directory for that plugin's patches (timbres). If not specified, the patch list will be empty. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error, the profile takes precedence).
- If the `[plugins.*]` pointed to by `active_plugin` does not exist, an error will occur, and defined profile names will be displayed.
- Dexed currently supports **initial patches only**. Selection of patches (cartridge `.syx` and program) is not yet supported, so please do not specify `patches_dirs`.
- When switching plugins, manually clear the rendering result cache. For lines where no patch is specified (lines without `{"Surge XT patch": ...}` at the beginning of the MML), the cache key remains the same before and after switching, so the sound from the previous plugin will be played. The two locations to clear are as follows (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\*.wav` (cache for notepad / MML input overlay)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\*.wav` (DAW track WAVs)

If `offline_render_backend = "render_server"` is set, the TUI will not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt will launch a child process and retry after restarting once in case of a communication error.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C, D, E.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It's more appropriate to retrieve Surge XT patches via an API, so that will be implemented (currently, they are searched from TOML specifications, which is inefficient. Implementation timing is postponed, prioritizing other features).

# Concept Memo
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units",
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing high-feature DAWs would be more suitable.
    - *Note: "Atomic measure" sounds like a physics term, so for now, I will keep it as "Atomic Measure" without further translation.

# Out of Scope
- Since effects require editing, we've decided to consider them out of scope and postpone them significantly. One reason for this is that in Surge XT, patches inherently include effects (effects are derived from patches).