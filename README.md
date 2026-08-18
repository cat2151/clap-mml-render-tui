# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Rust is all you need.

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Prerequisites

Install [Surge XT](https://surge-synthesizer.github.io/)

```
winget install "Surge XT"
```

### Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Running

```
cmrt
```

You can input MML and play around in the TUI screen.

### Keyboard View

Press `v` to move to the keyboard view.

- `c d e f g a b` keys: Play C D E F G A B.

### Configuration

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of the TUI / DAW, press `e` to open `config.toml` with an editor. After closing the editor, restart the application.

Here is an example configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering tasks for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the cmrt main process.
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

# List of directories to search in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default | Description |
| --- | --- | --- |
| `plugin_path` | Default Surge XT CLAP path per OS | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates to try in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate during rendering. |
| `buffer_size` | `512` | Buffer size during rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` rendering tasks. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` tasks. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Startup command for `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Startup command for `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | Default Surge XT patches directories per OS | List of directories to search for patches (timbres). |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused English letters within the category names. |

The default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using Multiple Plugins

You can switch plugins with a single `active_plugin` line. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line.

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths set to the standard installation locations per OS:

| Name | plugin_id | patches_dirs |
| --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) |

Names are matched case-insensitively and ignoring spaces and underscores (e.g., `Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

Only write `[plugins.<name>]` if you are using a non-standard installation path or a plugin not included in the built-in profiles. **Only the explicitly written items will override the built-in values**, so if you only want to change the path, a single `plugin_path` line is sufficient.

```toml
active_plugin = 'Surge XT'

# Only swap the path. plugin_id and patches_dirs remain as built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not in built-in profiles, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Specify either a built-in name or a `[plugins.*]` name. If not specified, the top-level `plugin_path` / `patches_dirs` will be used. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch location for that plugin. To clear built-in values, write `patches_dirs = []`. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error will occur, profiles take precedence).
- If the `active_plugin` name is neither built-in nor found in `[plugins.*]`, it will fail to start with an error. All available names from both sources will be displayed.
- Dexed patches are organized as 'one `.syx` cartridge = 32 programs', so in the list, each program is displayed individually, treating the cartridge as a directory, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). By specifying the cartridge location in `patches_dirs`, you can select patches just like Surge's `.fxp` files.
- Dexed's mono/poly setting is not a patch characteristic but an instance setting (`MonoMode`), and its default is POLY. Therefore, all Dexed patches are treated as chord-friendly in the grid sequencer's chord rows. Category settings (`chord_patch_categories`, etc.) used to narrow down candidates by row purpose (chord / bass / drum) default to Surge's category names, so when using purpose-specific auto-selection with Dexed, please specify the directory name where the cartridges are located.
- The shared judgment data for mono/poly (`voicing_shared_source` / `voicing_override_source`) used for purpose-specific auto-selection is exclusive to Surge XT. It is not acquired when using plugins other than Surge XT.
- When switching plugins, manually clear the rendering cache. **Only lines without a specified patch** (lines without `{"Surge XT patch": ...}` at the beginning of the MML) will play sounds from the previous plugin because their cache keys remain the same before and after the switch. Lines with specified patches will include the patch name in their keys, so they will not be mixed. The two locations to clear are (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\*.wav` (DAW track WAVs)

When `offline_render_backend = "render_server"` is set, the TUI itself does not directly load CLAP plugins. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, `cmrt` will launch a child process, and in case of a communication error, it will restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works with the bluesky-text-to-audio Chrome extension.
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C D E.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh.
- Also supports various chord progression notations (some are not yet supported).

# Breaking Changes
Frequent breaking changes occur daily.

# Future Plans
- Obtaining Surge XT patches via API is the correct approach (currently, searching directories specified in `toml` is inefficient. This implementation will be prioritized later, after others).

# Concept Notes
- Atomic Measures
    - Inspired by Obsidian's Atomic Notes concept.
    - By making the unit of all processing 'offline rendering in one-measure increments',
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing high-functional DAWs would be more suitable.

# Out of Scope
- Effects require essential editing, so they are intentionally out of scope and pushed to a much later priority. One reason is that Surge XT patches often contain effects (effects are often derived from patches).