# clap-mml-render-tui

### Overview
A TUI DAW (or similar) for MML. Enjoy the rich sounds of Surge XT easily with MML. Written in Rust.

### Usage

- For playing around with MML and making sounds.
- For casual installation. Requires only Rust.

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

You can enter MML and play around in the TUI screen.

### Keyboard screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play the notes C D E F G A B.

### Configuration

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory:

- Windows: `%LOCAL_APPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the cmrt main process.
# render_server: Renders by POST /render to a render-server child process.
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

# List of directories to search for WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates to try in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` renders. |
| `offline_render_backend` | `in_process` | Target for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` instances. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to start `render_server`. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to start `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for sound patches. |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories that can be assigned to loop directories. Keys for category overlay are determined from unused English letters within category names. |

The OS-specific `plugin_path` default values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The OS-specific `patches_dirs` default values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Switching between multiple plugins

You can switch with a single line `active_plugin`. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line.

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths set to the OS-specific standard installation locations:

| Name | plugin_id | patches_dirs | Category by Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Uses top-level settings (i.e., Surge's category names) as is |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

You only need to write `[plugins.<name>]` if the plugin is installed in a non-standard location or if it's not built-in. **Only the specified items will override the built-in values**, so if you just want to change the path, one line for `plugin_path` is sufficient.

```toml
active_plugin = 'Surge XT'

# Only replace the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For non-built-in plugins, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Write a built-in name or a `[plugins.*]` name. If not specified, the top-level `plugin_path` / `patches_dirs` will be used as is. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The location of sound patches for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filters for automatic patch selection by usage. You can write the same 7 key names as the top level (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will apply when that plugin is active. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error, profile takes precedence). For usage-specific categories, items specified in the profile (including the 7 items for built-in Dexed) also take precedence.
- If the `active_plugin` name is neither built-in nor found in `[plugins.*]`, the application will fail to launch with an error. All available names will be displayed.
- Dexed sounds are "1 `.syx` cartridge = 32 programs". So, in the list, each program is displayed like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (number is 0-indexed, two digits), treating the cartridge as a directory. By specifying the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is not part of the sound patch but an instance setting (`MonoMode`), and its default value is POLY. Therefore, all Dexed sound patches are treated as suitable for chords in the grid sequencer's chord rows.
- The category settings that filter candidates by line usage (chord / bass / arpeggio / drum) default to Surge's category names. Since Dexed cartridges do not use "directory name = usage", the built-in Dexed profile has all these filters empty (= all programs are candidates for all rows). If you want to filter by the cartridge's directory name, specify categories in `[plugins.Dexed]`.
- Mono/poly sharing determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-specific automatic selection is exclusively for Surge XT. It is not retrieved when using other than Surge XT.
- Rendered cache results are placed in separate directories for each plugin, so switching plugins will not play sounds from the previous plugin (no manual deletion needed). The locations are:
  - `%LOCAL_APPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCAL_APPDATA%\clap-mml-render-tui\daw\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI does not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, restart and retry once.

### Update command

```
cmrt update
```

### Server mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI mode

```
cmrt cde
```

- Typing `cde` plays the notes C, D, E.

```
cmrt CM7
```

- Typing `CM7` plays a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### Patch-roles command

```
cmrt patch-roles
```

- Displays how many sound patch candidates are available for selection with the wheel in the PATCH column for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The TUI screen does not launch.
- Use this to check if the wheel becomes "unresponsive" after changing plugins, `patches_dirs`, or usage-specific categories (like `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with status code 1.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes sense to retrieve Surge XT patches via API, so this will be implemented (currently, patches specified in toml are explored, which is inefficient. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measure (アトミック小節)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per measure,"
    - While imposing constraints,
    - It offers various advantages.
    - This approach is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note: The author has chosen to use the Japanese term "アトミック小節" for this concept, as a direct translation "atomic measure" might be confused with terms from physics.*

# Out of Scope
- Effects are deemed out of scope and postponed significantly, as editing them is essential. One reason for this is that Surge XT patches often contain effects (effects are derived from patches).