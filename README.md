# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Enjoy the rich sounds of Surge XT easily with MML. Written in Rust.

### Purpose

- For playing around with sound using MML
- For casual installation. Just having Rust installed is enough.

### Tech Stack
- Plugin Host Library
  - https://github.com/prokopyl/clack

### Preparation

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

You can play by entering MML in the TUI screen.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play CDEFGAB notes.

### Configuration

Upon first launch, `config.toml` will be automatically created. It is located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is a current configuration example.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the config directory.
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

# Real-time playback backend
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

# List of directories to search for WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows.

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP default path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates to try in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` renders. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` processes. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to start `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to start `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | OS-specific Surge XT patches default directories | List of directories to search for sound presets. |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Keys for category overlay are determined from unused English letters within the category name. |

OS-specific default `plugin_path` values are as follows.

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default `patches_dirs` values are as follows.

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using Multiple Plugins

You can switch with a single line `active_plugin`. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line.

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths being the standard installation locations for each OS.

| Name | plugin_id | patches_dirs |
| --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above |
| `Dexed` | `com.digital-suburban.dexed` | None (due to unsupported patch selection) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

You only need to write `[plugins.<name>]` if you have plugins installed in non-standard locations or if you are using a plugin not listed as built-in. **Only the specified items will override the built-in values**, so if you just want to change the path, a single `plugin_path` line is sufficient.

```toml
active_plugin = 'Surge XT'

# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Specify either a built-in name or a `[plugins.*]` name. If not specified, the top-level `plugin_path` / `patches_dirs` will be used directly. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The location for that plugin's patches. To clear built-in values, set `patches_dirs = []`. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error will occur, and the profile will take precedence).
- If the `active_plugin` name is neither built-in nor present in `[plugins.*]`, the application will fail to launch with an error. All available names from both sources will be displayed.
- Dexed currently only supports **initial patches**. Patch (cartridge `.syx` and program) selection is not supported, so please do not specify `patches_dirs`.
- When switching plugins, manually clear the rendering result cache. For lines without patch specified (lines without `{"Surge XT patch": ...}` at the beginning of MML), the cache key remains the same before and after the switch, resulting in the sound of the previous plugin being played. The two locations to clear the cache are (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"` is set, the TUI will not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt will launch a child process and retry once upon communication error.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play Do-Re-Mi.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes sense to obtain Surge XT patches via an API, so that will be implemented (currently, they are searched for from `toml` specifications, which is inefficient. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per measure",
    - while accepting certain constraints,
    - various benefits can be gained.
    - This approach is suitable for sketching and rapidly iterating on edits.
    - For more extensive editing, existing feature-rich DAWs would be more appropriate.
    - *Note: "atomic measure" in English might sound like a term from physics, so for now, the Japanese term 「アトミック小節」 (Atomic Measure) is kept without direct translation.

# Out of Scope
- Effects are essential for editing, so they are intentionally set as out of scope and deferred to much later. One reason for this is that Surge XT's patches encapsulate effects (effects are derived from patches).