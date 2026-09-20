# clap-mml-render-tui

### Overview
An MML TUI DAW (or similar). Easily enjoy rich sounds from Surge XT / Dexed / Vaporizer2 / Floe / Sforzando using MML. Written in Rust.

### Usage

- For playing around and having fun with MML sounds
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

You can enter MML and play around on the TUI screen.

#### Play Server Details

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, using the first one found:

1.  The full path specified by `--play-server <PATH>` (if the specified path does not exist, it will stop with an error without searching further).
2.  `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3.  The release build of a sibling repository (`../clap-mml-play-server/target/release/`).

PATH is not checked. Debug build servers are 4-5 times slower at pre-loading, causing playback to cut off at the beginning of measures. A warning will appear in the top-right corner of the screen if a debug build or an unknown executable is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- ※Limited to CLAP plugins available for Windows, free of charge, and without account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando
- Effects (can be inserted in series into DAW tracks)
  - TONE3000
  - Surge XT Effects

### AI-Generated Documentation
- Sections added by AI might be hard to read. These will be maintained periodically.

### Keyboard Screen

Press the `v` key to navigate to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to overview and edit the "structure" of a song's chord progression. Moving the cursor will play the chord progression on that line. Moving through chords within a line with `h` or `l` will play only the pointed chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- Correcting a section's progression will simultaneously change all its references throughout the song.
- The header's first line directly displays the `chord2mml` specification for the song's beginning (e.g., `Key=C BPM120`).
- Neither the progression nor the header are interpreted by this screen; they are held as raw strings.
- Only the section on the cursor line will play (not the entire song). You can select the timbre for the entire Chord Chart from the progression editing screen.
- You can audition chords one by one within a line using `h` and `l`. The currently pointed chord is highlighted in the progression.
- Only the Key from the header is passed to the audition (BPM remains at default).
- Audition inversions and octaves are always auto-voiced. These are determined by the connection within the section for "Sections" and by the overall song order for "Arrangement"; the same inversion is maintained even during single chord audition with `h` and `l`.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:quit ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

Here are the key bindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on the screen when you press `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Move pane (toggle between Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (line. The chord progression of the new line will play entirely) |
| `h` `l` `←` `→` | Common | Move chord within line (Only the pointed chord will play. Moves to the adjacent line at the end of a line) |
| `PgUp` `PgDn` | Common | Move cursor 10 lines |
| `dd` | Common | Delete cursor line (Deleting in Sections also deletes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor line up / down |
| `b` | Common | Rewrite header's Key / BPM via single-line input |
| `Shift+P` `Space` | Common | Audition the section on the cursor line (stops if already playing) |
| `?` | Common | Toggle help (also closes with `Esc`) |
| `q` | Common | Quit application |
| `g` | Sections | Add a section by lottery from the chord progression catalog |
| `r` | Sections | Reroll the progression of the cursor line, keeping its name |
| `i` | Sections | Edit progression with a single-line MML overlay for Chord Chart |
| `n` | Sections | Edit name via single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end. While typing, the chord at the cursor position will auto-voice and play as it becomes valid or changes. `Ctrl+Space` can play the entire progression with the same voicing. Unreadable input will not be played back as MML, but can still be saved as is. `Enter` confirms and saves the progression, removing leading/trailing spaces, while `Esc` discards changes.

Inside the editing screen, `Ctrl+T` opens the timbre list, similar to a regular MML overlay. The timbre doesn't change by simply navigating candidates; `Enter` confirms the selection, and `Esc` reverts to the original timbre. The Chord Chart's timbre is global for the entire screen and is saved to `history.json` separately from the regular `Ctrl+P` MML overlay timbre. Even if you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

Changes are automatically saved. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, it's `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If the save file is missing or unreadable, the screen will perform a single lottery draw (same as `g`) when first opened, starting with one section (or remaining empty if the draw fails).

The chord progression catalog used for `g` / `r` lottery is fetched from the network. Only the first time, when the cache is empty, will there be a wait (the wait time is recorded in `log.txt` as `chord-chart: event=catalog-first-load elapsed_ms=...`). If the data cannot be fetched, "Chord progression data not available" will appear at the bottom of the screen.

### DAW Screen Effect Chain

In NORMAL mode on the DAW screen, placing the cursor on a performance track and pressing `x` opens the EFFECT CHAIN overlay for that track. You can insert any number of TONE3000 / Surge XT Effects factory presets in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Opens the EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor lines) |
| `j` `k` | Move through the effect chain stages |
| `a` | Opens an "Add" overlay. All factory presets from all effects are listed in a single column. Select one with `j` `k`, press `Enter` to add it to the end, `ESC` to go back. |
| `dd` | Delete the cursor stage |
| `Enter` | Writes back to the JSON in the 'init' column and closes (the track's cache WAV is re-rendered) |
| `ESC` | Discard changes and close |

- The chain is saved in the `"effects after instrument"` JSON array in the `init` column (array order = signal order). Direct editing of the `init` column works the same way.
- Effects are baked into the cache WAV. This only works when `offline_render_backend` is `in_process` (cells with effects will result in a rendering error with `render_server`).
- Each effect's preset is read from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). If a plugin is not available, it won't appear as a candidate.
- The chain runs for the duration of the notes, so reverb tails are cut off at the end of the cell.

### Configuration

A `config.toml` file is automatically created on first launch. Its location is in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

You can open `config.toml` with an editor by pressing `e` in NORMAL mode in TUI / DAW. The application will restart after closing the editor.

Here is an example of the current configuration:

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline renders for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing /render to a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 42154

# Whether to auto-play on startup
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only write this section if you want to change Surge XT's default values
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
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for `in_process`. |
| `offline_render_backend` | `in_process` | Destination for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent workers for `render_server`. |
| `offline_render_server_port` | `42153` | `render_server`'s localhost port. |
| `offline_render_server_command` | Empty string | Command to launch `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback execution. |
| `realtime_play_server_port` | `42154` | `play_server`'s localhost port. |
| `autoplay_on_startup` | `true` | Whether to auto-play immediately after startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directories | List of directories to search for Surge XT timbres. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Keys for the category overlay are determined from unused English letters within the category names. |

OS-specific default `plugin_path` values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default `patches_dirs` values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin that plays lines where no timbre is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog from `[plugins.<Name>]` and used for lines where the timbre is explicitly stated.

The built-in profiles are as follows, with paths set to the standard installation location for each OS:

| Name | `plugin_id` | `patches_dirs` | Category for specific use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific defaults from the table above | Surge XT's category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed's cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2's category names (e.g., `Pad` / `Bass` / `Arpeggio`) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You should only write `[plugins.<Name>]` if you have installed a plugin in a non-standard location, need to supplement timbre locations, or want to use a plugin not included in the built-in profiles. **Only the specified items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Replace only the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | The path to that plugin. |
| `plugins.<Name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | The timbre location for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Use>_patch_categories` / `<Role>_patch_keywords` | Filtering for automatic patch selection by use. Seven key names can be specified (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If not specified, the plugin's default value will be used (Surge XT uses Surge's category names; others use "no filtering"). |

- `active_plugin` has been deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 use-specific categories at the top level will result in a configuration error rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default; others become candidates in the mixed catalog.
- For Dexed, one cartridge `.syx` file contains 32 programs. In the list, cartridges are treated as directories, displaying one program at a time, e.g., `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits). If `patches_dirs` specifies the cartridge location, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a timbre property, and its default is POLY. Therefore, in the grid sequencer's chord lines, all Dexed timbres are treated as suitable for chords.
- For Vaporizer2, one `.vvp` file equals one timbre, selectable like Surge's `.fxp` files. The category displayed in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs`. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and cmrt would risk corrupting your DAW environment by automatically reading/writing it. Please add one line as shown below. Until you do, it will not appear in the catalog as having 0 timbres.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting varies per timbre and is read from the `.vvp` file's content (`m_uPolyMode`). Therefore, in the grid sequencer's chord lines, only timbres that play chords will appear as candidates (timbres that could not be read are not offered as candidates for chord lines).
- Among Vaporizer2's factory presets, those with "MPE" in their name will not produce sound in cmrt. These timbres are designed for MPE (note-per-note pitch and pressure) performance data, which cmrt does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / 4 drum roles / other) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for all lines). This is because Dexed cartridges do not follow a "directory name = use" structure, and the timbre organization for non-built-in plugins is unknown, so filtering is not applied. If you wish to change this, specify the 7 items in `[plugins.<Name>]` (Surge XT's default values are included as comments at the end of the generated `config.toml`).
- The 7 use-specific category items should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in their respective plugin table.
- Shared judgment data for mono/poly determination used in automatic use-specific selection (`voicing_shared_source` / `voicing_override_source`) is only used for Surge XT's timbre determination.
- The rendering cache is stored in separate directories for each plugin, preventing misuses of sounds from different plugins even when mixed (no manual deletion is needed). The locations are as follows, where `<PLUGIN>` is the filename of the resolved `plugin_path` without extension (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<PLUGIN>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<PLUGIN>\*.wav` (DAW track WAVs)

If `offline_render_backend = "render_server"`, the TUI side will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAVs. If the connection to the render-server fails, cmrt will launch a child process and, in case of communication errors, will restart once and retry.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
  - When an MML is found in a Bluesky post, it can be played using Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C D E.

```
cmrt CM7
```

- Typing `CM7` will play C Major Seventh.
- It supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- Displays how many timbre candidates are available for each line (chord / bass / arpeggio / 4 drum roles / other) in the grid sequencer's PATCH wheel. The screen does not launch.
- Use this after changing plugins, `patches_dirs`, or use-specific categories (`chord_patch_categories`, etc.), to check if the wheel is unresponsive.
- If any line has 0 candidates, it will list that line and exit with error code 1.
- Adding `--config <PATH>` will read from that `config.toml`. This allows you to test how configuration changes affect things without modifying your active `config.toml`.
- When timbres from multiple plugins are listed, the output also includes a breakdown by plugin for the candidate count per use. This is to help notice if a particular plugin has no timbres available for a given line, which might not be obvious from the total count alone.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays length, volume (`peak` / `rms`), whether it's silent, and an audio digest value on a single line. The screen does not launch.
- While `patch-roles` counts whether a timbre appears in the list, this command checks if that timbre actually produces sound.
- You can specify multiple `--patch` arguments. The summary line will show "different sounds N / M", allowing you to check if the sound remains the same despite changing the timbre.
- Adding `--out-dir <DIRECTORY>` will write WAV files (otherwise, no bytes are written). Use this when you want to audition the sound.
- Adding `--poly-check` compares playing a chord and a single note to determine if the timbre plays chords.
- `--config <PATH>` works the same as with `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is more logical to obtain Surge XT patches via API, so this will be implemented (currently, they are inefficiently searched via toml specification. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- アトミック小節 (Atomic Measure)
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "one-measure offline rendering,"
    - While imposing constraints,
    - It offers various benefits.
    - This approach is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - ※ Note: As "atomic measure" is a term in physics, for now, the Japanese term "アトミック小節" will be retained without direct English translation.

# Out of Scope
- Effects, requiring mandatory editing, are deliberately treated as out of scope and deferred to a much later stage. One reason for this is that in Surge XT, patches inherently include effects (effects are derived from patches).