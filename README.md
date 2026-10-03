# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html) / [Shu](https://mikey.audio/shu) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) using MML. Written in Rust.

### Usage

- For playing around with MML sounds
- For casual installation; just having Rust installed is sufficient

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

You can enter MML in the TUI screen and play around.

#### Play Server Implementation

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1.  The full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further).
2.  `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3.  The release build of the sibling repository (`../clap-mml-play-server/target/release/`).

The system PATH is not consulted. Debug build servers have 4-5 times slower pre-reading, causing playback to cut off at the start of measures.
A warning will appear in the top-right corner of the screen when a debug build or an unknown executable is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP, Windows, and free plugins that don't require account registration*
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effects (can be inserted in series into a track in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
- SFZ (those that allow testing key switches and control changes on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Respective download pages (for those who get lost)
  - Surge XT: Easy to get with winget: [Fastest! How to install DAW and audio plugins even for cats (until you can make sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download Page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub Releases Page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [Voyage Voyage](https://www.musicalentropy.com/VoyageVoyage.html)
  - [Shu](https://mikey.audio/shu)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

### AI Generated Document
- The AI-appended sections below are hard to read. I will maintain them occasionally.

### Keyboard Screen

Press `v` to navigate to the keyboard screen.

- `c d e f g a b` keys: Play the notes C-D-E-F-G-A-B.

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to overview and edit the "structure" of a song's chord progression. Moving the cursor plays the chord progression of that row.
Pressing `h` or `l` to step through the chords in a row will play only the pointed chord.

- Left pane (Sections): Defines named chord progressions as building blocks.
- Right pane (Arrangement): The arrangement of defined sections forms the song. The same section can be used multiple times.
- If you modify a section's progression, all references to it in the song will change simultaneously.
- The header row directly displays the `chord2mml` specification for the beginning of the song (e.g., `Key=C BPM120`).
- This screen does not interpret the progression or header; it holds the string as entered.
- Only the section on the cursor row plays (not the entire song). You can choose the timbre for the entire Chord Chart from the progression editing screen.
- Press `h` or `l` to audition chords one by one within a row. The currently pointed chord is highlighted within the progression.
- Only the Key in the header is passed to the audition (BPM remains at the default).
- Audition inversions and octaves are always auto-voiced. In Sections, they are determined by the connection within the section, and in Arrangement, by the overall song order. Single chord auditions with `h` or `l` maintain the same inversion.

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

Here are the key bindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on screen by pressing `?`.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Switch panes (toggle between Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. The entire chord progression of the new row plays.) |
| `h` `l` `←` `→` | Common | Move through chords in a row (only the pointed chord plays. Wraps to adjacent rows at the ends of a row.) |
| `PgUp` `PgDn` | Common | Move cursor by 10 rows. |
| `dd` | Common | Delete cursor row (deleting in Sections also deletes its references in Arrangement). |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down. |
| `b` | Common | Overwrite Key / BPM in the header with a single-line input. |
| `Shift+P` `Space` | Common | Audition the section on the cursor row (stops if already playing). |
| `x` | Common | Select effect chain to apply to Chord timbre (`Enter` to confirm, `Esc` to discard. Does not apply to Bass timbre). |
| `?` | Common | Toggle help (also closes with `Esc`). |
| `q` | Common | Quit application. |
| `g` | Sections | Add a section by drawing from a chord progression catalog. |
| `r` | Sections | Redraw the progression of the cursor row, keeping its name. |
| `i` | Sections | Edit progression using a single-line MML overlay for Chord Chart. |
| `n` | Sections | Edit name with a single-line input. |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor. |

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end.
While typing, the chord at the cursor position will play with auto-voicing as it forms and changes. `Ctrl+Space` can play the entire progression with the same voicing.
Invalid input will not be played as MML but can be saved as is.
`Enter` confirms and saves the progression, stripping leading/trailing whitespace. `Esc` discards changes.

Inside the editing screen, `Ctrl+T` opens the same timbre list as the regular MML overlay. Moving through candidates does not change the timbre; `Enter` confirms, `Esc` reverts to the original timbre. The Chord Chart timbre is global to the screen and saved to `history.json` separately from the regular `Ctrl+P` MML overlay timbre. Even if you discard the progression edit with `Esc` after confirming a timbre, the confirmed timbre remains.

Changes are auto-saved. The save location is `clap-mml-render-tui/history/chord_chart.json` within the settings directory (on Windows, `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If the save file is missing or unreadable, when the screen is first opened, `g` will be executed once to start with one section (if drawing fails, it remains empty).

