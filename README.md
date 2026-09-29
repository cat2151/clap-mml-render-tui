# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) using MML. Written in Rust.

### Usage

- For experimenting with MML sound generation
- For casual installation; just having Rust is enough

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

### Execution

```
cmrt
```

You can play by entering MML in the TUI screen.

#### The 'play server' executable

Sound generation is handled by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it stops with an error without searching further)
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`)

PATH is not checked. The debug build server's pre-loading is 4-5 times slower, causing playback to cut off at the beginning of measures. A warning appears in the top right of the screen when a debug build or an unknown executable is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- *Limited to CLAP plugins available for free on Windows without account registration.
- [Surge XT](https://surge-synthesizer.github.io/)
- [Dexed](https://asb2m10.github.io/dexed/)
- [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2)
- [Floe](https://floe.audio/)
- [Sforzando](https://www.plogue.com/products/sforzando.html)
- [Six Sines](https://github.com/baconpaul/six-sines)
- [TyrellN6](https://u-he.com/products/tyrelln6/)
- Effects (can be inserted in series into tracks in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (bundled with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
- Download pages for each (for those who get lost)
  - Surge XT: Easiest to get via winget: [The fastest way for anyone to install DAW and audio plugins (up to playing sounds with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe Download Page](https://floe.audio/download/)
  - [Sforzando Download Page](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub Releases Page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 Download Page](https://u-he.com/products/tyrelln6/)
  - [TONE3000 Download Page](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub Releases Page](https://github.com/michaelwillis/dragonfly-reverb/releases) 

### AI-Generated Documentation
- The parts added by AI below are difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press the `v` key to navigate to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B (Do Re Mi Fa Sol La Si).

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen allows you to view and edit the 'composition' of a song's chord progression. Moving the cursor will play the chord progression of that row. Pressing `h` or `l` to step through chords within a row will play only the selected chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be arranged multiple times.
- If you modify a section's progression, all references to it in the song will change simultaneously.
- The first line of the header shows the `chord2mml` settings (e.g., `Key=C BPM120`) to be placed at the beginning of the song, as is.
- Neither progressions nor headers are interpreted by this screen. It holds the entered string as is.
- Only the section on the cursor's row plays (not the entire song). You can choose the overall timbre for the Chord Chart from the progression editing screen.
- You can audition individual chords within a row using `h` and `l`. The currently selected chord appears inverted within the progression.
- Only the Key from the header is passed for auditioning (BPM remains at its default).
- In audition mode, inversions and octaves are always auto-voiced. In Sections, they are determined within the section; in Arrangement, they are determined by the overall song flow. Individual chord auditions with `h` `l` also maintain the same inversion.

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

These are the keybindings. Only `q` and `?` are displayed on the bottom line of the screen. The full list appears on-screen by pressing the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Move pane (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. The chord progression of the new row plays entirely) |
| `h` `l` `←` `→` | Common | Move chord within row (only the selected chord plays. Wraps to adjacent rows at row ends) |
| `PgUp` `PgDn` | Common | Move cursor by 10 rows |
| `dd` | Common | Delete cursor row (deleting in Sections also removes all references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down |
| `b` | Common | Rewrite header Key / BPM with single-line input |
| `Shift+P` `Space` | Common | Audition the section on the cursor row (stops if already playing) |
| `?` | Common | Toggle help (closes with `Esc`) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Redraw the progression of the cursor row, preserving its name |
| `i` | Sections | Edit progression with a single-line MML overlay for Chord Chart |
| `n` | Sections | Edit name with single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end. While typing, the chord at the cursor position will play with auto-voicing each time it forms or changes. Pressing `Ctrl+Space` will play the entire progression with the same voicing. Unreadable input will not be played back as MML, but can still be saved as is. Press `Enter` to confirm and save the progression (trimming leading/trailing spaces), or `Esc` to discard changes.

Inside the editing screen, `Ctrl+T` opens the same timbre list as the regular MML overlay. The timbre doesn't change just by moving the candidate; `Enter` confirms the selection, and `Esc` reverts to the original timbre. The Chord Chart's timbre is global (one for the entire screen) and is saved to `history.json` separately from the regular `Ctrl+P` MML overlay timbre. If you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

Changes are automatically saved. The save location is `clap-mml-render-tui/history/chord_chart.json` within the configuration directory (on Windows, `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If the save file is missing or unreadable, the screen starts with a single section, generated by a one-time draw similar to `g` when first opened (it remains empty if a draw is not possible).

