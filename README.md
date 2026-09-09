# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy rich sounds from Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Just having Rust is enough.

### Technical Stack
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

### Run

```
cmrt
```

You can enter MML in the TUI screen and play around.

#### Play server's actual process

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further).
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`).

PATH is not searched. The debug build server's lookahead is 4-5 times slower, causing playback to cut off at the beginning of measures.
A warning will appear in the top right of the screen if you are using a debug build or an unknown executable.

```
cmrt --play-server "N:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP, Windows, and free plugins available without account registration*
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI Generated Documentation
- The following sections, added by AI, might be hard to read. I will maintain them periodically.

### Keyboard Screen

Press `v` to switch to the keyboard screen.

- `c d e f g a b` keys: Play the notes C, D, E, F, G, A, B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to switch to the chord chart screen.

This screen allows you to overview and edit the "structure" of chord progressions for a song. **It does not play any sound.**

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): Arrange the defined sections to form a song. The same section can be used multiple times.
- If you modify a section's progression, all references to that section in the arrangement will change simultaneously.
- The single line in the header displays the `chord2mml` specification placed at the beginning of the song (e.g., `Key=C BPM120`).
- This screen does not interpret progressions or headers. It stores the entered strings as-is.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ hl:pane g:抽選 r:引直し i:進行 n:名前 dd:削除 Alt+↑↓:移動 q:終了 ?:help      │
└──────────────────────────────────────────────────────────────────────────────┘
```

Here are the keybindings. Press `?` to see the full list on screen (due to width constraints, `b` is omitted from the bottom line).

| Key | Pane | Action |
|---|---|---|
| `h` `l` | Common | Move pane (`h`: Sections `l`: Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor |
| `PgUp` `PgDn` | Common | Move cursor by 10 lines |
| `dd` | Common | Delete cursor line (deleting from Sections also deletes all references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor line up / down |
| `b` | Common | Rewrite Key / BPM in the header with a single-line input |
| `?` | Common | Toggle help (`Esc` also closes it) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Redraw the progression for the cursor line, preserving the name |
| `i` | Sections | Edit the progression with a single-line input |
| `n` | Sections | Edit the name with a single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

Changes are automatically saved. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (e.g., `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json` on Windows). Only one song is saved at a time.
If no save file exists or it's unreadable, when the screen is first opened, it performs the same draw as `g` once to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. The first time, if the cache is empty, there will be a delay (the wait time is recorded in `log.txt` as `chord-chart: event=catalog-first-load elapsed_ms=...`). If fetching fails, "コード進行データがありません" (No chord progression data) will appear at the bottom of the screen.

### Configuration

A `config.toml` file is automatically created upon first launch. Its location is in the OS's standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in your editor. After closing the editor, restart the application.

Here is an example of the current configuration:

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the config directory.
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

# Whether to autoplay on startup
# Notepad mode: Plays current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for the WAV loop browser
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

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific standard path for Surge XT CLAP | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` renders. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` instances. |
| `offline_render_server_port` | `62153` | `render_server` localhost port. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback. |
| `realtime_play_server_port` | `62154` | `play_server` localhost port. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directories | List of directories to search for Surge XT patch selection. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for category overlay is determined from unused letters in the category name. |

OS-specific default values for `plugin_path`:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default values for `patches_dirs`:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without explicit patch selection is **fixed to Surge XT**. There is no `active_plugin` switch. Other plugins like Dexed are added to the mixed catalog via `[plugins.<Name>]` and used for lines where the patch is explicitly specified.

The built-in profile content is as follows, with paths pointing to OS-specific standard installation locations:

| Name | `plugin_id` | `patches_dirs` | Usage Category |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`.** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio` etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<Name>]` if you've installed a plugin in a non-standard location, need to add patch directories, or are using a plugin not built-in. **Only the written items will override the built-in values**, so if you just want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Patch directory for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filters for automatic patch selection by usage. You can specify seven key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will apply to that plugin. If not specified, the plugin's default (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the seven usage-specific categories at the top level will result in a configuration error, rather than being silently ignored. Please delete `active_plugin` and move other values into `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT overrides the fixed default; others become candidates in the mixed catalog.
- Dexed patches are "one cartridge `.syx` file = 32 programs," so they are listed as `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits) by treating the cartridge as a directory. If `patches_dirs` points to the cartridge location, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default is POLY. Therefore, all Dexed patches are treated as chord-suitable for grid sequencer chord lines.
- Vaporizer2 patches are "one `.vvp` file = one patch," selectable just like Surge's `.fxp` files. The category displayed in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. The preset location is determined by plugin-side global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), which are environment-dependent. `cmrt` deliberately avoids reading/writing here to prevent potential corruption of your DAW environment. Please add a line like this: `patches_dirs = ['D:\Vaporizer2\Presets']`. Until this is added, it will appear in the catalog as having 0 patches.
- Vaporizer2's mono/poly setting varies per patch and is read from the `.vvp` content (`m_uPolyMode`). Therefore, only patches capable of playing chords will appear as candidates in the grid sequencer's chord lines (unreadable patches are not listed as candidates).
- Some Vaporizer2 factory presets with "MPE" in their names will not produce sound in `cmrt`. These are patches designed for MPE (per-note pitch/pressure) performance data, which `cmrt` does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / 4 drum roles / other) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (i.e., all programs are candidates for all lines). This is because Dexed cartridges do not follow a "directory name = usage" pattern, and the patch organization of non-built-in plugins is unknown. If you want to change this, specify the 7 items in `[plugins.<Name>]` (the generated `config.toml` includes Surge XT's defaults as comments at the end).
- The seven usage-specific categories must also be placed only within the plugin profile. For Surge XT, it's `[plugins."Surge XT"]`; for other plugins, it's in their respective plugin tables.
- The shared voicing determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic mono/poly selection by usage is only for Surge XT patch determination.
- Render results are cached in separate directories per plugin, preventing accidental use of sounds from different plugins when mixing them (no manual deletion is needed). The locations are:
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<Plugin_Filename_No_Ext>\*.wav` (for notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<Plugin_Filename_No_Ext>\*.wav` (for DAW track WAVs)
  (Windows paths shown for example).

If `offline_render_backend = "render_server"` is set, the TUI itself does not directly load CLAP plugins. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, `cmrt` will launch a child process and, on a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interacts with the bluesky-text-to-audio Chrome extension.
  - When MML is found in a Bluesky post, it enables playing it with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` plays Do Re Mi.

```
cmrt CM7
```

- Typing `CM7` plays C Major Seventh.
- It supports various chord progression notations (some are not yet supported).

### patch-roles command

```
cmrt patch-roles
```

- Displays how many patch candidates are available for selection with the PATCH wheel for each line (chord / bass / arpeggio / 4 drum roles / other) in the grid sequencer. The screen does not launch.
- Use this to check if "the wheel is unresponsive" after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.).
- If any line has 0 candidates, it will list that line and exit with error code 1.
- Adding `--config <path>` reads that `config.toml`. This allows testing how changes affect behavior without modifying the currently used `config.toml`.
- When patches from multiple plugins are listed, the output will also include a plugin-specific breakdown of candidate counts per usage. This is because total counts alone might not reveal that "a specific plugin has no patches appearing for that line."

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound on a single line. The screen does not launch.
- While `patch-roles` counts "if a patch appears in the list," this command checks "if that patch actually produces sound."
- `--patch` can be specified multiple times. A summary line will show "N / M different outputs," which helps determine if **changing the patch unexpectedly produced the same sound as before**.
- Adding `--out-dir <directory>` writes WAV files (otherwise, no bytes are written). Use this when you want to listen to the result.
- Adding `--poly-check` compares playing chords and single notes to determine if the patch can play chords.
- `--config <path>` works the same as with `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Patches for Surge XT should ideally be retrieved via API (currently, they are inefficiently explored from those specified in `toml`. Implementation timing is postponed, prioritizing other tasks).

# Concept Notes
- アトミック小節 (Atomic Measure)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per 1 measure,"
    - while constrained,
    - various benefits can be gained.
    - This approach is suited for sketching and quickly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note: "Atomic measure" could be confused with physics terminology, so for now, it's left as "アトミック小節" (Atomic Measure) without further translation.*

# Out of Scope
- Effects require extensive editing, so they are intentionally placed out of scope and postponed significantly. One reason is that Surge XT patches often encapsulate effects (effects are derived from patches).