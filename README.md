# clap-mml-render-tui

### Overview
An MML TUI DAW (Digital Audio Workstation) - or something similar. Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Just having Rust is enough.

### Technology Stack
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

#### Play Server Implementation

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (If the specified path does not exist, it will stop with an error without searching further).
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3. The release build in the sibling repository (`../clap-mml-play-server/target/release/`).

The PATH environment variable is not checked. A debug build server is 4-5 times slower at pre-loading, causing playback to cut off at the beginning of measures. A warning will appear in the upper right corner of the screen if you are using a debug build or an executable of unknown origin.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- ※ Limited to those available for free without account registration, on CLAP, Windows.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando
- Effects (can be inserted in series on a DAW track)
  - TONE3000
  - Surge XT Effects
  - Dragonfly Reverb (Hall / Room / Plate / Early Reflections)

### AI Generated Documentation
- From here on, parts added by AI may be difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press `v` to navigate to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to overview and edit the "composition" of a song's chord progression. Moving the cursor will play the chord progression of that row. Press `h` and `l` to step through chords within a row, playing only the selected chord.

- Left pane (Sections): Define named chord progressions that serve as building blocks.
- Right pane (Arrangement): Arrange the defined sections to form a song. The same section can be used multiple times.
- If you modify a section's progression, all references to it in the song will change simultaneously.
- The first line of the header displays the `chord2mml` specification (e.g., `Key=C BPM120`) to be placed at the beginning of the song, as is.
- Neither the progression nor the header is interpreted by this screen. It simply stores the entered string.
- Only the section on the cursor row is played (not the entire song). You can select the overall timbre for the Chord Chart from the progression editing screen.
- Press `h` and `l` to audition chords one by one within a row. The currently selected chord appears highlighted within the progression.
- Only the Key from the header is used for auditioning (BPM remains at its default).
- Voicing and octave for auditioning are always auto-voiced. In Sections, this is determined within the section; in Arrangement, it's based on the overall song sequence. Single-chord auditioning with `h` `l` also maintains the same voicing.

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

Here are the keybindings. Only `q` and `?` are shown on the bottom line of the screen. The full list appears on-screen by pressing `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Pane movement (toggles between Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement (row. The chord progression of the newly selected row will play). |
| `h` `l` `←` `→` | Common | Chord movement within a row (plays only the selected chord. Wraps to the adjacent row at the ends of a line). |
| `PgUp` `PgDn` | Common | Move cursor by 10 rows. |
| `dd` | Common | Delete cursor row (Deleting in Sections also deletes its references in Arrangement). |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down. |
| `b` | Common | Overwrite Key / BPM in header with single-line input. |
| `Shift+P` `Space` | Common | Audition the section on the cursor row (stops if already playing). |
| `?` | Common | Toggle help (closes with `Esc` as well). |
| `q` | Common | Exit application. |
| `g` | Sections | Add a section by drawing from the chord progression catalog. |
| `r` | Sections | Redraw the progression of the cursor row, keeping its name. |
| `i` | Sections | Edit progression using a single-line MML overlay for the Chord Chart. |
| `n` | Sections | Edit name with single-line input. |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor.

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end. While typing, the chord at the cursor position will play with auto-voicing as it forms or changes, and `Ctrl+Space` will play the entire progression with the same voicing. Unreadable input will not be audibly replaced as MML but can still be saved. Press `Enter` to confirm and save the progression, trimming leading/trailing spaces. `Esc` discards changes.

Inside the editing screen, `Ctrl+T` opens the timbre list, similar to the regular MML overlay. Moving through candidates does not change the timbre; press `Enter` to confirm, or `Esc` to revert to the original timbre. The Chord Chart timbre is global to the screen and saved to `history.json` separately from the regular `Ctrl+P` MML overlay timbre. If you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

Changes are autosaved every time. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (e.g., `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json` on Windows). Only one song is saved at a time. If the save file is missing or unreadable, the screen will perform a single draw, identical to `g`, when first opened, starting with one section (it remains empty if no section can be drawn).

The chord progression catalog used for `g` / `r` draws is fetched from the network. There will be a wait only for the first time if the cache is not yet built (wait time is recorded in log.txt as `chord-chart: event=catalog-first-load elapsed_ms=...`). If the data cannot be fetched, "コード進行データがありません" (Chord progression data is not available) will appear at the bottom of the screen.

### Effect Chain on DAW Screen

In NORMAL mode on the DAW screen, place the cursor on a performance track and press `x` to open that track's EFFECT CHAIN overlay. You can insert factory presets of TONE3000 / Surge XT Effects / Dragonfly Reverb in series, any number of stages, after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Open the EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor rows). |
| `j` `k` | Move through chain stages. |
| `a` | Open the add overlay. All factory presets for all effects are listed in a single column; use `j` `k` to select, `Enter` to append to the end, `ESC` to go back. |
| `dd` | Delete the cursor stage. |
| `Enter` | Write back to the init column's JSON and close (the track's cached WAV will be re-rendered). |
| `ESC` | Discard changes and close.

- The chain is saved in the init column's JSON under `"effects after instrument"` (array order = signal order). Editing the init column directly achieves the same result.
- Effects are baked into the cached WAV (applied on the render-server side).
- Each effect's presets are loaded from their built-in default locations (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb's presets are built into the plugin itself, so they will appear as candidates if the plugin is present at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`. If the plugin is not found, no candidates will appear.
- The chain runs for the duration of the notes, so reverb tails will be cut off at the end of the cell.

