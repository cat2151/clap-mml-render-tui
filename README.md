# clap-mml-render-tui

### Overview
MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with sounds using MML
- For casual installation. Having Rust is enough.

### Technology Stack
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

You can input MML in the TUI screen and play around.

#### The `play server` executable

Sound generation is handled by a separate process, the `play server`. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it stops with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`)

The PATH environment variable is not used. Debug build servers are 4-5 times slower in pre-reading, causing playback to cut off at the beginning of measures. A warning appears in the upper right corner of the screen when a debug build or an executable of unknown origin is being used.

```
cmrt --play-server "N:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- ※ Limited to those available for free on CLAP, Windows, without account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI-Generated Documentation
- The parts added by AI from here on may be difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press `v` to navigate to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B (Do Re Mi Fa Sol La Si).

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to view and edit the 'structure' of a song's chord progression. Moving the cursor plays the chord progression of that line.

- Left pane (Sections): Define named chord progressions that serve as building blocks.
- Right pane (Arrangement): Arrange the defined sections to form a song. The same section can be used multiple times.
- If you modify a section's progression, all its references in the song will change accordingly.
- The first line of the header shows the chord2mml specification (`Key=C BPM120`, etc.) to be placed at the beginning of the song.
- This screen does not interpret progressions or headers. It simply stores the typed strings as they are.
- Only the section on the cursor line will play (not the entire song). The timbre is fixed and cannot be selected on this screen.
- Only the 'Key' in the header is passed for auditioning (BPM remains at its default).

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:Exit ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

Here are the keybindings. Only `q` and `?` are shown on the bottom line of the screen. The full list appears on-screen when you press `?`.

| Key | Pane | Action |
|---|---|---|
| `h` `l` | Common | Pane movement (`h`:Sections `l`:Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement |
| `PgUp` `PgDn` | Common | Move cursor 10 lines |
| `dd` | Common | Delete cursor line (deleting from Sections also removes references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor line up / down |
| `b` | Common | Rewrite header Key / BPM with single-line input |
| `Shift+P` `Space` | Common | Audition the section on the cursor line (stops if already playing) |
| `?` | Common | Toggle help (closes with `Esc` as well) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Redraw the progression of the cursor line, keeping its name |
| `i` | Sections | Edit progression with single-line input |
| `n` | Sections | Edit name with single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

It is automatically saved every time you edit. The save location is `clap-mml-render-tui/history/chord_chart.json` under the configuration directory (on Windows, `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If the save file is missing or unreadable, when the screen is first opened, it performs a single drawing operation (same as `g`) to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` drawing is fetched from the network. There will be a wait only for the first time if the cache is not yet present (the waiting time is recorded in `log.txt` as `chord-chart: event=catalog-first-load elapsed_ms=...`). If it cannot be fetched, "Chord progression data is not available" will appear at the bottom of the screen.

### Configuration

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of TUI / DAW, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is a current configuration example.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent DAW offline renderings (1-16)
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

# Whether to autoplay on startup
# notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
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
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` renderings. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` workers. |
| `offline_render_server_port` | `62153` | `render_server` localhost port. |
| `offline_render_server_command` | Empty string | Command to start `render_server`. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback execution. |
| `realtime_play_server_port` | `62154` | `play_server` localhost port. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for Surge XT patch selection. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused English letters in the category name. |

OS-specific default values for `plugin_path` are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default values for `patches_dirs` are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without explicit timbre specification is **Surge XT fixed**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog from `[plugins.<name>]` and used on lines where timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths being the OS-specific standard installation locations:

| Name | plugin_id | patches_dirs | Category by Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (i.e., no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you install plugins in non-standard locations, supplement patch directories, or use plugins not included by default. **Only the specified items override the built-in values**, so if you only want to change the Surge XT path, a single `plugin_path` line within its table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain as built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify everything.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch location for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by usage. Seven key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`) can be specified. Only the specified items will be effective for that plugin. If not specified, the plugin's default value (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error, rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default value; others will become candidates in the mixed catalog.
- Dexed patches are "1 `.syx` cartridge = 32 programs", so in the list, cartridges are treated as directories, and each program is listed individually, like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, 2 digits). By specifying the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is for the instance (`MonoMode`), not the patch itself, and its default is POLY. Therefore, all Dexed patches are treated as chord-friendly in the grid sequencer's chord lines.
- Vaporizer2 patches are "1 `.vvp` file = 1 patch", and can be selected just like Surge's `.fxp` files. The category shown in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 is the only plugin that does not have a default `patches_dirs` value. This is because preset locations are environment-dependent values determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing to them arbitrarily could damage your DAW environment. Please add the following line. Until then, it will not appear in the catalog with 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly varies per patch and is read from the `.vvp` file's content (`m_uPolyMode`). Therefore, only chord-playing patches appear as candidates in the grid sequencer's chord lines (patches that cannot be read are not offered as candidates for chord lines).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These patches assume MPE (per-note pitch/pressure) performance data, which `cmrt` does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (i.e., all programs are candidates for all lines). This is because Dexed cartridges do not follow a "directory name = usage" structure, and for non-built-in plugins, the patch organization system is unknown. If you wish to change this, specify the 7 items under `[plugins.<name>]` (Surge XT's default values are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also be written only within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly judgment data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by usage is only for Surge XT patch judgment.
- Rendering result caches are placed in separate directories for each plugin, preventing accidental use of sounds from other plugins even when mixed (no manual deletion is required). The two locations are: `<plugin>` is the filename (without extension) of the resolved `plugin_path` (on Windows).
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI does not directly load CLAP plugins. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, `cmrt` launches a child process and, in case of a communication error, retries after a single restart.

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

- Type `cde` to play Do Re Mi (CDE).

```
cmrt CM7
```

- Type `CM7` to play a C Major Seventh chord.
- Also supports various chord progression notations (some are not yet supported).

### Patch Roles Command

```
cmrt patch-roles
```

- Displays the number of patch candidates available for selection with the PATCH field's wheel for each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The screen does not launch.
- Use this to check if the wheel in the PATCH field has become unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.).
- If any line has 0 candidates, it lists that line and exits with error code 1.
- If you add `--config <path>`, it reads that `config.toml`. You can test how changes to settings will affect things without modifying your current `config.toml`.
- When patches from multiple plugins are present, the candidate count per usage also includes a plugin-specific breakdown. This is because relying only on the total might obscure that "a certain plugin has no patches available for that line".

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays its length, volume (`peak` / `rms`), whether it's silent, and an audio output digest value on a single line. The screen does not launch.
- Any number of `--patch` arguments can be provided. The summary line shows "N / M distinct outputs", allowing you to check if the **sound remained the same despite changing the patch**.
- If you add `--out-dir <directory>`, it writes WAV files (otherwise, it writes no bytes). Use this when you want to verify by ear.
- If you add `--poly-check`, it compares playing chords and single notes to determine if the patch can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is more appropriate to obtain Surge XT patches via API, so this will be implemented (currently, they are searched for from `toml` specifications, which is inefficient. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- アトミック小節 (Atomic Measure)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing 'offline rendering in 1-measure units',
    - while imposing constraints,
    - it offers various benefits.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - ※ Since 'atomic measure' tends to be a term in physics, for now, I will keep 'アトミック小節' without translating it.

# Out of Scope
- Effects are essential for editing, so we've decided to consider them out of scope and push them far back in priority. One reason for this is that in Surge XT, patches already encapsulate effects (effects are derived from patches).