The chord progression catalog used for `g` / `r` draws is fetched from the network. You will wait only on the first load if the cache is empty (the waiting time is logged in `log.txt` under `chord-chart: event=catalog-first-load elapsed_ms=...`). If the data could not be retrieved, 'Chord progression data is unavailable' will appear at the bottom of the screen.

### DAW Screen Effect Chain

In NORMAL mode on the DAW screen, pressing `x` while the cursor is on a playback track opens that track's EFFECT CHAIN overlay. You can insert any number of factory presets from TONE3000 / Surge XT Effects / Dragonfly Reverb in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Open EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor rows) |
| `j` `k` | Move through effect chain stages |
| `a` | Open add overlay. All factory presets from all effects are listed in a single column; select with `j` `k`, press `Enter` to add to the end, `ESC` to return |
| `dd` | Delete the cursor's effect stage |
| `Enter` | Write back to the JSON in the init column and close (the track's cache WAV is re-rendered) |
| `ESC` | Discard changes and close |

- The chain is saved in the `"effects after instrument"` (array order = signal order) within the init column's JSON. Editing the init column directly has the same effect.
- Effects are baked into the cache WAV (applied on the render-server side).
- Each effect's preset is read from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb's presets are embedded within the plugin itself, so they will appear as candidates if the plugin is located at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`. They will not appear if the plugin is not found.
- The chain runs for the duration of the notes, so reverb tails will be cut off at the end of the cell.

In the Guitar Articulation screen (`Ctrl+G` → `E`), pressing `x` also opens the same EFFECT CHAIN overlay. You can insert a guitar amp (`a` → kind `Amp Simulator`, TONE3000) or other effects after METAL-GTX and audition them with articulated playback each time you change the stage. Confirming with `Enter` applies that chain to subsequent `b` and `space` playback. The chain disappears when the application exits.

### Settings

`config.toml` is automatically created on first launch. It is located in the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of TUI / DAW, pressing `e` opens `config.toml` in an editor. Restart the application after closing the editor.

Current configuration example:

```toml
# [REQUIRED] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates for opening config.toml (tried in order from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi and output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/
# within the configuration directory.
# The values below are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by render-server child processes.
# Concurrent execution count (1-16), port, launch command (if empty, executable is searched for)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search in the WAV loop browser
loop_dirs = []

# List of categories that can be assigned to loop directories
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
| `plugins."Surge XT".plugin_path` | OS-specific standard path for Surge XT CLAP | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline render-server processes. |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Command to launch the render-server. If empty, the executable is searched for. |
| `realtime_audio_backend` | `cache_player` | Target for real-time playback (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard directories for Surge XT patches | List of directories to search for Surge XT timbre selection. |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Category overlay keys are determined from unused English letters in the category names. |

OS-specific default `plugin_path` values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

OS-specific default `patches_dirs` values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for rows where no timbre is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog from `[plugins.<name>]` and used in rows where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths being the standard installation locations for each OS.

| Name | plugin_id | patches_dirs | Category by Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | `Presets` under registry (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings`'s `InstallPath`). `patches_dirs` in config is ignored. | Vaporizer2 category names (e.g., `Pad` / `Bass` / `Arpeggio`) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory timbre acquisition destination). `patches_dirs` in config is ignored. | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | `Presets\TyrellN6` under registry (`HKCU\Software\u-he\TyrellN6`'s `DataPath`). `patches_dirs` in config is ignored. | All empty (= no filtering) |

Names are matched case-insensitively, ignoring spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you've installed a plugin in a non-standard location, need to specify a timbre location, or are using a plugin not built-in. **Only the specified items will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line in its table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For a non-built-in plugin, specify everything.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to the plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Location of the plugin's timbres. To clear built-in values, specify `patches_dirs = []`. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by usage. Seven key names can be specified: (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items take effect for that plugin. If not specified, the plugin's default value is used (Surge XT uses category names, others use "no filtering"). |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` will not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default value; others will become candidates in the mixed catalog.
- Dexed timbres are '1 cartridge `.syx` file = 32 programs', so in the list, cartridges are treated as directories, and programs are listed individually like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2 digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`) rather than a timbre property, with a default value of POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 `.vvp` file = 1 timbre, and can be selected similarly to Surge's `.fxp` files. The category appearing in the list header is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe timbre locations are read from the plugin's own settings (Vaporizer2 from `InstallPath\Presets` in the registry, Floe from `extra-presets-folder` in `%PUBLIC%\Floe\Preferences\floe.ini`; if not set in Floe, then `%PUBLIC%\Floe\Presets`). Reinstallation or changes in the plugin's settings do not require rewriting `config.toml`. Specifying `patches_dirs` in `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines factory timbres are embedded within the plugin itself and not present on disk, so during `cmrt build-patch-catalog-cache`, the same version as the installed Six Sines is fetched from GitHub (no communication if already fetched and versions match). Mono/poly is read from the `.sxsnp` content (play mode).
- TyrellN6 timbres are 1 `.h2p` file = 1 timbre, and subfolders under the timbre location (e.g., `01 Basses`) become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the timbre, and all timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2's mono/poly setting differs per timbre and is read from the `.vvp` content (`m_uPolyMode`). Consequently, only timbres that can play chords appear as candidates in the grid sequencer's chord rows (unreadable timbres are not listed as chord row candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. These timbres are designed to expect MPE (per-note pitch and pressure) performance information, which cmrt does not send.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (= all programs are candidates for all rows). This is because Dexed cartridges do not follow a 'directory name = usage' convention, and the timbre organization of non-built-in plugins is unknown. If you wish to change this, specify the 7 items under `[plugins.<name>]` (Surge XT's default values are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them under `[plugins."Surge XT"]`; for other plugins, place them in their respective plugin's table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-based automatic selection is only used for Surge XT timbre determination.
- Rendering result caches are placed in separate directories per plugin, so mixing them will not lead to accidental use of sounds from different plugins (no manual deletion required). The two cache locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins directly; it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, cmrt launches a child process and, in case of a communication error, retries after a single restart. If `offline_render_server_command` is empty, the child process executable is searched for in the order of 'same directory as `cmrt.exe` → release build of sibling repo `clap-mml-play-server`', and **PATH is not checked**.

### Update command

```
cmrt update
```

### Server mode

```
cmrt --server
```

- Interoperates with the bluesky-text-to-audio Chrome extension.
  - When an MML snippet is found in a Bluesky post, it can be played with Surge XT.

### CLI mode

```
cmrt cde
```

- Typing 'cde' will play Do-Re-Mi.

```
cmrt CM7
```

- Typing 'CM7' will play C Major Seventh.
- Also supports various chord progression notations (some are not yet supported).

### Patch Roles command

```
cmrt patch-roles
```

- Displays the number of timbre candidates available for selection via the PATCH wheel for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The screen does not launch.
- Used to check if the wheel becomes unresponsive after changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.).
- If any row has 0 candidates, it will list that row and exit with code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test how changes affect the settings without modifying your current `config.toml`.
- When multiple plugin timbres are listed, the breakdown by plugin will also be displayed for each usage category's candidate count. This is because relying only on the total count might hide cases where a specific plugin's timbre does not appear in a particular row.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### Render MML command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and an audio digest value on a single line. The screen does not launch.
- While `patch-roles` counts whether a timbre appears in the list, this command checks whether that timbre actually produces sound.
- Multiple `--patch` arguments can be specified. The summary line will show 'N / M different outputs', which helps determine if **changing the timbre resulted in the same sound as before**.
- Adding `--out-dir <directory>` writes WAV files (no bytes are written if omitted). Use this when you want to verify by ear.
- Adding `--poly-check` compares chord and monophonic playback to determine if the timbre can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- Obtaining Surge XT patches via API is the proper approach, so that will be implemented (currently, they are inefficiently searched via toml specification. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing '1-measure offline rendering',
    - while being subject to constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapidly iterating on edits.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *'Atomic measure' might sound like a physics term, so for now, I'll keep the term 'アトミック小節' (Atomic Measure) without directly translating it.

# Out of Scope
- Effects are essential for editing, so we've decided to put them out of scope and defer them significantly. One reason is that Surge XT's patches include effects (effects are derived from patches).