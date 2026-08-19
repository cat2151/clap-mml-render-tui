# clap-mml-render-tui

### Overview
A TUI DAW (of sorts) for MML. Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Usage
- For playing around with sounds using MML
- For casual installation. Just having Rust is enough.

### Tech Stack
- Plugin host library
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

### Run

```
cmrt
```

You can play by entering MML in the TUI screen.

### Keyboard Screen
Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play the notes C, D, E, F, G, A, B.

### Configuration
`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is a current example configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under the
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ directories
# in the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders in DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the cmrt main process.
# render_server: Renders by POSTing /render to a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# Realtime playback backend
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

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | List of editor candidates, tried from left to right. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` renders. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` instances. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback execution. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to launch `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for sound patches. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from unused English letters in the category name. |

The OS-specific default `plugin_path` values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The OS-specific default `patches_dirs` values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Switching between multiple plugins
You can switch with a single `active_plugin` line. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line:

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths set to the OS-specific standard installation locations:

| Name | `plugin_id` | `patches_dirs` | Category for intended use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Use top-level settings directly (= Surge's category names) |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |

Names are matched case-insensitively, ignoring spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

You only need to write `[plugins.<name>]` if you have plugins installed in non-standard locations or if you are using a plugin that is not built-in. **Only the specified items will override the built-in values**, so if you just want to change the path, a single `plugin_path` line is sufficient.

```toml
active_plugin = 'Surge XT'

# Only replace the path. plugin_id and patches_dirs remain as built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For non-built-in plugins, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Specify a built-in name or a name from `[plugins.*]`. If not specified, the top-level `plugin_path` / `patches_dirs` will be used directly. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch location for that plugin. To clear built-in values, set `patches_dirs = []`. |
| `plugins.<name>.<purpose>_patch_categories` / `<role>_patch_keywords` | Filters for automatic patch selection by purpose. The same 7 key names as top-level (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`) can be used directly. Only the specified items will apply when that plugin is active. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error will occur, profiles take precedence). For purpose-specific categories, items specified in the profile (including the 7 items for built-in Dexed) will take precedence.
- If the `active_plugin` name is neither built-in nor present in `[plugins.*]`, the application will fail to launch with an error, displaying all available names.
- Dexed's sounds are "one `.syx` cartridge = 32 programs", so in the list, each program is displayed as `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits), treating the cartridge as a directory. If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch setting, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer.
- The category settings for filtering candidates by line purpose (chord / bass / arpeggio / 4 drum roles / others) default to Surge's category names. Since Dexed cartridges do not have "directory name = purpose", the built-in Dexed profile has all these filters empty (meaning all programs are candidates for any line). If you want to filter by the directory name of the cartridge location, specify categories in `[plugins.Dexed]`.
- Mono/poly shared determination data (`voicing_shared_source` / `voicing_override_source`) used for purpose-specific automatic selection is for Surge XT only. It will not be retrieved when using plugins other than Surge XT.
- Rendering result caches are placed in separate directories for each plugin, so switching plugins will not play sounds from the previous plugin (no manual deletion is needed). The locations are as follows, where `<plugin>` is the filename of `plugin_path` (without extension) (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\<plugin>\*.wav` (DAW track WAVs)

If `offline_render_backend = "render_server"`, the TUI side does not load the CLAP plugin directly. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt will launch a child process and retry once if a communication error occurs.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - When an MML post is found on Bluesky, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing "cde" will play C-D-E.

```
cmrt CM7
```

- Typing "CM7" will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### `patch-roles` Command

```
cmrt patch-roles
```

- Displays how many sound patch candidates are available for selection with the wheel in the PATCH column for each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). This command does not launch the TUI screen.
- Use this command after changing plugins, `patches_dirs`, or purpose-specific categories (e.g., `chord_patch_categories`) to check if the "wheel is unresponsive".
- If any line has 0 candidates, it will list that line and exit with status code 1.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes more sense to fetch Surge XT patches via an API (currently, they are inefficiently explored from toml specifications. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measure (アトミック小節)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching purposes and rapidly iterating through editing cycles.
    - For more serious editing, existing high-feature DAWs would be more appropriate.
    - *Note: "Atomic measure" sounds like a term from physics, so for now, the Japanese "アトミック小節" is kept without direct translation.*

# Out of Scope
- Effects require editing, so they are intentionally kept out of scope and deferred significantly. One reason for this is that in Surge XT, patches already encapsulate effects (effects are derived from patches).