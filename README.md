# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Requires only Rust.

### Tech Stack
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

### Run

```
cmrt
```

You can input MML and play around in the TUI.

### Supported Audio Plugins
- ※ Limited to CLAP plugins that are free, available on Windows, and do not require account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI-Generated Documentation
- The following sections, added by AI, may be difficult to read. I will maintain them periodically.

### Keyboard Screen

Press the `v` key to navigate to the keyboard screen.

- `c d e f g a b` keys: Play the C, D, E, F, G, A, B notes.

### Configuration

`config.toml` is automatically created on first launch. It is located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is a current configuration example.

```toml
# [REQUIRED] CLAP plugin to use
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

# Number of concurrent offline rendering workers for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
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

# List of directories to search for in the WAV loop browser
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

The configuration items are as follows.

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | Surge XT CLAP standard path per OS | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` rendering workers. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` workers. |
| `offline_render_server_port` | `62153` | `render_server` localhost port. |
| `offline_render_server_command` | Empty string | `render_server` startup command. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback execution. |
| `realtime_play_server_port` | `62154` | `play_server` localhost port. |
| `realtime_play_server_command` | Empty string | `play_server` startup command. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `plugins."Surge XT".patches_dirs` | Surge XT patches standard directory per OS | List of directories to search for in Surge XT patch selection. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused English letters within the category name. |

The default `plugin_path` values per OS are as follows.

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values per OS are as follows.

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified patch is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog from `[plugins.<name>]` and used on lines where the patch is explicitly specified.

The contents of the built-in profiles are as follows, with paths pointing to the standard installation location for each OS.

| Name | `plugin_id` | `patches_dirs` | Category by Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default value. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you've installed it in a non-standard location, need to supplement patch directories, or are using a plugin not built-in. **Only the specified items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Only replace the path. plugin_id and patches_dirs remain as built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, write all details.
[plugins.my_synth]
plugin_path   = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch directory for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<purpose>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use case. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items apply to that plugin. If not specified, the plugin's default values (Surge XT uses category names, others use 'no filtering') will be used. |

- `active_plugin` is deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 use-case categories at the top level will result in a configuration error, rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default values; others become candidates in the mixed catalog.
- Dexed patches are '1 `.syx` cartridge = 32 programs', so in the list, each program is displayed individually, treating the cartridge as a directory, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, 0-indexed). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord lines.
- Vaporizer2 patches are 1 `.vvp` file = 1 patch, and can be selected just like Surge's `.fxp` files. The category appearing in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because preset locations are environment-dependent values determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing them arbitrarily could damage your DAW environment.
- Please add a single line as shown below. Until you do, it will not appear in the catalog with 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly varies per patch and is read from the contents of the `.vvp` file (`m_uPolyMode`). Therefore, only patches that play chords will appear as candidates in the grid sequencer's chord lines (patches that could not be read will not be shown as chord line candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These patches are designed to work with MPE (per-note pitch and pressure) performance information, which `cmrt` does not transmit.
- The default category settings for filtering candidates by line purpose (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, while Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for all lines).
- This is because Dexed cartridges do not follow a 'directory name = purpose' convention, and for non-built-in plugins, the patch directory structure is unknown, so no filtering is applied. If you wish to change this, write the 7 items in `[plugins.<name>]` (Surge XT's default values are included as comments at the end of the generated `config.toml`).
- The 7 use-case categories should also only be written within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by purpose is only applied to Surge XT patch determination.
- Rendering result caches are placed in separate directories per plugin, so there's no accidental misuse of sounds from different plugins even when mixed (no manual deletion is required). The two locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows).
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAV data. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C-D-E notes.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- Displays the number of patch candidates available for selection via the PATCH wheel in each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / other). The screen does not launch.
- Used to check if the PATCH wheel remains unresponsive after changing plugins, `patches_dirs`, or use-case categories (`chord_patch_categories`, etc.).
- If any line has 0 candidates, it will list that line and exit with code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test how changes to settings would behave without modifying your current `config.toml`.
- When multiple plugin patches are listed, the breakdown by plugin will also be shown for each use-case's candidate count. This is because relying only on the total count might not reveal if "a specific plugin has no patches available for that line".

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and a sound digest value on a single line. The screen does not launch.
- While `patch-roles` counts whether a patch appears in the list, this command checks whether that patch actually produces sound.
- You can use `--patch` multiple times. The summary line shows "Different sounds N / M", which helps verify if **the patch changed but the sound remained the same**.
- Adding `--out-dir <directory>` writes WAV files (otherwise, no bytes are written). Use this when you want to verify by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the patch can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes sense to retrieve Surge XT patches via API, so that will be implemented (currently, they are inefficiently searched for based on `toml` specifications. Implementation timing is deferred, prioritizing other tasks).

# Concept Notes
## アトミック小節
- Inspired by Obsidian's Atomic Notes.
- By making the unit of all processing "offline rendering in one-measure increments",
- while incurring constraints,
- various benefits can be gained.
- This approach is suitable for sketching and rapid editing cycles.
- For more serious editing, existing feature-rich DAWs would be more suitable.
- ※ If translated as 'atomic measure', it would become a physics term, so for now, the term 'アトミック小節' is left untranslated.

# Out of Scope
- Since effects require editing, they are intentionally deemed out of scope and postponed significantly. One reason for this is that Surge XT patches often encapsulate effects (effects are derived from patches).