# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando easily with MML. Written in Rust.

### Purpose

- For playing around with MML and making sounds
- For casual installation. Only Rust is required.

### Technology Stack
- Plugin Host Library
  - https://github.com/prokopyl/clack

### Preparation

Please install [Surge XT](https://surge-synthesizer.github.io/)

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

### Supported Audio Plugins
- *Limited to CLAP, Windows, and free plugins available without account registration.*
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C, D, E, F, G, A, B notes.

### Configuration

A `config.toml` file will be automatically created on first launch. Its location is under the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In the TUI / DAW's NORMAL mode, pressing `e` will open `config.toml` in an editor. After closing the editor, restart the application.

Here is an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ under the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering workers for DAW (1-16)
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
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for WAV loops in the loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only write if you want to change Surge XT's default values
[plugins."Surge XT"]
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path for Surge XT if installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Internal input MIDI filename. |
| `output_midi` | `output.mid` | Internal output MIDI filename. |
| `output_wav` | `output.wav` | Internal output WAV filename. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` rendering tasks. |
| `offline_render_backend` | `in_process` | Destination for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` tasks. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Command to launch `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for Surge XT patches (timbres). |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from an unused English letter within the category name. |

The default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin that plays lines without timbre specification is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog from `[plugins.<Name>]` and used for lines where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths set to OS-specific standard installation locations:

| Name | `plugin_id` | `patches_dirs` | Category for use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific defaults from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`.** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio` etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

You only need to write `[plugins.<Name>]` if you are installing in a non-standard location, providing a timbre location, or using a plugin not built-in. **Only the specified items will override the built-in values**, so if you only want to change Surge XT's path, one `plugin_path` line within that table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all relevant fields.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | The path to that plugin. |
| `plugins.<Name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | The timbre location for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Purpose>_patch_categories` / `<Role>_patch_keywords` | Filters for automatic patch selection by purpose. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If not specified, the default value for that plugin (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 purpose-specific category items at the top level will result in a configuration error rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default, while others become candidates in the mixed catalog.
- Dexed's timbres are "1 cartridge `.syx` file = 32 programs," so in the list, cartridges are treated as directories, and each program is listed like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, 2 digits). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance setting (`MonoMode`), not a timbre setting, and its default is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord lines.
- Vaporizer2's timbres are "1 `.vvp` file = 1 timbre," and can be selected just like Surge's `.fxp` files. The category shown in the list header is the **first 2 characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 is the only plugin that does not have default `patches_dirs`. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing to it arbitrarily could break your DAW environment. Please add a single line like this: `patches_dirs = ['D:\Vaporizer2\Presets']`. Until you do, it will not appear in the catalog as having 0 timbres.
- Vaporizer2's mono/poly setting differs per timbre, read from the content of the `.vvp` file (`m_uPolyMode`). Therefore, only polyphonic timbres (those that play chords) appear as candidates in the grid sequencer's chord lines (unreadable timbres are not shown as candidates for chord lines).
- Some of Vaporizer2's factory presets with "MPE" in their name will not produce sound in cmrt. These timbres assume MPE (note-per-pitch/pressure) performance information, which cmrt does not send.
- The default category settings for filtering candidates by line purpose (chord / bass / arpeggio / drum 4 roles / others) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (= all programs are candidates for all lines). This is because Dexed's cartridges are not "directory name = purpose," and for non-built-in plugins, the timbre organization is unknown, so no filtering is applied. If you want to change this, write the 7 items in `[plugins.<Name>]` (the generated `config.toml` includes Surge XT's defaults as comments at the end).
- The 7 purpose-specific category items should also be written only within the plugin profile. For Surge XT, it goes in `[plugins."Surge XT"]`; for other plugins, it goes in that plugin's own table.
- The shared voicing determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by purpose is only used for Surge XT's timbre determination.
- Rendering results are cached in separate directories per plugin, so mixing plugins will not lead to accidental use of sounds from different plugins (no manual deletion needed). The cache locations are these two, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAVs)

If `offline_render_backend = "render_server"`, the TUI itself does not directly load CLAP plugins. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, cmrt will launch a child process and retry once if a communication error occurs.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - When MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C, D, E.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- Also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- Displays how many timbre candidates are available for selection via the PATCH column wheel for each line in the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Use this after changing plugins, `patches_dirs`, or purpose-specific categories (`chord_patch_categories`, etc.) to confirm that the wheel is not unresponsive.
- If any line has 0 candidates, it will list that line and exit with code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test how changes would affect settings without modifying your current `config.toml`.
- When timbres from multiple plugins are listed, the output also includes a plugin-specific breakdown of candidates per purpose. This helps to notice if a specific plugin's timbres are not appearing for a given line, which might be missed by only looking at the total.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and a digest value of the sound output on one line. No screen is launched.
- While `patch-roles` counts whether timbres appear in the list, this command checks whether the timbre actually produces sound.
- `--patch` can be specified multiple times. A summary line will show "N / M distinct outputs," which helps verify if the **sound remains the same even after changing the timbre**.
- Adding `--out-dir <directory>` writes WAV files (otherwise, no bytes are written). Use this when you want to listen to the output.
- Adding `--poly-check` compares playing a chord and a single note to determine if the timbre plays chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- The Surge XT patches should ideally be obtained via an API (currently, they are inefficiently explored from `toml` specified directories. Implementation timing is deferred to prioritize other features).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "one-measure offline rendering,"
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching and quickly iterating through edits.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - Note: The term "Atomic Measure" in a music production context might evoke an indivisible unit of musical time, which aligns with its use here.

# Out of Scope
- Effects are essential for editing, but we've consciously decided to put them out of scope and prioritize them much later. One reason is that Surge XT's patches include effects (effects are derived from patches).