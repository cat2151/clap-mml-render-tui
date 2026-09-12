# clap-mml-render-tui

### Overview
A TUI DAW (of sorts) for MML. Enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando easily with MML. Written in Rust.

### Usage

- For experimenting with MML-driven audio
- For casual installation. Just having Rust is enough

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

Please install [Surge XT](https://surge-synthesizer.github.io/).

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

You can input MML and play around in the TUI screen.

#### The play server's implementation

Audio playback is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it will stop with an error instead of searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`)

The PATH environment variable is not consulted. The debug build server has 4-5 times slower pre-buffering, causing playback to cut off at the beginning of measures.
A warning appears in the top right of the screen when running a debug build or an unknown executable.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- * Limited to CLAP, Windows, and free-to-obtain without account registration
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI Generated Documentation
- The following sections, added by AI, may be difficult to read. I will maintain them occasionally.

### Keyboard Screen

Pressing the `v` key moves to the keyboard screen.

- `c d e f g a b` keys: Play the do-re-mi-fa-sol-la-ti notes.

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to view and edit the "structure" of a song's chord progression. Moving the cursor plays the chord progression of that row.
Use `h` `l` to step through chords within a row, playing only the selected chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- Changing a section's progression updates all its references throughout the song.
- The single header line displays chord2mml specifications (e.g., `Key=C BPM120`) to be placed at the beginning of the song, verbatim.
- This screen does not interpret progressions or headers; it stores the typed strings as-is.
- Only the section on the cursor's row plays (not the entire song). The patch is fixed and cannot be selected on this screen.
- Use `h` `l` to audition individual chords within a row. The currently selected chord is highlighted within the progression.
- Only the 'Key' from the header is used for auditioning (BPM remains at its default).

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:Quit ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

Key bindings. Only `q` and `?` are shown on the bottom line of the screen. The full list appears on screen by pressing the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Move pane (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. Plays the entire chord progression of the new row) |
| `h` `l` `←` `→` | Common | Move chord within row (plays only the selected chord. Wraps to adjacent rows at row ends) |
| `PgUp` `PgDn` | Common | Move cursor by 10 rows |
| `dd` | Common | Delete cursor row (deleting from Sections also removes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down |
| `b` | Common | Rewrite header Key / BPM with single line input |
| `Shift+P` `Space` | Common | Audition the section on the cursor row (stops if already playing) |
| `?` | Common | Toggle help (also closes with `Esc`) |
| `q` | Common | Quit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Redraw the progression of the cursor row, keeping its name |
| `i` | Sections | Edit progression with single line input |
| `n` | Sections | Edit name with single line input |
| `1`〜`9` | Arrangement | Insert the section of that number after the cursor |

Changes are automatically saved after each edit. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, this is `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If no save file exists or it cannot be read, the screen starts with a single section, generated once by the same lottery as `g`, upon opening (if no section can be drawn, it remains empty).

The chord progression catalog used for `g` / `r` lottery is fetched from the network. The first time, when there's no cache, you'll experience a wait (wait time is logged in `log.txt` under `chord-chart: event=catalog-first-load elapsed_ms=...`). If data fetching fails, "No chord progression data available" will appear at the bottom of the screen.

### Configuration

`config.toml` is automatically created on first launch. It is located within the OS's standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Current configuration example:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering processes for DAW (1-16)
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
# notepad mode: Immediately plays the current line. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only specify if you want to change Surge XT's default values
[plugins."Surge XT"]
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]
```

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | Standard Surge XT CLAP path per OS | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in_process renders. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server instances. |
| `offline_render_server_port` | `62153` | localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | localhost port for play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for Surge XT patches. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused alphabetic characters in the category name. |

Default `plugin_path` values per OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

Default `patches_dirs` values per OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (`~/.local/share` if `XDG_DATA_HOME` is not set)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multi-Plugin Support

The default plugin for lines without a specified patch is fixed to **Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<name>]` and used on lines where patches are explicitly specified.

The contents of the built-in profiles are as follows, with paths pointing to the standard installation locations for each OS:

