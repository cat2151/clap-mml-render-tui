# clap-mml-render-tui

### Overview
A MML TUI DAW (or something similar). Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Purpose

- For playing around with MML to generate sounds
- For casual installation. Just having Rust is enough.

### Technology Stack
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

### Execution

```
cmrt
```

You can enter MML in the TUI screen and play around.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play CDEFGAB notes.

### Configuration

Upon first launch, `config.toml` will be automatically created. It will be located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` will open `config.toml` with an editor. After closing the editor, restart the application.

Here is an example configuration.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under the configuration directory:
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering tasks for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Sends POST /render to a render-server child process for rendering.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# Realtime playback backend
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

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific standard Surge XT CLAP path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate during rendering. |
| `buffer_size` | `512` | Buffer size during rendering. |
| `offline_render_workers` | `2` | Number of concurrent in_process rendering tasks. |
| `offline_render_backend` | `in_process` | Destination for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server tasks. |
| `offline_render_server_port` | `62153` | Localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Destination for realtime audio playback. |
| `realtime_play_server_port` | `62154` | Localhost port for play_server. |
| `realtime_play_server_command` | Empty string | Command to launch play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directory | List of directories to search for patches (timbres/presets) for Surge XT. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for category overlay is determined from unused English letters in the category name. |

The default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified timbre is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<name>]` and used in lines where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths being the standard installation locations per OS.

| Name | plugin_id | patches_dirs | Category for Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched case-insensitively and ignoring spaces/underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You should only write `[plugins.<name>]` if you've installed a plugin in a non-standard location, need to supplement patch directories, or are using a plugin not built-in. **Only the specified items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only swap the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch (timbre/preset) directory for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by usage. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If not specified, the plugin's default value (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error rather than silently ignoring them. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only profiles with the same name as Surge XT will override the fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are "1 `.syx` cartridge = 32 programs", so in the list, cartridges are treated like directories, and programs are listed one by one like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2 digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not part of the timbre, and its default value is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord lines.
- Vaporizer2 timbres are "1 `.vvp` file = 1 timbre", and can be selected just like Surge's `.fxp` files. The category appearing in the list header is the **first 2 characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs`. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and cmrt arbitrarily reading/writing there could damage your DAW environment. Please add a single line as shown below. Until you do, it will not appear in the catalog as having 0 timbres.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting differs per timbre and is read from the content of the `.vvp` file (`m_uPolyMode`). Therefore, only timbres that can play chords will appear as candidates in the grid sequencer's chord lines (timbres that could not be read will not appear as candidates in chord lines).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These timbres assume MPE (per-note pitch and pressure) performance information, which cmrt does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / 4 drum roles / others) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for all lines). This is because Dexed cartridges do not follow a "directory name = usage" convention, and the timbre storage system for non-built-in plugins is unknown, so they are not filtered. If you wish to change this, specify the 7 items in `[plugins.<name>]` (the generated config.toml will have Surge XT's default values commented out at the end).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by usage is only used for Surge XT timbre determination.
- Rendering results are cached in separate directories for each plugin, so even if mixed, sounds from different plugins will not be misused (no manual deletion is required). The two storage locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (cache for notepad / MML input overlay)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"` is set, the TUI side will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAVs. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension
  - When a Bluesky post contains MML, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing "cde" will play CDE (Do-Re-Mi).

```
cmrt CM7
```

- Typing "CM7" will play a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- Displays the number of timbre candidates available in the PATCH field for each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / other). The screen does not launch.
- Use this after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.) to check if "turning the wheel has no effect".
- If there are lines with 0 candidates, it lists those lines and exits with code 1.
- Adding `--config <path>` reads that specific config.toml. This allows you to test how settings change without modifying your currently used config.toml.
- When multiple plugin timbres are listed, the candidate count for each usage will also show a breakdown by plugin. This is because summing them up alone might hide the fact that "a certain plugin's timbre does not appear in that line at all".

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound, all on a single line. The screen does not launch.
- `--patch` can be specified multiple times. A summary line will show "N / M different outputs", so you can check if the sound **remains the same even after changing the timbre**.
- Adding `--out-dir <directory>` writes WAV files (if not added, no bytes are written). Use this when you want to verify by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is more proper to obtain Surge XT patches via API, so that will be done (currently, they are inefficiently searched from TOML-specified paths. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measures
    - Inspired by Obsidian's Atomic Notes.
    - By making the unit of all processing "offline rendering per measure,"
    - while accepting limitations,
    - various benefits can be gained.
    - This is suitable for sketching and quickly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Note: "atomic measure" sounds like a term from physics, so for now, it remains untranslated as "アトミック小節" (Atomic Measures).

# Out of Scope
- Effects are essential for editing, so they are intentionally kept out of scope and deferred to a much later stage. One reason for this is that Surge XT's patches already encapsulate effects (effects are derived from patches).