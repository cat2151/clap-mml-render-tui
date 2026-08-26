# clap-mml-render-tui

### Overview
A MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Purpose

- For playing around with sounds using MML
- For casual installation. Only Rust is required.

### Technology Stack
- Plugin Host Library
  - https://github.com/prokopyl/clack

### Prerequisites

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

- `c d e f g a b` keys: Play CDEFGAB.

### Configuration

Upon first launch, `config.toml` will be automatically created. It is located in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Current configuration example:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# under the configuration directory.
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
# Notepad mode: Plays current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only write if changing Surge XT's default values
[plugins."Surge XT"]
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

Configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Candidate editors, tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in-process renders. |
| `offline_render_backend` | `in_process` | Target for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server instances. |
| `offline_render_server_port` | `62153` | localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback. |
| `realtime_play_server_port` | `62154` | localhost port for play_server. |
| `realtime_play_server_command` | Empty string | Command to launch play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for Surge XT patches. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the loop browser. Run `cmrt scan-loops` after changes. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories that can be assigned to loop directories. The key for the category overlay is determined from an unused letter in the category name. |

The default `plugin_path` for each OS is as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` for each OS is as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (If `XDG_DATA_HOME` is not set, `~/.local/share` is used)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines where no patch is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog via `[plugins.<Name>]` and used on lines where the patch is explicitly stated.

The contents of the built-in profiles are as follows, with paths pointing to the standard installation locations for each OS:

| Name | plugin_id | patches_dirs | Categories by Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<Name>]` if the plugin is installed in a non-standard location, if you want to supplement the patch location, or if you are using a plugin not included in the built-in profiles. **Only the explicitly written items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only swap the path. plugin_id and patches_dirs remain the built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, write all properties.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Location for that plugin's patches. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filtering for automatic patch selection by usage. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If not specified, the plugin's default values will be used (Surge XT uses category names, others use 'no filtering'). |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error, rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default, while others become candidates in the mixed catalog.
- Dexed patches are '1 `.syx` cartridge = 32 programs', so in the list, each program is displayed as if the cartridge were a directory, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is not patch-specific but an instance setting (`MonoMode`), and its default is POLY. Therefore, all Dexed patches are treated as chord-friendly in the grid sequencer's chord lines.
- Vaporizer2 patches are '1 `.vvp` file = 1 patch', and can be selected just like Surge's `.fxp` files. The category shown in the list heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs`. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing to it automatically could damage your DAW environment. Please add a single line as shown below. Until you do, it will not appear in the catalog with 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting varies per patch and is read from the `.vvp` content (`m_uPolyMode`). Therefore, only patches that play chords will appear as candidates in the grid sequencer's chord lines (patches that cannot be read will not appear as candidates for chord lines).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These patches are designed to work with MPE (per-note pitch and pressure) performance information, which `cmrt` does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / 4 drum roles / others) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for all lines). This is because Dexed cartridges do not follow a 'directory name = usage' convention, and for non-built-in plugins, the patch organization system is unknown, so no filtering is applied. If you want to change this, add the 7 items to `[plugins.<Name>]` (the default Surge XT values are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly judgment data (`voicing_shared_source` / `voicing_override_source`) used for usage-based automatic selection is only applied to Surge XT patch judgments.
- Rendering results are cached in separate directories for each plugin, preventing accidental use of sounds from different plugins even when mixed (no manual deletion required). There are two cache locations, where `<Plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<Plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<Plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI side does not directly load the CLAP plugin but instead sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension.
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play CDE.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh.
- It also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- For each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others), shows how many patch candidates are available in the PATCH field's wheel. The screen does not launch.
- Use this to check if the wheel is unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.).
- If any line has 0 candidates, it lists those lines and exits with exit code 1.
- If `--config <path>` is added, it reads that `config.toml`. This allows you to test how changes in settings affect things without modifying your current `config.toml`.
- When patches from multiple plugins are listed, the candidate count per usage also shows a breakdown by plugin. This is because summing only would not reveal if "a specific plugin's patch isn't appearing for a given line at all".

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the sound output on a single line. The screen does not launch.
- While `patch-roles` counts whether a patch appears in the list, this command checks if that patch actually produces sound.
- You can specify any number of `--patch` arguments. The summary line will show "N / M distinct outputs", so you can check if **the sound remains the same even after changing patches**.
- If `--out-dir <directory>` is added, it writes WAV files (if not added, it writes zero bytes). Use this when you want to verify by ear.
- If `--poly-check` is added, it compares chords and single notes to determine if the patch can play polyphonically.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Retrieving Surge XT patches via API is the proper way, so that will be implemented (currently, they are discovered via TOML, which is inefficient. Implementation timing is deferred, other priorities are higher).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing 'offline rendering in single-measure units',
    - while accepting certain constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Note: 'atomic measure' might sound like a physics term, so for now, I'll keep the Japanese 'アトミック小節' (Atomic Measure) without directly translating it as 'atomic measure' in English.

# Out of Scope
- Effects require editing, so we've decided to put them out of scope and defer them significantly. One reason for this is that in Surge XT's case, patches inherently include effects (effects are derived from patches).