| Name | `plugin_id` | `patches_dirs` | Category by Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT's category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default value. Please specify `patches_dirs`** | Vaporizer2 category names (e.g., `Pad` / `Bass` / `Arpeggio`) |

Names are matched ignoring case, spaces, and underscores (e.g., `Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to specify `[plugins.<name>]` if the plugin is installed in a non-standard location, if you need to augment patch directories, or if you are using a plugin not built-in. Only the specified items will override the built-in values, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain at built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all fields.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Patch directories for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<purpose>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will apply to that specific plugin. If omitted, the plugin's default value (Surge XT uses category names, others use 'no filtering') will be used. |

- `active_plugin` is deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 purpose-specific category items at the top level will result in a configuration error, not silent ignore. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only profiles with the same name as Surge XT will override the fixed default value; others become candidates in the mixed catalog.
- Since a Dexed patch is "one `.syx` cartridge = 32 programs", the list displays each program individually, treating the cartridge as a directory (e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.`, with 0-indexed two-digit numbers). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord rows.
- A Vaporizer2 patch is "one `.vvp` file = 1 patch", and they can be selected just like Surge's `.fxp` files. The category shown in the list's heading is derived from **the first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because preset locations are environment-dependent values determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing to them automatically could corrupt your DAW environment. Please add a line as follows. Until specified, it will not appear in the catalog with 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting differs per patch and is read from the contents of the `.vvp` file (`m_uPolyMode`). Therefore, only patches that can play chords will appear as candidates in the grid sequencer's chord rows (patches that cannot be read will not be shown as candidates for chord rows).
- Among Vaporizer2's factory presets, those with "MPE" in their name will not produce sound in `cmrt`. These patches are designed for MPE (per-note pitch and pressure) performance data, which `cmrt` does not send.
- The default category settings for filtering candidates by row purpose (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for all rows). This is because Dexed cartridges do not follow a "directory name = purpose" convention, and for non-built-in plugins, the patch directory structure is unknown, so no filtering is applied. If you want to change this, please specify the 7 items under `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 purpose-specific category items should also only be written within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them within their respective plugin table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic purpose-based selection is only applied to Surge XT patch determination.
- Rendering results are cached in separate directories for each plugin, so mixing them will not lead to accidental use of sounds from different plugins (manual deletion is not necessary). The two cache locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (on Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"` is set, the TUI will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAV data. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of communication errors, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension
  - When an MML entry is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play do-re-mi.

```
cmrt CM7
```

- Typing `CM7` will play C major seventh.
- Also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- Displays the number of patch candidates available for selection in the PATCH column's wheel for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Use this to check if the wheel has become unresponsive after changing plugins, `patches_dirs`, or purpose-specific categories (e.g., `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with exit code 1.
- Adding `--config <path>` will load that `config.toml`. This allows you to test how changes to settings would affect the application without modifying your current `config.toml`.
- When multiple plugin patches are listed, the breakdown by plugin will also be shown alongside the number of candidates per purpose. This is because simply showing the total count might not reveal that "a certain plugin has no patches available for that row".

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and an audio output digest value on a single line. No screen is launched.
- While `patch-roles` counts whether a patch appears in the list, this command checks whether that patch actually produces sound.
- `--patch` can be specified multiple times. The summary line will show "N / M distinct outputs", allowing you to verify if changing the patch had no effect (i.e., it's still playing the previous sound).
- Adding `--out-dir <directory>` writes the WAV file (if not specified, no bytes are written). Use this when you want to verify by ear.
- Adding `--poly-check` will compare playing chords and single notes to determine if the patch can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes occur daily.

# Future Plans
- It is more appropriate to obtain Surge XT patches via API, so this will be implemented (currently, patches specified in `toml` are searched, which is inefficient. Implementation timing is deferred; other priorities are being addressed).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while accepting certain constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note: "atomic measure" tends to imply a term from physics, so for now, I will keep "Atomic Measure" without directly translating it.

# Out of Scope
- Effects are essential for editing, so they are explicitly deemed out of scope and postponed to a much later stage. One reason for this is that Surge XT's patches inherently include effects (effects are derived from patches).