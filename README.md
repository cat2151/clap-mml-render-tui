# clap-mml-render-tui

### Overview
MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Purpose

- For playing around with sounds using MML
- For casual installation. Having just Rust installed is sufficient.

### Technology Stack
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

You can input MML and play around in the TUI screen.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B (Do Re Mi Fa Sol La Si)

### Configuration

A `config.toml` file will be automatically created on first launch. It is located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW's NORMAL mode, pressing `e` opens `config.toml` in your editor. After closing the editor, restart the application.

Here is an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Candidate editors to open config.toml (tried in order from left)
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

# List of directories to search for WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to be used. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Candidate editors to try, in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate during rendering. |
| `buffer_size` | `512` | Buffer size during rendering. |
| `offline_render_workers` | `2` | Number of concurrent in_process renders. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server processes. |
| `offline_render_server_port` | `62153` | localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to start render_server. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | localhost port for play_server. |
| `realtime_play_server_command` | Empty string | Command to start play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for sound patches. |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from unused English letters within the category name. |

Default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

Default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, then `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using Multiple Plugins

You can switch with a single line for `active_plugin`. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line.

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths set to the standard installation location for each OS.

| Name | plugin_id | patches_dirs | Categories by Use Case |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Uses top-level settings (i.e., Surge's category names) as is |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |

Names are matched case-insensitively, ignoring spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

Only if you have installed a plugin in a non-standard location, or are using a plugin not built-in, should you write `[plugins.<name>]`. **Only the items you write will override the built-in values**, so if you only want to change the path, a single `plugin_path` line is sufficient.

```toml
active_plugin = 'Surge XT'

# Only replace the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Write a built-in name or a `[plugins.*]` name. If not specified, the top-level `plugin_path` / `patches_dirs` will be used as is. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch location for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<use_case>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use case. You can write 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will take effect for that plugin. If not specified, the plugin's default values will be used (Surge XT uses Surge's category names, others use "no filtering"). |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error, the profile takes precedence). For use-case specific categories, items specified in the profile (including the 7 items for built-in Dexed) will also take precedence.
- If the `active_plugin` name is neither built-in nor present in `[plugins.*]`, it will fail to start with an error. All available names will be displayed.
- Since Dexed's sounds are "1 cartridge `.syx` = 32 programs", the list displays each program individually, treating the cartridge as a directory, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2 digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default value is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord rows.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / 4 drum roles / others) **differ per plugin**. Surge XT uses Surge's category names, while Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for all rows). This is because Dexed cartridges do not use "directory name = use case," and for non-built-in plugins, the patch organization system is unknown. If you want to change this, write the 7 items in `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated config.toml).
- The 7 items can also be written at the top level, but they **only apply to the default plugin (the one that plays lines with no patch specified)**. This is a legacy writing style from before `active_plugin` existed; newly generated config.toml files will not write them at the top level. Existing configs with top-level settings will continue to function.
- The shared judgment data for mono/poly used for automatic selection by use case (`voicing_shared_source` / `voicing_override_source`) is for Surge XT only. It will not be retrieved when using plugins other than Surge XT.
- Rendering results are cached in separate directories per plugin, so switching plugins will not play sounds from the previous one (no need to manually delete them). The cache locations are as follows, where `<plugin>` is the filename of `plugin_path` (without extension) (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI will not directly load CLAP plugins. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once.

### update command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension
  - When MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play Do Re Mi.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### patch-roles command

```
cmrt patch-roles
```

- Displays the number of sound patch candidates available for selection with the wheel in the PATCH column for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen will be launched.
- Use this to check if the wheel has become unresponsive after changing plugins, `patches_dirs`, or use-case categories (like `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with code 1.

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is proper to acquire Surge XT patches via API, so this will be implemented (currently, searching specified items in toml is inefficient. Implementation timing is deferred, prioritizing other tasks).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's Atomic Notes.
    - By making the unit of all processing "offline rendering in 1-measure units",
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.

# Out of Scope
- Effects require essential editing, so they are intentionally placed out of scope and pushed far back in priority. One reason for this is that in Surge XT, patches include effects (effects are derived from patches).