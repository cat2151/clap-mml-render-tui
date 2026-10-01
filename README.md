# clap-mml-render-tui

### Overview
An MML TUI DAW (or similar). Enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) / [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) easily with MML. Written in Rust.

### Purpose

- For playing around with MML sounds
- For casual installation. Just having Rust is enough.

### Technology Stack
- Plugin Host Library
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

### Usage

```
cmrt
```

You can play with MML input on the TUI screen.

#### The Play Server

Sound playback is handled by a separate process, the play server. Its executable is determined in the following order, using the first one found:

1. The full path specified by `--play-server <PATH>` (if not found, it stops with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`)

It does not look at PATH. Debug build servers have 4-5 times slower pre-loading, causing playback to cut off at the beginning of measures.
A warning will appear in the upper-right corner of the screen if a debug build or an unknown executable is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP plugins available for free on Windows without account registration
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effect (can be inserted in series into tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
- sfz (those for which key switches and control changes can be tested on a dedicated screen)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 
- Download pages (for those who get lost)
  - Surge XT: Easy to get with winget: [The fastest way for anyone to install a DAW and audio plugins (up to playing sounds with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download Page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub Releases Page](https://github.com/michaelwillis/dragonfly-reverb/releases)
  - [METAL-GTX](https://unreal-instruments.wixsite.com/unreal-instruments/metal-gtx) 

### AI-Generated Documentation
- The AI-appended sections below may be difficult to read. I will maintain them periodically.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play C, D, E, F, G, A, B notes.

### Chord Chart Screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to view and edit the "structure" of chord progressions for an entire song. Moving the cursor will play the chord progression of that row.
When you navigate through chords one by one within a row using `h` `l`, only the selected chord will play.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The arrangement of defined sections forms the song. The same section can be arranged multiple times.
- If you modify a section's progression, all references to it within the song will change simultaneously.
- The first line of the header displays chord2mml specifications (e.g., `Key=C BPM120`) to be placed at the beginning of the song.
- Neither progressions nor headers are interpreted by this screen. It retains the entered strings as is.
- Only the section on the cursor's row plays (not the entire song). You can choose the overall timbre for the Chord Chart from the progression editing screen.
- You can preview chords one by one within a row using `h` `l`. The currently selected chord appears inverted within the progression.
- Only the Key from the header is passed for preview (BPM remains at its default).
- In previews, inversion and octave are always auto-voiced. In Sections, it's determined within the section; in Arrangement, it's based on the overall song order. Single chord previews with `h` `l` also maintain the same inversion.

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

These are the keybindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on the screen when you press the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Pane navigation (toggles between Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Cursor movement (row. The chord progression of the newly selected row will play entirely) |
| `h` `l` `←` `→` | Common | Move within chords in a row (only the selected chord plays. Moves to the adjacent row at the end of a line) |
| `PgUp` `PgDn` | Common | Moves the cursor by 10 rows |
| `dd` | Common | Deletes the cursor row (deleting in Sections also removes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Moves the cursor row up / down |
| `b` | Common | Rewrites the header's Key / BPM with a single line input |
| `Shift+P` `Space` | Common | Previews the section on the cursor's row (stops if already playing) |
| `?` | Common | Opens/closes help (closes with `Esc` too) |
| `q` | Common | Exits the application |
| `g` | Sections | Adds a section by drawing from the chord progression catalog |
| `r` | Sections | Re-draws the progression of the cursor row, keeping its name |
| `i` | Sections | Edits the progression with a single-line MML overlay for the Chord Chart |
| `n` | Sections | Edits the name with a single-line input |
| `1`〜`9` | Arrangement | Inserts the section with that number after the cursor |

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end.
While typing, the chord at the cursor position will play with auto-voicing each time it forms or changes. `Ctrl+Space` plays the entire progression with the same voicing. Unreadable input will not be played as alternative MML but can be saved as is. Press `Enter` to confirm and save the progression, removing leading/trailing spaces. `Esc` discards changes.

Within the editing screen, `Ctrl+T` opens the same timbre list as the regular MML overlay. Moving through candidates alone does not change the timbre. Press `Enter` to confirm, or `Esc` to revert to the original timbre. The Chord Chart timbre is a single setting for the entire screen. It is saved to `history.json` separately from the timbre of the regular `Ctrl+P` MML overlay. Even if you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

It is automatically saved every time you edit. The save location is under the settings directory at
`clap-mml-render-tui/history/chord_chart.json` (on Windows, it's
`%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If the save file does not exist or cannot be read, when the screen is first opened, a single `g`-like draw is performed.
It starts with one section (if a draw cannot be made, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. It will only wait on the first attempt if the cache is empty
(The waiting time is recorded in log.txt as
`chord-chart: event=catalog-first-load elapsed_ms=...`). If it fails to fetch,
"No chord progression data" will appear at the bottom of the screen.

### DAW Screen Effect Chain

In DAW screen's NORMAL mode, pressing `x` while the cursor is on a performance track opens that track's EFFECT CHAIN overlay.
You can insert any number of factory presets from TONE3000 / Surge XT Effects / Dragonfly Reverb in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Opens the EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor rows) |
| `j` `k` | Moves up/down the chain stages |
| `a` | Opens the Add overlay. All factory presets for all effects are listed in a single column; select with `j` `k`, press `Enter` to add to the end, `ESC` to return. |
| `dd` | Deletes the cursor stage |
| `Enter` | Writes back to the init column's JSON and closes (the track's cached WAV is re-rendered) |
| `ESC` | Discards changes and closes |

