# clap-mml-render-tui

### Overview
An MML TUI DAW (or similar). Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with sounds using MML
- For casual installation. Only Rust is required

### Tech Stack
- Plugin host library
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

### Execution

```
cmrt
```

You can input MML and play in the TUI screen.

#### Play Server Details

Sound playback is handled by a separate process, the play server. The play server executable is determined in the following order, using the first one found:

1.  Full path specified by `--play-server <PATH>` (If the specified path does not exist, it will stop with an error without proceeding to search).
2.  `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3.  Release build of the sibling repository (`../clap-mml-play-server/target/release/`).

The PATH environment variable is not used. Debug build servers are 4-5 times slower in pre-reading, causing playback to break at the start of a measure.
A warning appears in the upper-right corner of the screen when a debug build or an executable with an unknown origin is being used.

```
cmrt --play-server "N:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to those available for free on CLAP, Windows, and without account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI Generated Documentation
- From this point onwards, sections added by AI may be hard to read. We will maintain them periodically.

### Keyboard Screen

Press the `v` key to switch to the keyboard screen.

- `c d e f g a b` keys: Play the Do-Re-Mi scale.

### Settings

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, press `e` to open `config.toml` in an editor. After closing the editor, restart the application.

Current configuration example:

```toml
# 【Required】CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi and output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders in DAW mode (1-16)
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

# Whether to autoplay on startup
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for WAV loops
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
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate during rendering. |
| `buffer_size` | `512` | Buffer size during rendering. |
| `offline_render_workers` | `2` | Number of concurrent in-process renders. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server instances. |
| `offline_render_server_port` | `62153` | render_server localhost port. |
| `offline_render_server_command` | Empty string | Command to launch the render_server. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback execution. |
| `realtime_play_server_port` | `62154` | play_server localhost port. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directories | List of directories to search for Surge XT patches. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused alphabetic characters within the category names. |

OS-specific `plugin_path` default values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific `patches_dirs` default values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified patch is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<name>]` and used on lines where the patch is explicitly specified.

The contents of the built-in profiles are as follows, with paths set to the standard installation locations for each OS.

| Name | plugin_id | patches_dirs | Category for specific use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Surge XT's category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default value. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched case-insensitively, ignoring differences in spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you have installed a plugin in a non-standard location, need to specify patch directories, or are using a plugin not built-in. **Only the written items will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only change the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For a plugin not built-in, specify everything.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Patch directory for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<purpose>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use case. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will apply to that plugin. If not specified, the plugin's default value will be used (Surge XT uses category names, others use 'no filtering'). |

- `active_plugin` is deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 use-case categories at the top level will result in a configuration error, rather than silently ignoring them. Please remove `active_plugin` and move other values into `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default value; others will become candidates in the mixed catalog.
- Dexed patches are '1 `.syx` cartridge = 32 programs', so in the list, cartridges are treated as directories, and each program is listed individually, like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not part of the patch itself, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord lines.
- Vaporizer2 patches are 1 `.vvp` file = 1 patch, and can be selected just like Surge's `.fxp` files. The category appearing in the list heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing to it arbitrarily could break your DAW environment. Please add a single line as shown below. Until you do, it will appear in the catalog as having 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting varies per patch and is read from the contents of the `.vvp` file (`m_uPolyMode`). Therefore, only patches that play chords will appear as candidates in the grid sequencer's chord lines (patches that could not be read are not offered as chord line candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These patches are designed for MPE (per-note pitch and pressure) performance information, which cmrt does not send.
- The default category settings for filtering candidates by line purpose (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for all lines). This is because Dexed cartridges do not follow a 'directory name = purpose' convention, and for non-built-in plugins, the patch directory structure is unknown, so no filtering is applied. If you wish to change this, add the 7 items to `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 use-case categories should also be written only within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in that plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by use case is only applied to Surge XT patch determination.
- Rendering results are cached in separate directories for each plugin, so even when mixed, sounds from different plugins are not misused (no manual deletion is required). The two locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"` is set, the TUI will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAV data. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works with the bluesky-text-to-audio Chrome extension.
- When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play Do-Re-Mi.

```
cmrt CM7
```

- Typing `CM7` will play C Major Seventh.
- It also supports various chord progression notations (some are not yet supported).

### Patch-Roles Command

```
cmrt patch-roles
```

- Displays how many patch candidates are available for selection with the PATCH wheel in each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Used to check if the PATCH wheel has become unresponsive after changing plugins, `patches_dirs`, or use-case categories (e.g., `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with status code 1.
- Adding `--config <path>` will load that `config.toml`. This allows you to test how changes to settings would affect behavior without modifying your current `config.toml`.
- When multiple plugin patches are listed, the breakdown by plugin will also be shown for each use-case's candidate count. This is because simply showing the total might not reveal that 'a specific plugin has no patches available for a given line'.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render-MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Performs offline rendering of MML with the specified patch, and displays the length, volume (`peak` / `rms`), whether it's silent, and an audio digest value on a single line. No screen is launched.
- While `patch-roles` counts whether a patch appears in the list, this command checks whether that patch actually produces sound.
- You can specify `--patch` multiple times. A summary line will show 'N / M distinct outputs', allowing you to check if the **patch changed but the sound remained the same**.
- Adding `--out-dir <directory>` will write WAV files (if not specified, no bytes are written). Use this when you want to verify by ear.
- Adding `--poly-check` will compare playing chords and single notes to determine if the patch can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes occur daily.

# Future Plans
- It is more appropriate to retrieve Surge XT patches via API, so this will be implemented (currently, they are inefficiently searched via toml specification. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measures
    - Inspired by Obsidian's atomic notes concept.
    - By making the unit of all processing 'offline rendering in 1-measure units',
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *The term 'atomic measure' could be confused with a term in physics, so for now, we'll keep the Japanese term 'アトミック小節' (Atomic Measure) without direct English translation, but the concept is as described.

# Out of Scope
- Effects require editing, so we've decided to put them out of scope and prioritize them much later. One reason for this is that Surge XT's patches include effects (effects are derived from patches).