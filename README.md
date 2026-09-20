# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation. Just having Rust installed is enough.

### Technology Stack
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

You can input MML and play in the TUI screen.

#### Play Server Implementation

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build from the sibling repository (`../clap-mml-play-server/target/release/`)

PATH is not consulted. Debug build servers have a 4-5 times slower pre-fetch, causing playback to cut off at the beginning of measures.
A warning appears in the top right of the screen when a debug build or an unknown executable is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- * Limited to plugins that are CLAP, Windows-compatible, and freely available without account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando
- Effects (can be inserted in series into DAW tracks)
  - TONE3000
  - Surge XT Effects

### AI Generated Documentation
- The sections appended by AI below may be difficult to read. They will be maintained periodically.

### Keyboard Screen

Press the `v` key to navigate to the keyboard screen.

- `c d e f g a b` keys: Play the musical notes C D E F G A B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to view and edit the "structure" of chord progressions for an entire song. Moving the cursor will play the chord progression on that line.
Use `h` `l` to step through chords within a line; only the pointed chord will play.

- Left pane (Sections): Define named chord progressions that serve as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- If you correct a section's progression, all its references in the song will change collectively.
- The first line in the header displays the `chord2mml` specification (e.g., `Key=C BPM120`) to be placed at the beginning of the song as is.
- Neither the progressions nor the header are interpreted by this screen. It retains the typed strings as-is.
- Only the section on the cursor's line will play (not the entire song). The overall timbre for the Chord Chart can be selected from the progression editing screen.
- Use `h` `l` to preview chords one by one within a line. The currently pointed chord will appear inverted within the progression.
- Only the Key from the header is passed for preview (BPM remains at its default).
- Chord inversions and octaves during preview are always auto-voiced. In Sections, they are determined within the section; in Arrangement, they are determined by the overall song sequence, and `h` `l` single chord previews maintain the same inversion.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:Exit ?:Help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

