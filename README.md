# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Rust is all you need.

### Tech Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Setup

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

You can enter MML and play around in the TUI screen.

#### Play Server Implementation

The sound is played by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1.  The full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further).
2.  `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3.  The release build of the sibling repository (`../clap-mml-play-server/target/release/`).

It does not check PATH. A debug-built server is 4-5 times slower at preloading, causing playback to cut off at the beginning of measures.
A warning will appear in the top right of the screen if you are using a debug build or an executable of unknown origin.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP, Windows, and free plugins available without account registration.*
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- Effects (can be inserted in series into DAW mode tracks)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
- Direct download links (for those who get lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install a DAW and audio plugins (even for cats, up to playing sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (studiorack-site introduction page)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (studiorack-site introduction page)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)

### AI-Generated Documentation
- From here on, the AI-appended sections might be difficult to read. I'll maintain them occasionally.

### Keyboard Screen

Press `v` to move to the keyboard screen.

- `c d e f g a b` keys: Play Do-Re-Mi-Fa-Sol-La-Si.

### Chord Chart Screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to overview and edit the "composition" of a song's chord progression. Moving the cursor will play the chord progression on that row.
Pressing `h` or `l` to step through chords within a row will play only the highlighted chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): Arrange the defined sections to form a song. The same section can be used multiple times.
- Modifying a section's progression will update all its references throughout the song.
- The header row directly displays the chord2mml specification (`Key=C BPM120` etc.) to be placed at the beginning of the song.
- Neither the progression nor the header is interpreted by this screen. It simply stores the typed string as is.
- Only the section on the cursor's row will play (not the entire song). You can select the overall timbre for the Chord Chart from the progression editing screen.
- You can audition individual chords within a row using `h` and `l`. The currently highlighted chord appears in reverse in the progression.
- Only the `Key` in the header is passed for auditioning (BPM remains at the default).
- Auditioning inversions and octaves are always auto-voiced. In Sections, they are determined within the section, and in Arrangement, by the overall song sequence, maintaining the same inversion even during single-chord auditions with `h` and `l`.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q: quit ?: help                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

Here are the keybindings. Only `q` and `?` are shown in the bottom line of the screen. The full list appears on screen when you press `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Toggle pane (Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. The chord progression of the new row plays entirely) |
| `h` `l` `←` `→` | Common | Move chord within a row (Plays only the highlighted chord. Moves to the adjacent row at the end of a row) |
| `PgUp` `PgDn` | Common | Move cursor 10 rows |
| `dd` | Common | Delete cursor row (Deleting in Sections also deletes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down |
| `b` | Common | Edit header's Key / BPM via single-line input |
| `Shift+P` `Space` | Common | Audition the section on the cursor row (stops if already playing) |
| `?` | Common | Toggle help (closes with `Esc` as well) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Redraw the progression of the cursor row, preserving its name |
| `i` | Sections | Edit the progression using a single-line MML overlay for Chord Chart |
| `n` | Sections | Edit the name via single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end.
While typing, the chord at the cursor position will play with auto voicing whenever it's formed or changed. `Ctrl+Space` plays the entire progression with the same voicing. Unreadable input is not played back as MML but can be saved as is.
`Enter` confirms and saves the progression, stripping leading/trailing spaces. `Esc` discards changes.

Inside the editing screen, `Ctrl+T` opens the same timbre list as the regular MML overlay. Moving through candidates does not change the timbre; `Enter` confirms, `Esc` reverts to the original timbre. The Chord Chart's timbre is global for the entire screen and saved separately in `history.json` from the regular `Ctrl+P` MML overlay's timbre. If you confirm a timbre and then `Esc` to discard progression edits, the confirmed timbre remains.

Changes are auto-saved. The save location is `clap-mml-render-tui/history/chord_chart.json` within the settings directory (e.g., `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json` on Windows). Only one song is saved at a time.
If the save file is missing or unreadable, the screen performs a one-time draw (like `g`) when first opened, starting with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. Only the first time, when the cache is empty, will there be a wait (wait time is recorded in log.txt under `chord-chart: event=catalog-first-load elapsed_ms=...`). If it fails to fetch, "Chord progression data not available" will appear at the bottom of the screen.

### DAW Screen Effect Chain

In the DAW screen's NORMAL mode, place the cursor on a performance track and press `x` to open the EFFECT CHAIN overlay for that track.
You can insert any number of factory presets from TONE3000 / Surge XT Effects / Dragonfly Reverb in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Open EFFECT CHAIN overlay for the cursor track (invalid for chord/conductor rows) |
| `j` `k` | Move through chain stages |
| `a` | Open add overlay. All factory presets of all effects are listed in one column. Use `j` `k` to select, `Enter` to add to the end, `ESC` to go back. |
| `dd` | Delete the cursor stage |
| `Enter` | Write back to the JSON in the `init` column and close (re-renders the track's cached WAV) |
| `ESC` | Discard changes and close |

- The chain is saved in the `"effects after instrument"` array (order = signal flow) in the `init` column's JSON. Editing the `init` column directly has the same effect.
- Effects are baked into the cached WAV (applied on the render-server side).
- Each effect's preset is read from its built-in default location (e.g., `%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb presets are built into the plugin itself, so if the plugin is found at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, its presets will appear as candidates. If the plugin is not found, no candidates appear.
- The chain runs for the duration of the notes, so reverb tails are cut off at the end of the cell.

### Configuration

A `config.toml` file is automatically created on first launch. Its location is within the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, press `e` to open `config.toml` in your editor. After closing the editor, restart the application.

Here's an example of the current configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the settings directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by the render-server child process.
# Concurrency (1-16), port, launch command (empty means search for executable)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: immediately plays the current line. DAW mode: starts playing from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for the WAV loop browser
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
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP default path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates tried in order from left. |
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline rendering (render-server) processes. |
| `offline_render_server_port` | `42153` | `localhost` port for the render-server. |
| `offline_render_server_command` | Empty string | Launch command for the render-server. If empty, it searches for the executable. |
| `realtime_audio_backend` | `cache_player` | Real-time playback target (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | `localhost` port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches default directories | List of directories to search for Surge XT patch selection. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. The key for the category overlay is determined from unused letters in the category name. |

OS-specific default values for `plugin_path` are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default values for `patches_dirs` are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is unset, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin that plays for lines not specifying a timbre is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog via `[plugins.<Name>]` and used on lines where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths set to the OS-specific standard installation locations:

| Name | `plugin_id` | `patches_dirs` | Usage-specific categories |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific defaults from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`.** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio` etc.) |

Names are matched ignoring case, spaces, and underscores (e.g., `Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<Name>]` if you have installed a plugin in a non-standard location, want to add timbre locations, or are using a plugin not built-in. **Only the specified items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For a plugin not built-in, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Location of patches for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filters for automatic patch selection by usage. Seven key names can be written: (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items take effect for that plugin. If not specified, the plugin's default (Surge XT uses its category names; others use "no filtering") is used. |

- `active_plugin` has been deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific categories at the top level will result in a configuration error, not just silent ignoring. Please remove `active_plugin` and move other values into `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default; others become candidates in the mixed catalog.
- Dexed's patches are "1 cartridge `.syx` file = 32 programs", so in the list, cartridges are treated as directories, and each program is listed like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, 2 digits). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance setting (`MonoMode`), not part of the patch, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord rows.
- Vaporizer2's patches are "1 `.vvp` file = 1 patch", selectable like Surge's `.fxp` files. The category shown in the list's heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 is unique in that it has no default for `patches_dirs`. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and cmrt arbitrarily reading/writing there could damage your DAW environment. Please add a line like this: Until then, it will not appear in the catalog with 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly varies per patch, read from the `.vvp` file's content (`m_uPolyMode`). Therefore, only patches capable of playing chords appear as candidates in the grid sequencer's chord rows (patches that cannot be read are not offered as candidates for chord rows).
- Among Vaporizer2's factory presets, those with "MPE" in their name will not produce sound in cmrt. These patches rely on MPE (per-note pitch/pressure) performance information, which cmrt does not send.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / 4 drum roles / others) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (i.e., all programs are candidates for all rows). This is because Dexed cartridges do not have "directory name = usage", and the system for non-built-in plugins' patch locations is unknown, so they are not filtered. If you wish to change this, add the 7 items to `[plugins.<Name>]` (the generated `config.toml` includes Surge XT's default values as comments at the end).
- The 7 usage-specific categories must also be written only within the plugin profile. For Surge XT, under `[plugins."Surge XT"]`; for other plugins, within their own table.
- Shared mono/poly determination data for usage-specific auto-selection (`voicing_shared_source` / `voicing_override_source`) is only used for Surge XT patch determination.
- Rendered results are cached in separate directories per plugin, preventing incorrect use of sounds from different plugins when mixed (no manual deletion needed). The locations are two: `<PLUGIN>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<PLUGIN>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<PLUGIN>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI (`cmrt.exe`) does not load CLAP plugins directly but sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, cmrt launches a child process, and upon communication error, it restarts and retries once. If `offline_render_server_command` is empty, the child process executable is searched for in the order of "same directory as `cmrt.exe` → release build of sibling repo `clap-mml-play-server`", and **PATH is not checked**.

### `update` command

```
cmrt update
```

### `server` mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension.
  - If a Bluesky post contains MML, it can be played with Surge XT.

### CLI mode

```
cmrt cde
```

- Typing `cde` plays Do-Re-Mi.

```
cmrt CM7
```

- Typing `CM7` plays C major seventh.
- It also supports various chord progression notations (some are not yet supported).

### `patch-roles` command

```
cmrt patch-roles
```

- Displays how many patch candidates are available for selection via the PATCH column's wheel for each grid sequencer row (chord / bass / arpeggio / 4 drum roles / others). Does not launch a UI.
- Use this after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.) to confirm that the "wheel is unresponsive" issue does not occur.
- If any row has 0 candidates, it lists those rows and exits with exit code 1.
- Adding `--config <path>` reads that `config.toml`. This allows testing how changes would affect without modifying the currently used `config.toml`.
- When patches from multiple plugins are listed, the output includes a breakdown by plugin for each usage-specific candidate count. This is because the total count alone might not reveal that "a specific plugin has no patches available for that row".

```
cmrt patch-roles --config C:\tmp\try.toml
```

### `render-mml` command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Performs offline rendering of MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound on a single line. Does not launch a UI.
- While `patch-roles` counts whether a patch appears in the list, this command checks if that patch actually produces sound.
- `--patch` can be specified multiple times. A summary line shows "N different outputs / M total" to indicate if a **patch change did not result in a different sound**.
- Adding `--out-dir <directory>` writes WAV files (does not write any bytes otherwise). Use this when you want to audit the sound by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the patch can play polyphonically.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Obtaining Surge XT patches via API is the correct approach, so that will be done (currently, they are inefficiently explored from `toml` specifications. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- アトミック小節
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per measure,"
    - it gains various benefits,
    - at the cost of being constrained.
    - This approach is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Note: "atomic measure" would refer to a physics term, so for now, I'm keeping it as "アトミック小節" without direct English translation.*

# Out of Scope
- Effects are considered out of scope and of very low priority because they require editing. One reason is that Surge XT's patches encapsulate effects (effects are extracted from patches).