### Configuration

`config.toml` is automatically created on first launch. It is located in the OS's standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW's NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is a current configuration example.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by the render-server child process.
# Concurrency (1-16), port, launch command (if empty, the executable is searched for)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only write this if you want to change Surge XT's default values
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
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline rendering (render-server) processes. |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Command to launch the render-server. If empty, the executable is searched for. |
| `realtime_audio_backend` | `cache_player` | Target for real-time playback (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard directories for Surge XT patches | List of directories to search for Surge XT timbres. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused English letters within the category names. |

OS-specific default values for `plugin_path` are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default values for `patches_dirs` are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share` is used).
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified timbre is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog from `[plugins.<name>]` and used on lines where the timbre is explicitly stated.

The built-in profiles are as follows, with paths set to the standard installation location for each OS.

| Name | plugin_id | patches_dirs | Category for Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific defaults from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you've installed a plugin in a non-standard location, need to specify a timbre location, or are using a plugin not built-in. **Only the specified items will override the built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For a non-built-in plugin, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | The path to that plugin. |
| `plugins.<name>.plugin_id` | The expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | The timbre location for that plugin. To remove built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filters for automatic patch selection by usage. Seven key names can be used (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will apply to that plugin. If not specified, the plugin's default value (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific categories at the top level will result in a configuration error, rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default, while others become candidates in the mixed catalog.
- Dexed timbres are structured as '1 `.syx` cartridge = 32 programs.' Thus, in the list, cartridges are treated like directories, displaying each program individually, such as `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). By specifying the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a timbre property, with a default of POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 `.vvp` file = 1 timbre, selectable just like Surge's `.fxp` files. The category shown in the list header is determined by **the first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 is the only plugin that does not have a default `patches_dirs` value. This is because its preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and cmrt arbitrarily reading or writing to it could corrupt your DAW environment. Please add a single line as shown below. Until you do, it will not appear in the catalog with 0 timbres.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly varies per timbre, read from the `.vvp` file's content (`m_uPolyMode`). Therefore, only timbres that play chords will appear as candidates in the grid sequencer's chord rows (unreadable timbres will not be listed as candidates for chord rows).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These timbres are designed with MPE (per-note pitch and pressure) performance information in mind, which cmrt does not transmit.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for all rows). This is because Dexed cartridges do not follow a 'directory name = usage' convention, and the timbre storage system for non-built-in plugins is unknown, so they are not filtered. If you wish to change this, please add the 7 items to `[plugins.<name>]` (the default Surge XT values are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific categories should also be written only within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in their respective plugin tables.
- The mono/poly shared determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-based auto-selection is only applied to Surge XT timbre determination.
- Rendering results are cached in separate directories per plugin, so mixing them will not lead to accidental use of sounds from different plugins (no manual deletion is required). The two cache locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins; instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, restart and retry once. If `offline_render_server_command` is empty, the child process executable is searched for in the following order: 'same directory as `cmrt.exe`' then 'release build of sibling repo `clap-mml-play-server`'. **The PATH environment variable is not checked**.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Connects with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing 'cde' plays C D E.

```
cmrt CM7
```

- Typing 'CM7' plays C Major Seventh.
- Also supports various chord progression notations (some are not yet supported).

### Patch Roles Command

```
cmrt patch-roles
```

- Displays the number of timbre candidates selectable with the wheel in the PATCH column for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Use this to check if the wheel has become 'unresponsive' after changing plugins, `patches_dirs`, or usage-specific categories (e.g., `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with status code 1.
- Adding `--config <path>` makes it read that `config.toml`. This allows you to test how changes to settings would behave without modifying your current `config.toml`.
- When multiple plugin timbres are listed, the breakdown by plugin will also be shown for the number of candidates per usage. This is because simply showing the total count might prevent you from noticing if 'a certain plugin has no timbres appearing for that row'.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and a sound digest value on a single line. No screen is launched.
- While `patch-roles` counts whether a timbre appears in the list, this command checks whether that timbre actually produces sound.
- Multiple `--patch` arguments can be supplied. A summary line will show 'N / M distinct sounds,' allowing you to check if **the timbre remained the same despite being changed**.
- Adding `--out-dir <directory>` writes out WAV files (if omitted, no bytes are written). Use this when you want to verify by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Breaking changes are made frequently, on a daily basis.

# Future Plans
- Acquiring Surge XT patches via API is the proper approach, so that will be implemented (currently, they are inefficiently searched from TOML specifications. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measures
    - Inspired by Obsidian's Atomic Notes.
    - By making the unit of all processing 'offline rendering in 1-measure units,'
    - While accepting certain constraints,
    - Various benefits can be gained.
    - This approach is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - Note: While "atomic measure" might evoke physics, the concept here is inspired by Obsidian's Atomic Notes.

# Out of Scope
- Effects are essential for editing, but for now, they are explicitly considered out of scope and pushed far back in priority. One reason for this is that Surge XT's patches inherently include effects (effects are extracted from patches).