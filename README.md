# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando easily with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Just having Rust is enough.

### Technical Stack
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

You can input MML in the TUI screen and play around.

#### Play Server Implementation

The sound is produced by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. Full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. Release build of the sibling repository (`../clap-mml-play-server/target/release/`)

The PATH environment variable is not used. Debug build servers are 4-5 times slower at pre-loading, causing playback to cut off at the beginning of measures. A warning appears in the top-right corner of the screen when using a debug build or an unknown executable.

```
cmrt --play-server "N:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- ※Limited to CLAP plugins that are free, available on Windows, and do not require account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI Generated Documentation
- The sections appended by AI below are difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press `v` to move to the keyboard screen.

- `c d e f g a b` keys: Play C, D, E, F, G, A, B (Do-Re-Mi-Fa-So-La-Si).

### Chord Chart Screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to view and edit the "structure" of a song's chord progression. Moving the cursor will play the chord progression of that row. Traversing the chords within a row with `h` `l` will play only the selected chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- If you correct a section's progression, all its references in the song will be updated together.
- The first line of the header directly displays the chord2mml specification (`Key=C BPM120`, etc.) placed at the beginning of the song.
- Neither the progression nor the header is interpreted by this screen. It holds the input string as is.
- Only the section on the cursor's row plays (not the entire song). The timbre is fixed and cannot be selected on this screen.
- You can preview chords one by one within a row using `h` `l`. The currently selected chord appears inverted within the progression.
- Only the Key in the header is passed to the preview (BPM remains at its default).

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

Here are the keybindings. Only `q` and `?` are shown in the bottom line of the screen. The full list appears on screen by pressing `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Pane movement (toggles between Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement (row. The entire chord progression of the destination row plays) |
| `h` `l` `←` `→` | Common | Chord movement within a row (only the selected chord plays. Wraps to the adjacent row at the end of a line) |
| `PgUp` `PgDn` | Common | Moves the cursor by 10 rows |
| `dd` | Common | Deletes the cursor row (deleting in Sections also removes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Moves the cursor row up / down |
| `b` | Common | Rewrites the header's Key / BPM via single-line input |
| `Shift+P` `Space` | Common | Previews the section on the cursor row (stops if already playing) |
| `?` | Common | Toggles help (closes with `Esc` as well) |
| `q` | Common | Exits the application |
| `g` | Sections | Adds a section by drawing from the chord progression catalog |
| `r` | Sections | Redraws the progression of the cursor row, retaining its name |
| `i` | Sections | Edits the progression via single-line input |
| `n` | Sections | Edits the name via single-line input |
| `1`〜`9` | Arrangement | Inserts the section with that number after the cursor |

Changes are autosaved. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, it's `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If the save file is missing or unreadable, the first time the screen is opened, it performs the same draw as `g` once to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. There will be a delay only on the first access when the cache is empty (the wait time is recorded in log.txt as `chord-chart: event=catalog-first-load elapsed_ms=...`). If the data cannot be fetched, "Chord progression data is not available" will appear at the bottom of the screen.

### Configuration

`config.toml` is automatically created on first launch. It is located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW's NORMAL mode, pressing `e` opens `config.toml` in an editor. Restart the application after closing the editor.

Current configuration example:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering workers for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the main cmrt process.
# render_server: Renders by POSTing /render to a render-server child process.
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
| `plugins."Surge XT".plugin_path` | OS-specific standard Surge XT CLAP path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | List of editor candidates to try in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in_process renders. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server executions. |
| `offline_render_server_port` | `62153` | Localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directory | List of directories to search for Surge XT patches during timbre selection. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after making changes. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories that can be assigned to loop directories. The key for the category overlay is determined from unused English letters in the category name. |

The default `plugin_path` values for each OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` values for each OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines where no timbre is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<Name>]` and used in lines where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths set to the standard installation location for each OS:

| Name | plugin_id | patches_dirs | Category by Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category name |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category name (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to specify `[plugins.<Name>]` if you've installed a plugin in a non-standard location, need to supplement patch locations, or are using a plugin not built-in. **Only the specified items will override the built-in values**, so if you just want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only replaces the path. plugin_id and patches_dirs remain as built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all fields.
[plugins.my_synth]
plugin_path   = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Patch location for that plugin. Write `patches_dirs = []` to clear built-in values. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filters for automatic patch selection by usage. Seven key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`) can be specified. Only the specified items take effect for that plugin. If not specified, the plugin's default value (Surge XT uses category names, others use "no filtering") is used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error, not silent ignoring. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are "1 `.syx` cartridge = 32 programs", so in the list, cartridges are treated as directories, and programs are listed individually, like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2 digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly is an instance setting (`MonoMode`), not a timbre setting, and its default is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 timbre per `.vvp` file, and can be selected just like Surge's `.fxp` files. The category shown in the list's header is derived from the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 is the only plugin that does not have a default `patches_dirs`. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and cmrt arbitrarily reading/writing there could corrupt your DAW environment. Please add a single line as shown below. Until you do, it will not appear in the catalog with 0 timbres.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly differs per timbre and is read from the `.vvp` file content (`m_uPolyMode`). Therefore, only timbres that can play chords are offered as candidates in the grid sequencer's chord rows (timbres that cannot be read are not offered).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These timbres are designed with MPE (per-note pitch and pressure) performance information in mind, which cmrt does not send.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for all rows). This is because Dexed cartridges do not follow a "directory name = usage" convention, and the patch organization of non-built-in plugins is unknown. If you wish to change this, specify the 7 items under `[plugins.<Name>]` (the default values for Surge XT are included as comments at the end of the generated config.toml).
- The 7 usage-specific category items should also be written only within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly judgment data (`voicing_shared_source` / `voicing_override_source`) used for usage-based automatic selection is only for Surge XT timbre judgment.
- Rendering result caches are stored in separate directories per plugin, preventing accidental use of sounds from different plugins even when mixed (no manual deletion is required). The locations are these two, where `<Plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<Plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<Plugin>\*.wav` (DAW track WAV)

When `offline_render_backend = "render_server"` is set, the TUI does not directly load CLAP plugins. Instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, cmrt will launch a child process and, in case of a communication error, restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interacts with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Writing `cde` plays Do-Re-Mi.

```
cmrt CM7
```

- Writing `CM7` plays C Major Seventh.
- Also supports various chord progression notations (some are not yet supported).

### Patch Roles Command

```
cmrt patch-roles
```

- Displays the number of timbre candidates available for selection via the PATCH wheel for each row in the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The screen does not launch.
- Used to check if the PATCH wheel has become unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.).
- If any row has 0 candidates, it will list that row and exit with exit code 1.
- Adding `--config <PATH>` reads that `config.toml`. This allows you to test how changes to settings would affect things without modifying your current `config.toml`.
- When timbres from multiple plugins are listed, the breakdown by plugin will also be shown for each usage category's candidate count. This is because the total count alone might not reveal that "a certain plugin has no timbres appearing for that row."

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Performs offline rendering of MML with the specified timbre and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound on a single line. The screen does not launch.
- While `patch-roles` counts whether timbres appear in the list, this command checks whether that timbre actually produces sound.
- `--patch` can be specified multiple times. A summary line showing "N / M distinct sounds" helps you identify if **the timbre changed but the sound remained the same as before**.
- Adding `--out-dir <DIRECTORY>` writes WAV files (if not specified, no bytes are written). Use this when you want to confirm by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre can play chords.
- `--config <PATH>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is more appropriate to acquire Surge XT patches via API, so this will be implemented (currently, searching specified items in toml is inefficient. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- アトミック小節
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - ※"atomic measure" sounds like a physics term, so for now, I'll keep it as "アトミック小節" without direct English translation.

# Out of Scope
- Effects require editing, so they are explicitly out of scope and deferred to a much later stage. One reason for this is that in Surge XT's case, patches inherently include effects (effects are derived from patches).