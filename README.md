# clap-mml-render-tui

### Overview
MML TUI DAW (or something similar). Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Purpose

- For playing around with MML sounds
- For casual installation. Just having Rust is enough.

### Tech Stack
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

### Usage

```
cmrt
```

You can play around by entering MML in the TUI screen.

#### Play Server Implementation

The sound is played by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1.  The full path specified by `--play-server <PATH>` (If the specified path doesn't exist, it stops with an error without searching further).
2.  `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3.  The release build from the sibling repository (`../clap-mml-play-server/target/release/`).

It does not look at the system `PATH`. A debug build server has 4-5 times slower pre-buffering, which may cause playback to cut off at the beginning of measures. A warning appears in the upper right corner of the screen when a debug build or an executable of unknown origin is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- * Limited to CLAP, Windows, and freely available plugins without account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI Generated Documentation
- The following sections, appended by AI, may be difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C, D, E, F, G, A, B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to view and edit the 'structure' of a song's chord progression. Moving the cursor plays the chord progression of that row. Traversing the chords within a row with `h` and `l` plays only the selected chord.

- Left pane (Sections): Define named chord progressions that serve as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- If you modify a section's progression, all its references in the song will be updated simultaneously.
- The header line directly displays the chord2mml specification (`Key=C BPM120`, etc.) placed at the beginning of the song.
- Neither progressions nor headers are interpreted by this screen; it retains the strings as entered.
- Only the section on the cursor's row plays (the entire song does not play). You can select the overall timbre for the Chord Chart from the progression editing screen.
- You can preview chords one by one within a row using `h` `l`. The currently selected chord appears inverted within the progression.
- Only the Key from the header is passed for preview (BPM remains at its default).
- Inversions and octaves for preview are always auto-voiced. In Sections, this is determined within the section; in Arrangement, it's determined by the overall song sequence. Single-chord previews with `h` `l` also maintain the same inversion.

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

Here are the keybindings. Only `q` and `?` are shown on the bottom line of the screen. The full list appears on-screen when pressing the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Toggle pane (Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. The chord progression of the new row plays entirely) |
| `h` `l` `←` `→` | Common | Move chord within row (only the selected chord plays. Moves to an adjacent row at the end of a line) |
| `PgUp` `PgDn` | Common | Move cursor 10 rows |
| `dd` | Common | Delete cursor row (deleting in Sections also deletes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down |
| `b` | Common | Overwrite Header Key / BPM with single-line input |
| `Shift+P` `Space` | Common | Preview cursor row's section (stops if already playing) |
| `?` | Common | Open/close help (also closes with `Esc`) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Re-draw the progression of the cursor row, retaining its name |
| `i` | Sections | Edit progression using a single-line MML overlay for the Chord Chart |
| `n` | Sections | Edit name with single-line input |
| `1`〜`9` | Arrangement | Insert the section corresponding to that number after the cursor |

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end. During input, the chord at the cursor position will play with auto-voicing as it is formed or changed. Press `Ctrl+Space` to play the entire progression with the same voicing. Unreadable input will not be played as alternative MML but can still be saved as is. Press `Enter` to confirm and save the progression, trimming leading/trailing spaces. Press `Esc` to discard changes.

Inside the editing screen, `Ctrl+T` opens the same timbre list as the normal MML overlay. The timbre does not change by merely navigating candidates; press `Enter` to confirm, or `Esc` to revert to the original timbre. The Chord Chart timbre is global to the screen and saved to `history.json` separately from the normal `Ctrl+P` MML overlay timbre. Even if you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

It is automatically saved every time you edit. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, it's `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If no save file exists or it's unreadable, when the screen is first opened, a single draw (same as `g`) is performed to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. You will only experience a delay on the first launch when the cache is not yet available (the wait time is recorded in `log.txt` under `chord-chart: event=catalog-first-load elapsed_ms=...`). If it cannot be fetched, "No chord progression data" appears at the bottom of the screen.

### Configuration

On first launch, `config.toml` is automatically created. It is located in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of TUI / DAW, pressing `e` opens `config.toml` with an editor. After closing the editor, restart the application.

Here's an example of the current configuration.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates for opening config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ under the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent DAW offline rendering tasks (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing to /render on a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 42154

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

Configuration items are as follows:

| Item | Default | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates tried in order from left to right. |
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in_process rendering tasks. |
| `offline_render_backend` | `in_process` | Target for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server tasks. |
| `offline_render_server_port` | `42153` | Localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback. |
| `realtime_play_server_port` | `42154` | Localhost port for play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for Surge XT patch selection. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after making changes. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop dirs. Category overlay keys are determined from unused English letters within the category names. |

The default `plugin_path` values by OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values by OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines where no timbre is specified is **Surge XT fixed**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog via `[plugins.<name>]` and used on lines where the timbre is explicitly stated.

The contents of the built-in profiles are as follows, with paths set to the standard installation locations per OS.

| Name | plugin_id | patches_dirs | Categories by use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default value. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You should only write `[plugins.<name>]` if you install plugins in non-standard locations, need to supplement patch directories, or use plugins not built-in. Since **only the explicitly written items override built-in values**, if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, define everything.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The patch directory for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by use. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If not specified, the plugin's default values will be used (Surge XT uses category names, others use "no filtering"). |

- `active_plugin` is deprecated. Also, if `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items are written at the top level, it will result in a configuration error instead of silently ignoring them. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT overrides the fixed default value; others become candidates in the mixed catalog.
- Dexed's timbres are '1 `.syx` cartridge = 32 programs', so in the list, each program is displayed like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbering is 0-indexed, 2 digits), treating the cartridge as a directory. If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not part of the timbre, and its default value is POLY. Therefore, all Dexed timbres are treated as chord-suitable in the grid sequencer's chord rows.
- Vaporizer2's timbres are '1 `.vvp` file = 1 timbre', and can be selected just like Surge's `.fxp` files. The category appearing in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and cmrt arbitrarily reading/writing to it could corrupt your DAW environment. Please add a single line as shown below. Until you do, it will not appear in the catalog as having 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting varies per timbre and is read from the `.vvp` file's content (`m_uPolyMode`). Therefore, only timbres that can play chords appear as candidates in the grid sequencer's chord rows (unreadable timbres are not listed as candidates for chord rows).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These are timbres that rely on MPE (per-note pitch and pressure) performance information, which cmrt does not transmit.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / 4 drum roles / others) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for all lines). This is because Dexed cartridges do not follow a 'directory name = usage' convention, and for non-built-in plugins, the patch directory structure is unknown, so no filtering is applied. If you want to change this, please add the 7 items to `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also be written only within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The mono/poly shared judgment data (`voicing_shared_source` / `voicing_override_source`) used for usage-based auto-selection is only applied to Surge XT timbre determination.
- Rendering results are cached in separate directories per plugin, so mixing them will not lead to accidental use of sounds from other plugins (no manual deletion is required). The two locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"` is set, the TUI side will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAV data. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, restart and retry once.

### update command

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

- Typing `cde` plays C-D-E (do-re-mi).

```
cmrt CM7
```

- Typing `CM7` plays a C Major 7th chord.
- Also supports various chord progression notations (some are not yet supported).

### patch-roles command

```
cmrt patch-roles
```

- Displays the number of available patch candidates that can be selected with the wheel in the PATCH column for each line of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The screen does not launch.
- Use this to check if the wheel has become unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (e.g., `chord_patch_categories`).
- If any line has 0 candidates, it will list that line and exit with exit code 1.
- If `--config <path>` is specified, it reads that `config.toml`. This allows you to test how changes in settings affect things without modifying your current `config.toml`.
- When multiple plugins' patches are listed, the breakdown by plugin will also be shown for the number of candidates per usage. This is because summing them up alone might not reveal that 'a specific plugin has no patches available for that line'.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays its length, volume (`peak` / `rms`), whether it's silent, and an output digest value on a single line. The screen does not launch.
- While `patch-roles` counts whether a patch appears in a list, this command checks whether that patch actually produces sound.
- `--patch` can be specified multiple times. The summary line shows 'N / M distinct outputs', allowing you to verify if the **sound remained the same despite changing the patch**.
- If `--out-dir <directory>` is specified, it writes WAV files (otherwise, no bytes are written). Use this when you want to verify by ear.
- If `--poly-check` is specified, it compares playing chords and single notes to determine if the patch can play polyphonically.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes more sense to retrieve Surge XT patches via API, so that will be implemented (currently, they are inefficiently searched from TOML-specified paths. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's Atomic Notes.
    - By making the unit of all processing 'offline rendering in 1-measure units',
    - while accepting certain constraints,
    - various benefits can be gained.
    - This approach is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - * 'Atomic measure' might sound like a physics term, so for now, I'll keep it as 'Atomic Measure' (アトミック小節) with a direct English translation here, noting its inspiration from "atomic notes."

# Out of Scope
- Effects require editing, so they are explicitly deemed out of scope and postponed significantly. One reason is that in Surge XT, patches already encapsulate effects (effects are derived from patches).