These are the key bindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on screen by pressing the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Move pane (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (line. All chord progressions in the new line will play) |
| `h` `l` `←` `→` | Common | Move chord within line (only the pointed chord plays. At the end of a line, it wraps to the next line) |
| `PgUp` `PgDn` | Common | Move cursor by 10 lines |
| `dd` | Common | Delete cursor line (deleting in Sections also deletes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor line up / down |
| `b` | Common | Rewrite header Key / BPM with single line input |
| `Shift+P` `Space` | Common | Preview cursor line's section (stops if already playing) |
| `?` | Common | Toggle help (`Esc` also closes it) |
| `q` | Common | Exit application |
| `g` | Sections | Add section by drawing from the chord progression catalog |
| `r` | Sections | Redraw progression for cursor line, preserving its name |
| `i` | Sections | Edit progression using single-line MML overlay for Chord Chart |
| `n` | Sections | Edit name with single line input |
| `1`〜`9` | Arrangement | Insert section with that number after the cursor |

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end.
During input, the chord at the cursor position will auto-voice and play whenever it's formed or changed. `Ctrl+Space` plays the entire progression with the same voicing. Unreadable input is not substituted for MML playback but can still be saved.
Press `Enter` to confirm and save the progression (whitespace trimmed from both ends), and `Esc` to discard changes.

From within the editing screen, `Ctrl+T` opens the same patch list as the regular MML overlay. Moving through candidates does not change the patch; `Enter` confirms it, and `Esc` reverts to the original patch. The Chord Chart's patch is global to the entire screen and saved to `history.json` separately from the regular `Ctrl+P` MML overlay's patch. If you discard progression edits with `Esc` after confirming a patch, the confirmed patch remains.

It is automatically saved with every edit. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, this is `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If the save file is missing or unreadable, when the screen is first opened, it performs a single `g`-like drawing to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` drawing is retrieved from the network. It will only wait on the first access when the cache is not yet available (wait time is logged in `log.txt` under `chord-chart: event=catalog-first-load elapsed_ms=...`). If it cannot be retrieved, "Chord progression data is not available" will appear at the bottom of the screen.

### DAW Screen Effect Chain

In NORMAL mode of the DAW screen, place the cursor on a performance track and press `x` to open that track's EFFECT CHAIN overlay.
You can insert any number of TONE3000 / Surge XT Effects factory presets in series after the instrument (patch).

| Key | Action |
|---|---|
| `x` | Open EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor lines) |
| `j` `k` | Move through effect chain stages |
| `a` | Open add overlay. All factory presets for all effects are listed in one column; select with `j` `k`, add to end with `Enter`, `ESC` to return |
| `dd` | Delete the cursor's stage |
| `Enter` | Write back to init column JSON and close (the track's cached WAV is re-rendered) |
| `ESC` | Discard changes and close |

- The chain is saved in the `"effects after instrument"` (array order = signal order) within the init column's JSON. Editing the init column directly has the same effect.
- Effects are baked into the cached WAV. This works regardless of whether `offline_render_backend` is `in_process` or `render_server`.
- Each effect's preset is read from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). If a plugin is not available, it won't appear as a candidate.
- The chain runs for the same duration as the notes, so reverb tails will be cut off at the end of the cell.

### Configuration

`config.toml` is automatically created on first launch. Its location is within the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, press `e` to open `config.toml` in an editor. Restart the application after closing the editor.

Current configuration example:

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the config directory.
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
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: Plays current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for WAV loops in the WAV loop browser
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
| `plugins."Surge XT".plugin_path` | OS-specific default Surge XT CLAP path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent renders for in_process. |
| `offline_render_backend` | `in_process` | Target for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server workers. |
| `offline_render_server_port` | `42153` | localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to start render_server. |
| `realtime_audio_backend` | `in_process` | Target for real-time playback. |
| `realtime_play_server_port` | `42154` | localhost port for play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches default directories | List of directories to search for Surge XT patches. |
| `loop_dirs` | `[]` | List of directories to search for WAV loops in the WAV loop browser. Run `cmrt scan-loops` after making changes. |
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

The default plugin for lines without a specified patch is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog via `[plugins.<name>]` and used on lines where the patch is explicitly specified.

The contents of the built-in profiles are as follows, with paths being the standard installation locations for each OS:

| Name | `plugin_id` | `patches_dirs` | Category for usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT's category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (e.g., `Pad` / `Bass` / `Arpeggio`) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated the same).

You only need to write `[plugins.<name>]` if you've installed a plugin in a non-standard location, need to specify a patch location, or are using a plugin not included by default. **Only the specified items will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Replace only the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all fields.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to the plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Patch locations for the plugin. Write `patches_dirs = []` if you want to clear built-in values. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by usage. Seven key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`) can be specified. Only the specified items take effect for that plugin. If not specified, the plugin's default value (Surge XT uses its category names, others use "no filtering") is used. |

- `active_plugin` is deprecated. Also, specifying `plugin_path` / `plugin_id` / `patches_dirs` and the seven usage-specific categories at the top level will result in a configuration error, rather than silently ignoring them. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default value; others become candidates in the mixed catalog.
- Dexed patches are "1 `.syx` cartridge = 32 programs", so in the list, cartridges are treated as directories, and programs are listed one by one like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly is an instance setting (`MonoMode`), not a patch property, and its default is POLY. Therefore, all Dexed patches are treated as suitable for chords in the grid sequencer's chord lines.
- Vaporizer2 patches are 1 `.vvp` file = 1 patch, and can be selected just like Surge's `.fxp` files. The category that appears in the list heading is determined by the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because preset locations are environment-dependent values determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` automatically reading/writing there could corrupt your DAW environment. Please add a line as shown below. Until you do, it will not appear in the catalog with 0 patches.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly differs per patch, read from the `.vvp` file content (`m_uPolyMode`). Therefore, only patches that play chords will appear as candidates in the grid sequencer's chord lines (unreadable patches are not offered as candidates for chord lines).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These patches are designed to use MPE (per-note pitch and pressure) performance information, which `cmrt` does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for all lines). This is because Dexed cartridges do not follow a "directory name = usage" scheme, and the patch organization of non-built-in plugins is unknown, so no filtering is applied. If you want to change this, specify the 7 items in `[plugins.<name>]` (the generated `config.toml` includes Surge XT's default values as comments at the end).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-specific auto-selection is only applied to Surge XT patch determination.
- Rendering result caches are stored in separate directories per plugin, so mixing them will not lead to incorrect use of sounds from different plugins (no manual deletion is needed). The two locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI side does not directly load CLAP plugins, but instead sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once. If `offline_render_server_command` is empty, the child process executable is searched in the order of "same directory as `cmrt.exe` → release build of sibling repo `clap-mml-play-server`", and **PATH is not consulted**.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Integrates with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C D E.

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- Also supports various chord progression notations (some are not yet supported).

### patch-roles command

```
cmrt patch-roles
```

- Displays how many patch candidates are available for selection with the PATCH column's wheel for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The screen does not launch.
- Used to check if the wheel is unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (e.g., `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with code 1.
- If `--config <path>` is specified, it reads that `config.toml`. This allows you to test how changes to settings would behave without modifying your current `config.toml`.
- When patches from multiple plugins are available, the breakdown by plugin will also be shown for the number of candidates per usage. This is to ensure you notice if "a particular plugin has no patches available for that row," which might be missed by just the total.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound on a single line. The screen does not launch.
- While `patch-roles` counts whether a patch appears in the list, this command checks whether that patch actually produces sound.
- Any number of `--patch` arguments can be specified. A summary line showing "N / M different sounds" helps you determine if the **patch was changed but the sound remained the same as before**.
- Specifying `--out-dir <directory>` writes WAV files (otherwise, no bytes are written). Use this when you want to confirm by ear.
- Specifying `--poly-check` compares playing chords and single notes to determine if the patch can play polyphonically.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is more appropriate to retrieve Surge XT patches via API, so that will be implemented (currently, they are inefficiently searched via toml specification. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing 'offline rendering of a single measure',
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - * `atomic measure` could be misinterpreted as a term in physics, so for now, I will keep it as 'アトミック小節' (Atomic Measure) without directly translating it.

# Out of Scope
- Effects, being essential for editing, are intentionally deemed out of scope and pushed far back in priority. One reason for this is that Surge XT's patches include effects (effects are derived from patches).