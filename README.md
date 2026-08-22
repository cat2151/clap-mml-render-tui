# clap-mml-render-tui

### Overview
A TUI DAW (of sorts) for MML. Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Purpose

- For playing around with MML sounds
- For casual installation. Rust is all you need.

### Technology Stack
- Plugin Host Library
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

### Usage

```
cmrt
```

You can input MML and play around in the TUI screen.

### Keyboard Screen

Press the ``v` key to move to the keyboard screen.

- ``c d e f g a b` keys: Play C, D, E, F, G, A, B.

### Configuration

A `config.toml` file is automatically created on the first launch. It is located in the OS's standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of TUI / DAW, pressing ``e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example of the current configuration.

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
# Notepad mode: Plays the current line immediately. DAW mode: Starts playing from the beginning of the song (measure 0).
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
| `plugin_path` | Surge XT CLAP standard path for each OS | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for in_process. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server instances. |
| `offline_render_server_port` | `62153` | Localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback execution. |
| `realtime_play_server_port` | `62154` | Localhost port for play_server. |
| `realtime_play_server_command` | Empty string | Command to launch play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `patches_dirs` | Surge XT patches standard directory for each OS | List of directories to search for sound presets. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from unused English letters in the category name. |

The default `plugin_path` values for each OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values for each OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using Multiple Plugins

You can switch with a single `active_plugin` line. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line (Vaporizer2 is also built-in, but you need to specify the patch location as described below).

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths set to the standard installation location for each OS.

| Name | plugin_id | patches_dirs | Categories for use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Use top-level settings (i.e., Surge's category names) as-is |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched case-insensitively, ignoring spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

Only write `[plugins.<name>]` if you've installed plugins in non-standard locations or if you're using a plugin not built-in. **Only the specified items will override the built-in values**, so if you just want to change the path, one line for `plugin_path` is sufficient.

```toml
active_plugin = 'Surge XT'

# Only override the path. plugin_id and patches_dirs remain at their built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | The name of the profile to use. Specify either a built-in name or a `[plugins.*]` name. If omitted, the top-level `plugin_path` / `patches_dirs` will be used. |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch location for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<use>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use case. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If omitted, the plugin's default value (Surge XT uses category names, others use "no filtering") will be used. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` are not used (no error occurs, the profile takes precedence). For use-case specific categories as well, items specified in the profile (including the 7 items for built-in Dexed) take precedence.
- If the `active_plugin` name is neither built-in nor found under `[plugins.*]`, it will fail to launch with an error. All available names will be displayed.
- Dexed patches are "1 `.syx` cartridge = 32 programs", so in the list, cartridges are treated as directories, and programs are listed one by one like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2 digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default value is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord rows.
- Vaporizer2 patches are "1 `.vvp` file = 1 patch", and can be selected just like Surge's `.fxp` files. The category displayed in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing there arbitrarily could corrupt your DAW environment. Please add one line as follows. Until you do, no patches will appear in the catalog (0 items).

```toml
active_plugin = 'Vaporizer2'

[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting varies per patch and is read from the contents of the `.vvp` file (`m_uPolyMode`). Therefore, only patches that can play chords will appear as candidates in the grid sequencer's chord rows (patches that could not be read will not be shown as candidates for chord rows).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These patches are designed to use MPE (per-note pitch and pressure) performance information, which `cmrt` does not send.
- The default category settings for filtering candidates by row purpose (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (= all programs are candidates for all rows). This is because Dexed cartridges do not have "directory name = purpose", and the patch organization of non-built-in plugins is unknown, so no filtering is applied. If you want to change this, write the 7 items under `[plugins.<name>]` (the generated config.toml will have Surge XT's default values commented out at the end).
- The 7 items can also be written at the top level, but they are **only effective for the default plugin (the one that plays lines with unspecified patches)**. This is an old writing style from before `active_plugin` existed, and newly generated config.toml files will not write them at the top level. Existing configs that already have them at the top level will continue to work.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for use-case specific automatic selection is exclusive to Surge XT. It is not retrieved when using plugins other than Surge XT.
- The rendering result cache is stored in separate directories for each plugin, so switching plugins will not play sounds from the previous plugin (no need to manually delete them). The two cache locations are as follows, where `<plugin>` is the filename of `plugin_path` (without extension) (for Windows):
  - `%LOCAL_APPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (cache for notepad / MML input overlay)
  - `%LOCAL_APPDATA%\clap-mml-render-tui\daw\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI side will not directly load the CLAP plugin. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interacts with the bluesky-text-to-audio Chrome extension
  - When an MML is found in a Bluesky post, it can be played with Surge XT.

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

### patch-roles command

```
cmrt patch-roles
```

- For each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others),
  it displays how many patch candidates are available for selection via the PATCH wheel. The GUI does not launch.
- After changing plugins, `patches_dirs`, or use-case specific categories (`chord_patch_categories`, etc.),
  this is used to check if the "wheel is unresponsive".
- If any row has 0 candidates, it will list that row and exit with code 1.
- Adding `--config <path>` will read that `config.toml`. This allows you to test how settings change
  without modifying your currently active `config.toml`.
- When patches from multiple plugins are listed, the breakdown by plugin will also be shown for each use-case's candidate count.
  This is because simply summing them might obscure the fact that "a certain plugin's patches are not appearing at all for a specific row."

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch, and displays length, volume (`peak` / `rms`), whether it's silent, and
  a digest value of the output sound on a single line. The GUI does not launch.
- While `patch-roles` counts "if a patch appears in the list", this command checks "if that patch actually produces sound."
- `--patch` can be specified multiple times. A summary line showing "N / M different sounds" helps verify if
  **the sound hasn't changed despite switching patches**.
- Adding `--out-dir <directory>` will write WAV files (if not specified, no bytes are written).
  Use this when you want to verify by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the patch can play polyphonically.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is logical to retrieve Surge XT patches via API, so this will be implemented (currently, patches specified in toml are explored, which is inefficient. Implementation timing is deferred; other priorities are higher).

# Concept Notes
- Atomic Measures
  - Inspired by Obsidian's atomic notes.
  - By making the unit of all processing "offline rendering in 1-measure units,"
  - while accepting constraints,
  - various benefits can be gained.
  - This approach is suitable for sketching and for quickly iterating through editing cycles.
  - For more serious editing, existing feature-rich DAWs would be more suitable.
  - *Note: "atomic measure" might sound like a physics term, so for now, I'll keep it as "アトミック小節" without direct English translation, though "Atomic Measures" is used for the header.

# Out of Scope
- Effects require mandatory editing, so they are intentionally kept out of scope and deferred to a much later stage. One reason for this is that Surge XT patches often encapsulate effects (effects are derived from patches).