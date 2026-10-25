# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) with MML. Written in Rust.

### Usage

- For playing around with sounds using MML
- For casual installation. Just having Rust is enough.

### Technology Stack
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

#### Play Server Implementation Details

Sound playback is handled by a separate process, the play server. The executable is determined by the following order, using the first one found:

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it will stop with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`)

PATH is not consulted. A debug build server is 4-5 times slower at pre-reading, causing playback to cut off at the beginning of measures. A warning will appear in the top-right corner of the screen when using a debug build or an executable of unknown origin.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP plugins available for free on Windows without account registration*
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effects (can be inserted in series on tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- SFZ (those that allow trying key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Download pages for each (for those who get lost)
  - Surge XT : Easy installation with winget : [Fastest! DAW and audio plugin installation even for cats (up to playing sound with virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download page](https://floe.audio/download/)
  - [Sforzando Download page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

### AI-Generated Documentation
From this point onwards, sections added by AI may be hard to read. These will be maintained periodically.

### Keyboard Screen

Press `v` to navigate to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B (Do Re Mi Fa Sol La Si).

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to view and edit the 'structure' of a song's chord progression. Moving the cursor will play the chord progression of that row. Traversing chords one by one within a row using `h` `l` will play only the pointed chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- Correcting a section's progression will update all references to it throughout the song.
- The first line of the header displays the chord2mml specification placed at the beginning of the song (e.g., `Key=C BPM120`).
- This screen does not interpret progressions or headers. It stores the entered strings as is.
- Only the section on the cursor line will play (not the entire song). The overall Chord Chart timbre can be selected from the progression editing screen.
- `h` `l` allows you to preview chords one by one within a row. The currently selected chord appears highlighted within the progression.
- Only the Key from the header is passed for preview (BPM remains at its default).
- Chord inversions and octaves in previews are always auto-voiced. In Sections, they are determined within the section; in Arrangement, by the overall song sequence. `h` `l` single-chord previews also maintain the same inversion.

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

Here are the keybindings. Only `q` and `?` are shown in the bottom line of the screen. The full list appears on screen by pressing `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Pane movement (toggles Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement (row. The entire chord progression of the new row plays) |
| `h` `l` `←` `→` | Common | Chord movement within a row (only the selected chord plays. Moves to the adjacent row at the end of a line) |
| `PgUp` `PgDn` | Common | Moves the cursor 10 rows |
| `dd` | Common | Deletes the cursor row (deleting in Sections also removes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Moves the cursor row up / down |
| `b` | Common | Rewrites the Key / BPM in the header with single-line input |
| `Shift+P` `Space` | Common | Previews the section on the cursor row (stops if already playing) |
| `x` | Common | Selects an effect chain for the Chord timbre (`Enter` to confirm, `Esc` to discard. Does not apply to Bass timbre) |
| `?` | Common | Toggles help (closes with `Esc` as well) |
| `q` | Common | Exits the application |
| `g` | Sections | Adds a section by drawing from the chord progression catalog |
| `r` | Sections | Re-draws the progression of the cursor row, retaining its name |
| `i` | Sections | Edits the progression using a single-line MML overlay for the Chord Chart |
| `n` | Sections | Edits the name with single-line input |
| `1`〜`9` | Arrangement | Inserts the section with that number after the cursor |

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end. While inputting, the chord at the cursor position will auto-voice and play whenever it's formed or changed. `Ctrl+Space` can play the entire progression with the same voicing. Unreadable input will not be played as alternative MML but can still be saved. Press `Enter` to confirm and save the progression (trimming leading/trailing whitespace), or `Esc` to discard changes.

Inside the editing screen, `Ctrl+T` opens the same timbre list as the regular MML overlay. The timbre doesn't change by merely moving the candidate; `Enter` confirms, and `Esc` reverts to the original timbre. The Chord Chart timbre is global to the screen and saved to `history.json` separately from the regular `Ctrl+P` MML overlay timbre. Even if you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

It auto-saves with each edit. The save location is `clap-mml-render-tui/history/chord_chart.json` under the configuration directory (on Windows, it's `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If the save file is missing or unreadable, when the screen is first opened, it performs a single `g`-like drawing to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` drawing is fetched from the network. There will be a wait only for the first time when the cache is not yet available (wait time is logged in `log.txt` as `chord-chart: event=catalog-first-load elapsed_ms=...`). If it fails to fetch, 'No chord progression data' will appear at the bottom of the screen.

### Effect Chain in DAW Screen

In NORMAL mode of the DAW screen, placing the cursor on a performance track and pressing `x` opens the EFFECT CHAIN overlay for that track. You can insert any number of factory presets from TONE3000 / Surge XT Effects / Dragonfly Reverb in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Opens the EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor rows) |
| `j` `k` | Moves between chain stages |
| `a` | Opens the add overlay. All factory presets for all effects are listed in one column; select with `j` `k`, `Enter` to add to the end, `ESC` to return |
| `dd` | Deletes the cursor stage |
| `Enter` | Writes back to the `init` column's JSON and closes (the track's cache WAV is re-rendered) |
| `ESC` | Discards changes and closes |

- The chain is saved in the `init` column's JSON under `"effects after instrument"` (array order = signal flow order). Direct editing of the `init` column has the same effect.
- Effects are baked into the cache WAV (applied on the render-server side).
- Each effect's preset is loaded from the built-in default locations (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb's presets are built into the plugin itself, so if the plugin exists at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, its presets will appear as candidates. If the plugin is not found, its presets will not appear.
- Since the chain runs only for the duration of the notes, reverb tails will be cut off at the end of the cell.

In the Guitar Articulation screen (`Ctrl+G` → `E`), `x` also opens the same EFFECT CHAIN overlay. You can insert a guitar amp (press `a` then select `Amp Simulator` kind, TONE3000) or other effects after METAL-GTX and preview them with articulated performance whenever you change stages. Confirming with `Enter` applies the chain to subsequent `b` and `space` performances. The chain disappears when the application exits.

### Configuration

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example configuration.

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi and output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by a render-server child process.
# Concurrent execution count (1-16) - port - startup command (searches for executable if empty)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Realtime playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: immediately plays the current line. DAW mode: starts playback from the beginning of the song (measure 0).
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

The configuration items are as follows.

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates to try in order from left. |
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline rendering (render-server) executions. |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Startup command for the render-server. If empty, it searches for the executable. |
| `realtime_audio_backend` | `cache_player` | Real-time playback execution target (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for Surge XT timbres. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories that can be assigned to loop directories. The key for the category overlay is determined from unused English letters within the category name. |

OS-specific default `plugin_path` values are as follows.

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default `patches_dirs` values are as follows.

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified timbre is **Surge XT fixed**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<Name>]` and used on lines where the timbre is explicitly stated.

The contents of the built-in profiles are as follows, with paths set to OS-specific standard installation locations.

| Name | `plugin_id` | `patches_dirs` | Usage-specific categories |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | `Presets` under registry (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings`'s `InstallPath`). `patches_dirs` in config is ignored | Vaporizer2 category names (e.g., `Pad` / `Bass` / `Arpeggio`) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory timbre acquisition location). `patches_dirs` in config is ignored | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | `Presets\TyrellN6` under registry (`HKCU\Software\u-he\TyrellN6`'s `DataPath`). `patches_dirs` in config is ignored | All empty (= no filtering) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same). You only write `[plugins.<Name>]` if you are installing in a non-standard location, supplementing timbre locations, or using a plugin not built-in. **Only the written items will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Path only replaced. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify everything.
[plugins.my_synth]
plugin_path   = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Location for that plugin's timbres. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filtering for usage-specific patch auto-selection. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the written items take effect for that specific plugin. If omitted, the plugin's default value (Surge XT uses category names, others use "no filtering") is used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific categories at the top level will result in a configuration error rather than silent ignoring. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only profiles with the same name as Surge XT will override fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are '1 `.syx` cartridge = 32 programs', so in the list, cartridges are treated as directories, and programs are listed one by one like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp`.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a timbre setting, and its default is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 `.vvp` file = 1 timbre, and can be selected just like Surge's `.fxp`. The category displayed in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe timbre locations are read from the plugin's own settings (Vaporizer2 reads `InstallPath\Presets` from the registry, Floe reads `extra-presets-folder` from `%PUBLIC%\Floe\Preferences\floe.ini`; if unset in Floe, it reads `%PUBLIC%\Floe\Presets`). Reinstallation or changing the plugin's internal settings does not require rewriting `config.toml`. Writing `patches_dirs` under `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines factory timbres are embedded in the plugin itself and not on disk, so `cmrt build-patch-catalog-cache` fetches the same version as the installed Six Sines from GitHub (if already fetched and version is the same, no communication occurs). Mono/poly is read from the contents of `.sxsnp` (play mode).
- TyrellN6 timbres are 1 `.h2p` file = 1 timbre, and subfolders under the timbre location (e.g., `01 Basses`) become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the timbre; all timbres are treated as chord-friendly in the grid sequencer's chord rows.

