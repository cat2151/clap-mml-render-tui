# clap-mml-render-tui

### Overview
An MML TUI DAW (or something similar). Enjoy the rich sound of Surge XT easily with MML. Written in Rust.

### Usage

- For playing and experimenting with MML sounds
- For casual installation. Just having Rust is enough.

### Tech Stack
- Plugin host library
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

- `c d e f g a b` keys: Play the notes C-D-E-F-G-A-B.

### Configuration

On first launch, `config.toml` is automatically created. Its location is under the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of TUI / DAW, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example of the current configuration.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the configuration directory.
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

# List of directories to search for WAV loops in the loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates tried from left to right. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for `in_process`. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` instances. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to launch `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for sound presets. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused letters within the category names. |

The `plugin_path` default values for each OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The `patches_dirs` default values for each OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, then `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using Multiple Plugins

You can switch with a single `active_plugin` line. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line.

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, and paths are the standard installation locations for each OS.

| Name | `plugin_id` | `patches_dirs` | Categories by use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Uses top-level settings (i.e., Surge's category names) as is |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |

Names are matched case-insensitively and ignoring differences in spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you have installed the plugin in a non-standard location or if you are using a plugin that is not built-in. **Only the written items will override the built-in values**, so if you only want to change the path, a single `plugin_path` line is sufficient.

```toml
active_plugin = 'Surge XT'

# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, write all entries.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Write a built-in name or a `[plugins.*]` name. If omitted, the top-level `plugin_path` / `patches_dirs` will be used as is. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The preset location for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<use>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use. The same seven key names as the top-level (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`) can be written as is. Only the written items will take effect for that plugin. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error will occur, and the profile will take precedence). For categories by use, items specified in the profile (including the 7 items for built-in Dexed) will also take precedence.
- If the `active_plugin` name is not found in either built-in profiles or `[plugins.*]`, the application will fail to start with an error. All available names from both sources will be displayed.
- Dexed presets are "1 `.syx` cartridge = 32 programs", so in the list, cartridges are treated as directories, and each program is displayed individually, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, 0-indexed). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a preset property, and its default value is POLY. Therefore, all Dexed presets are treated as polyphonic for chord rows in the grid sequencer.
- The category settings that filter candidates by row use (chord / bass / arpeggio / drum) default to Surge's category names. Since Dexed cartridges do not follow the "directory name = use" convention, the built-in Dexed profile has all these filters empty (meaning all programs are candidates for any row). If you wish to filter by cartridge directory name, please specify categories under `[plugins.Dexed]`.
- The shared mono/poly judgment data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by purpose is exclusive to Surge XT. It is not retrieved when using plugins other than Surge XT.
- Rendering result caches are stored in separate directories for each plugin, so switching plugins will not play the sound of the previous plugin (no manual deletion is required). The two locations are as follows, where `<plugin>` is the file name of `plugin_path` (without extension) (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCAL_APPDATA%\clap-mml-render-tui\daw\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI will not directly load the CLAP plugin but will send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAV data. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

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
- It's more logical to obtain Surge XT patches via API, so that will be implemented (currently, they are inefficiently searched from `toml` specified paths. Implementation timing is deferred; other priorities come first).

# Concept Memo
- Atomic Measure
    - Inspired by Obsidian's Atomic Notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Note: "Atomic measure" sounds like a physics term, so for now, the Japanese term "アトミック小節" will be kept as is without direct English translation.

# Out of Scope
- Effects are essential for editing, so they are intentionally placed out of scope and will be prioritized much later. One reason for this is that in Surge XT, patches already encapsulate effects (effects are derived from patches).