- The chain is saved in the init column's JSON under `"effects after instrument"` (array order = signal flow order). Editing the init column directly has the same effect.
- Effects are baked into the cached WAV (applied on the render-server side).
- Each effect's preset is loaded from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb's presets are built into the plugin itself, so if the plugin is present at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, it will appear as a candidate. If the plugin is not found, no candidates will appear.
- The chain runs for the duration of the notes, so reverb tails are cut off at the end of the cell.

The same EFFECT CHAIN overlay can also be opened by pressing `x` in the Guitar Articulation screen (`Ctrl+G` → `E`).
You can insert a guitar amp (press `a` → kind `Amp Simulator`, TONE3000) or other effects after METAL-GTX and preview them with articulated playback each time you change a stage.
Once confirmed with `Enter`, the chain will also be applied to subsequent `b` and `space` playback. The chain is discarded when the application exits.

### Configuration

Upon first launch, `config.toml` is automatically created. Its location is within the OS's standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example configuration:

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under the settings directory at
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by the render-server child process.
# Number of concurrent executions (1-16), port, and startup command (searches for executable if empty)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
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

The configuration items are as follows:

| Item | Default Value | Description |
| --- | --- | --- |
| `plugins."Surge XT".plugin_path` | OS-specific standard Surge XT CLAP path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline render-server executions. |
| `offline_render_server_port` | `42153` | localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Startup command for the render-server. If empty, it searches for the executable. |
| `realtime_audio_backend` | `cache_player` | Real-time playback target (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directories | List of directories to search for Surge XT timbres. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after changes. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop dirs. The key for a category overlay is determined from unused English letters within the category name. |

The OS-specific default `plugin_path` values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The OS-specific default `patches_dirs` values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is unset, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines where no timbre is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog via `[plugins.<name>]` and used on lines where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths being the standard installation locations per OS.

| Name | plugin_id | patches_dirs | Category for Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | Under registry (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings` `InstallPath`) `Presets`. Ignores `patches_dirs` in config. | Vaporizer2 category names (e.g., `Pad` / `Bass` / `Arpeggio`) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory timbre acquisition destination). Ignores `patches_dirs` in config. | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | Under registry (`HKCU\Software\u-he\TyrellN6` `DataPath`) `Presets\TyrellN6`. Ignores `patches_dirs` in config. | All empty (= no filtering) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You should only write `[plugins.<name>]` if you are installing in a non-standard location, supplementing timbre locations, or using a plugin not built-in. **Only the items you write will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# Only replace the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, write all properties.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to the plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Timbre locations for the plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<name>.<purpose>_patch_categories` / `<role>_patch_keywords` | Filter for automatic patch selection by purpose. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the written items apply to that plugin. If not written, the plugin's default values (Surge XT uses category names; others use "no filtering") are used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 purpose-specific categories at the top level will result in a configuration error, rather than silently ignoring them. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only profiles with the same name as Surge XT become overrides for the fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are "1 `.syx` cartridge = 32 programs," so in the list, cartridges are treated as directories, and programs are listed individually like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not part of the timbre, and its default value is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 `.vvp` file = 1 timbre, and can be selected just like Surge's `.fxp` files. The category displayed in the list's heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe timbre locations are read from the plugin's own settings (Vaporizer2 from `InstallPath\Presets` in the registry, Floe from `extra-presets-folder` in `%PUBLIC%\Floe\Preferences\floe.ini`; if unset in Floe, then `%PUBLIC%\Floe\Presets`). Reinstallation or changing settings within the plugin itself does not require rewriting config.toml. Writing `patches_dirs` in `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines' factory timbres are embedded in the plugin itself and not on disk, so when `cmrt build-patch-catalog-cache` is run, it fetches the same version as the installed Six Sines from GitHub (no communication if already fetched and versions match). Mono/poly is read from the `.sxsnp` content (play mode).
- TyrellN6 timbres are 1 `.h2p` file = 1 timbre, and subfolders under the timbre location (e.g., `01 Basses`) become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the timbre, and all timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2's mono/poly differs per timbre and is read from the `.vvp` content (`m_uPolyMode`). Therefore, only timbres that play chords appear as candidates in the grid sequencer's chord rows (timbres that cannot be read are not presented as chord row candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. This is because these timbres are designed with MPE (per-note pitch and pressure) performance information in mind, which cmrt does not transmit.
- The default category settings for filtering candidates by row purpose (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for any row). This is because Dexed cartridges do not follow a "directory name = purpose" structure, and the timbre organization of non-built-in plugins is unknown, so no filtering is applied. If you want to change this, please write the 7 items in `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated config.toml).
- The 7 purpose-specific categories should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared determination data for mono/poly used for automatic selection by purpose (`voicing_shared_source` / `voicing_override_source`) is only used for Surge XT timbre determination.
- Caches of rendering results are placed in separate directories for each plugin, so mixing them will not lead to accidental use of sounds from other plugins (no manual deletion is required). The locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (cache for notepad / MML input overlay)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI (`cmrt.exe`) does not load CLAP plugins directly; it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If the connection to the render-server fails, cmrt starts a child process, and in case of communication errors, it retries once after restarting. If `offline_render_server_command` is empty, the child process executable is searched in the order of "same directory as `cmrt.exe` → release build of sibling repo `clap-mml-play-server`," and **PATH is not consulted**.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interacts with the bluesky-text-to-audio Chrome extension
- When an MML is found in a Bluesky post, it can be played with Surge XT

### CLI Mode

```
cmrt cde
```

- Typing "cde" plays C, D, E.

```
cmrt CM7
```

- Typing "CM7" plays a C major seventh chord.
- It also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- For each row in the grid sequencer (chord / bass / arpeggio / 4 drum roles / other), it displays
  how many timbre candidates are available for selection with the PATCH wheel. The GUI does not launch.
- Use this after changing plugins, `patches_dirs`, or purpose-specific categories (`chord_patch_categories`, etc.),
  to check if the "wheel is unresponsive."
- If there is a row with 0 candidates, it lists that row and exits with error code 1.
- Adding `--config <path>` makes it read that config.toml. If settings are changed,
  you can test the effect without modifying the currently used config.toml.
- When timbres from multiple plugins are listed, the number of candidates for each purpose also includes a breakdown by plugin.
  This is because totals alone might not reveal that "no timbres from a specific plugin are appearing for that row."

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays the length, volume (`peak` / `rms`), whether it's silent, and
  the sound output digest value on a single line. The GUI does not launch.
- While `patch-roles` counts "if a timbre appears in the list," this command checks "if that timbre actually produces sound."
- `--patch` can be specified multiple times. A summary line "different outputs N / M" will appear, allowing you to check if
  **the timbre changed but the sound remained the same as before**.
- Adding `--out-dir <directory>` writes WAV files (no bytes are written if not specified).
  Use this when you want to confirm by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre supports polyphonic playback.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Retrieving Surge XT patches via API is the proper way, so that will be implemented (currently, they are inefficiently searched from toml specifications. Implementation timing is deferred; other priorities are higher).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - in exchange for constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Note: "atomic measure" sounds like a physics term, so for now, "アトミック小節" (Atomic Measure) is kept without direct English translation.

# Out of Scope
- Since effects require editing, they are deliberately considered out of scope and deferred to much later. One reason for this is that Surge XT's patches include effects (effects are derived from patches).