- Vaporizer2's mono/poly differs per timbre and is read from the contents of `.vvp` (`m_uPolyMode`). Therefore, only timbres that play chords appear as candidates in the grid sequencer's chord rows (timbre files that cannot be read are not presented as chord row candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These are timbres that rely on MPE performance information (per-note pitch and pressure), which cmrt does not send.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (meaning all programs are candidates for any row). This is because Dexed cartridges do not follow a 'directory name = usage' system, and the timbre organization of non-built-in plugins is unknown, so they are not filtered. If you wish to change this, write the 7 items in `[plugins.<Name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-specific auto-selection is only used for Surge XT timbre determination.
- Among the timbres in the drum percussion column, those that change pitch when the note is varied (where `tracks_keyboard` is true in the shared table `cat-music-patterns/patch-pitch-tracking.json`) are moved to the lead `pitched percussion` column, regardless of the plugin (timbres identifiable as mallet or bell by name go into that column). Timbres in the kick, snare, and hi-hat columns are not moved. The table is fetched on startup and saved to `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\patch-pitch-tracking.json`. If it cannot be fetched, the previously saved table is used; if that's also unavailable, the list is displayed without moving anything. The table can be rebuilt with `cmrt build-pitch-tracking --output <PATH>` (which launches `probe-pitch-tracking` from clap-mml-play-server for each timbre to measure).
- Rendering result caches are stored in separate directories for each plugin, preventing accidental use of sounds from different plugins even when mixed (no manual deletion is required). There are two storage locations, where `<PLUGIN>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<PLUGIN>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<PLUGIN>\*.wav` (DAW track WAV)

All offline rendering is performed via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins; it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, cmrt launches a child process and retries once after restarting it in case of a communication error. If `offline_render_server_command` is empty, the child process executable is searched for in the order of 'same directory as `cmrt.exe` -> sibling repo `clap-mml-play-server`'s release build', and **PATH is not consulted**.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension
- When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` plays Do Re Mi (C D E).

```
cmrt CM7
```

- Typing `CM7` plays C Major Seventh.
- Also supports various chord progression notations (some are not yet supported).

### Patch Roles Command

```
cmrt patch-roles
```

- Displays how many timbre candidates are available for selection with the wheel in the PATCH column for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Used to check if the wheel is 'unresponsive' after changing plugins, `patches_dirs`, or usage-specific categories (e.g., `chord_patch_categories`).
- If there is a row with 0 candidates, it lists that row and exits with exit code 1.
- Adding `--config <PATH>` reads that `config.toml`. This allows you to test how changes to settings would affect without modifying your current `config.toml`.
- When multiple plugins' timbres are listed, the count of candidates per usage also shows a breakdown by plugin. This is because relying solely on the total count might not reveal that 'a specific plugin's timbre is not appearing at all for that row'.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound on a single line. No screen is launched.
- While `patch-roles` counts whether a timbre appears in the list, this command checks 'whether that timbre actually produces sound'.
- `--patch` can be specified multiple times. A summary line will show 'N / M distinct outputs', allowing you to check if the **timbre changed but the sound remained the same as before**.
- Adding `--out-dir <DIRECTORY>` writes WAV files (nothing is written if omitted). Use this when you want to verify by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre supports polyphonic playback.
- `--config <PATH>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes sense to acquire Surge XT patches via API, so that will be implemented (currently, they are inefficiently searched via toml specification. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- アトミック小節
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing 'offline rendering in 1-measure units,'
    - while imposing constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *'Atomic measure' could be confused with a physics term, so for now, we'll keep it as 'アトミック小節' without direct English translation.*

# Out of Scope
- Effects are essential for editing, but we've decided to consider them out of scope and defer them significantly. One reason is that Surge XT patches often encapsulate effects (effects are derived from patches).