The chord progression catalog for `g` / `r` draws from the network. The first time, it will wait if the cache is empty (the waiting time is recorded in `log.txt` as `chord-chart: event=catalog-first-load elapsed_ms=...`). If data cannot be retrieved, "Chord progression data not available" appears at the bottom of the screen.

### DAW Screen Effect Chain

In the DAW screen's NORMAL mode, place the cursor on a performance track and press `x` to open that track's EFFECT CHAIN overlay.
You can insert any number of factory presets from TONE3000 / Surge XT Effects / Dragonfly Reverb in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Open EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor rows). |
| `j` `k` | Move between chain stages. |
| `a` | Open add overlay. All factory presets from all effects are listed in one column. Select with `j` `k`, `Enter` to add to end, `Esc` to go back. |
| `dd` | Delete the cursor stage. |
| `Enter` | Write back to the JSON in the init column and close (the track's cached WAV will be re-rendered). |
| `ESC` | Discard changes and close. |

- The chain is saved in the `"effects after instrument"` JSON array within the init column (array order = signal flow order). Editing the init column directly has the same effect.
- Effects are baked into the cached WAV (applied on the render-server side).
- Each effect's preset is read from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb presets are built into the plugin itself, so if the plugin is in `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, it will appear as a candidate. If the plugin is not present, it won't appear as a candidate.
- The chain runs for the duration of the note, so reverb tails will be cut off at the end of the cell.

In the Guitar Articulation screen (`Ctrl+G` → `E`), pressing `x` also opens the same EFFECT CHAIN overlay.
You can insert guitar amps (`a` → kind `Amp Simulator`, TONE3000) or other effects after METAL-GTX and audition them with Articulated performances as you change stages.
Confirming with `Enter` applies that chain to subsequent `b` and `space` performances. The chain disappears when the application is closed.

### Configuration

A `config.toml` file is automatically created on first launch. It is located in the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example of the current settings:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates for opening config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi and output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by a render-server child process.
# Concurrency (1-16), port, launch command (explores for executable if empty)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: Plays current line immediately. DAW mode: Starts playback from song start (measure 0).
autoplay_on_startup = true

# List of directories to search for WAV loops in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to WAV loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Only write if you want to change Surge XT default values
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
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Concurrency for offline rendering (render-server). |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Launch command for the render-server. Explores for executable if empty. |
| `realtime_audio_backend` | `cache_player` | Real-time playback destination (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for Surge XT timbres. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop dirs. Category overlay keys are determined from unused English letters in the category name. |

OS-specific `plugin_path` default values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific `patches_dirs` default values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines where no timbre is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog from `[plugins.<Name>]` and used on lines where the timbre is explicitly stated.

The built-in profile contents are as follows, with paths being the OS-specific standard installation locations:

| Name | `plugin_id` | `patches_dirs` | Category by Use |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | `Presets` under registry (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings`'s `InstallPath`). Ignores `patches_dirs` in config. | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory timbre source). Ignores `patches_dirs` in config. | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | `Presets\TyrellN6` under registry (`HKCU\Software\u-he\TyrellN6`'s `DataPath`). Ignores `patches_dirs` in config. | All empty (= no filtering) |

Names are matched case-insensitively, ignoring spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all considered the same).

You only need to write `[plugins.<Name>]` if you are installing in a non-standard location, supplementing timbre locations, or using a plugin not included in the built-in list. **Only the specified items will overwrite the built-in values**, so if you just want to change Surge XT's path, one `plugin_path` line within that table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, define everything.
[plugins.my_synth]
plugin_path   = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Timbre locations for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Use>_patch_categories` / `<Role>_patch_keywords` | Filtering for automatic patch selection by use. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will be effective for that plugin. If not specified, the default value for that plugin (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` has been deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 use-specific category items at the top level will result in a configuration error, not silently ignored. Please remove `active_plugin` and move other values into `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are "1 cartridge `.syx` file = 32 programs", so in the list, each cartridge is treated as a directory, and programs are listed individually like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits). If you specify the cartridge location in `patches_dirs`, you can select them like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance setting (`MonoMode`), not a timbre setting, and its default is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are "1 `.vvp` file = 1 timbre" and can be selected like Surge's `.fxp` files. The category displayed in the list's heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe timbre locations are read from the plugin's own settings (Vaporizer2 from the registry's `InstallPath\Presets`, Floe from `%PUBLIC%\Floe\Preferences\floe.ini`'s `extra-presets-folder`. If not set for Floe, then `%PUBLIC%\Floe\Presets`). Reinstallation or changes in the plugin's own settings do not require editing `config.toml`. `patches_dirs` written in `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines factory timbres are embedded within the plugin and not on disk. Therefore, during `cmrt build-patch-catalog-cache`, they are retrieved from GitHub if the installed Six Sines is the same version (no communication if already retrieved and same version). Mono/poly is read from the `.sxsnp` content (play mode).
- TyrellN6 timbres are "1 `.h2p` file = 1 timbre", and folders under its location (e.g., `01 Basses`) become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the timbre; all timbres are treated as chord-friendly in the grid sequencer's chord rows.

- Vaporizer2's mono/poly setting differs per timbre, read from the `.vvp` content (`m_uPolyMode`). Therefore, only chord-playing timbres appear as candidates in the grid sequencer's chord rows (timbre that cannot be read will not be displayed as candidates in chord rows).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These timbres assume MPE (per-note pitch/pressure) performance information, which cmrt does not transmit.
- The default category settings for filtering candidates by row purpose (chord / bass / arpeggio / drum 4 roles / other) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and plugins not built-in use "no filtering" (= all programs are candidates for all rows). This is because Dexed cartridges do not follow a "directory name = purpose" scheme, and built-in plugins' timbre organization is unknown, so they are not filtered. If you wish to change this, write the 7 items in `[plugins.<Name>]` (the `config.toml` generated will include Surge XT's default values commented out at the end).
- The 7 use-specific category items should also only be written within the plugin profile. For Surge XT, use `[plugins."Surge XT"]`; for other plugins, place them in that plugin's own table.
- The shared determination data for mono/poly used in use-specific automatic selection (`voicing_shared_source` / `voicing_override_source`) is only used for Surge XT's timbre determination.
- Rendered cache results are stored in separate directories per plugin, so mixing plugins will not lead to accidental use of sounds from different plugins (no need to delete them manually). The locations are two-fold, where `<Plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<Plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<Plugin>\*.wav` (DAW track WAVs)

All offline rendering is done via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins; instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If connection to the render-server fails, cmrt launches a child process, and upon communication error, it restarts and retries once. If `offline_render_server_command` is empty, the child process executable is searched for in the order: "same directory as `cmrt.exe` → release build of sibling repo `clap-mml-play-server`". **The system PATH is not consulted.**

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works in conjunction with the bluesky-text-to-audio Chrome extension.
  - When MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` plays C-D-E.

```
cmrt CM7
```

- Typing `CM7` plays a C major seventh.
- It also supports various chord progression notations (some are currently not supported).

### Patch Roles Command

```
cmrt patch-roles
```

- Displays the number of timbre candidates available for each row (chord / bass / arpeggio / 4 drum roles / other) in the grid sequencer that can be selected with the PATCH wheel. The screen does not launch.
- Use this to check if "the wheel is unresponsive" after changing plugins, `patches_dirs`, or use-specific categories (`chord_patch_categories`, etc.).
- If any row has 0 candidates, it lists those rows and exits with code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test how changes would affect settings without modifying the currently used `config.toml`.
- When multiple plugin timbres are listed, the number of candidates for each purpose will also show a plugin-specific breakdown. This helps avoid not noticing if "a particular plugin's timbre is not appearing in that row" if only the total is shown.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and a digest value of the output sound on a single line. The screen does not launch.
- While `patch-roles` counts "whether a timbre appears in the list," this command checks "whether that timbre actually produces sound."
- `--patch` can be specified multiple times. A summary line shows "N / M different outputs," which helps verify if the **timbre changed but the sound remained the same as before**.
- Adding `--out-dir <directory>` writes WAV files (otherwise, no bytes are written). Use this when you want to listen to the output.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre plays polyphonically.
- `--config <path>` works the same as with `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes occur daily.

# Future Plans
- Obtaining Surge XT patches via API is the proper way, so that will be implemented (currently, specified paths in toml are searched, which is inefficient. Implementation timing is deferred, prioritizing other tasks).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing an "offline render of one measure,"
    - while facing constraints,
    - various benefits can be gained.
    - This is suited for sketching and quickly iterating through edits.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - *Since "atomic measure" might be confused with a physics term, for now, it's left as "アトミック小節" without direct translation.*

# Out of Scope
- Effects are considered essential for editing, so they are intentionally put out of scope and deferred to much later. One reason is that in Surge XT, patches already encapsulate effects (effects are derived from patches).