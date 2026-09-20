# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around and making sounds with MML
- For casual installation. Rust is all you need.

### Tech Stack
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

You can input MML and play around in the TUI screen.

#### `play server` Details

Sound is produced by a separate `play server` process. The executable is determined in the following order, and the first one found is used:

1.  The full path specified by `--play-server <PATH>` (if the specified path does not exist, it stops with an error without proceeding to search).
2.  `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3.  The release build of the sibling repository (`../clap-mml-play-server/target/release/`).

PATH is not consulted. The look-ahead for debug build servers is 4-5 times slower, causing playback to cut off at the beginning of measures.
A warning will appear in the top-right corner of the screen if you are using a debug build or an executable of unknown origin.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- ※Limited to CLAP, Windows, and freely available without account registration.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando
- Effect (can be inserted in series on a DAW track)
  - TONE3000
  - Surge XT Effects

### AI-Generated Documentation
- The AI-added parts below may be difficult to read. I will occasionally maintain them.

### Keyboard Screen

Press the `v` key to navigate to the keyboard screen.

- `c d e f g a b` keys: Play Do, Re, Mi, Fa, Sol, La, Si.

### Chord Chart Screen

Press `Ctrl+G` then `C` to navigate to the chord chart screen.

This screen is for overviewing and editing the "structure" of chord progressions for an entire song. Moving the cursor will play the chord progression of that line.
If you traverse chords one by one within a line using `h` `l`, only the pointed chord will play.

- Left pane (Sections): Define chord progressions as building blocks with names.
- Right pane (Arrangement): The sequence of defined sections becomes a song. The same section can be arranged multiple times.
- If you modify a section's progression, all references to it in the song will change accordingly.
- The first line of the header displays the `chord2mml` specification for the beginning of the song (e.g., `Key=C BPM120`) as-is.
- Neither progressions nor headers are interpreted by this screen; it retains the entered string exactly as is.
- Only the section on the cursor line will play (not the entire song). You can choose the global patch for the Chord Chart from the progression editing screen.
- You can audition chords one by one within a line using `h` `l`. The currently pointed chord appears inverted within the progression.
- Only the Key from the header is passed for auditioning (BPM remains at its default).
- Inversions and octaves for auditioning are always auto-voiced. In Sections, this is determined within the section; in Arrangement, it's determined by the overall song sequence, and `h` `l` individual auditions maintain the same inversion.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:Quit ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

Keybindings. Only `q` and `?` are shown in the bottom line of the screen. The full list appears on-screen when you press the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Move pane (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (line. The chord progression of the new line will play entirely) |
| `h` `l` `←` `→` | Common | Move chord within line (only the pointed chord will play. Moves to the next line at the end of a line) |
| `PgUp` `PgDn` | Common | Move cursor 10 lines |
| `dd` | Common | Delete current line (deleting in Sections also deletes its references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move current line up / down |
| `b` | Common | Rewrite Header Key / BPM with single line input |
| `Shift+P` `Space` | Common | Audition current section (stops if already playing) |
| `?` | Common | Toggle help (`Esc` also closes it) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Re-draw the progression of the current line, preserving its name |
| `i` | Sections | Edit progression with a single-line MML overlay for Chord Chart |
| `n` | Sections | Edit name with single line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end.
While inputting, the chord at the cursor position will play with auto-voicing whenever it forms or changes. Pressing `Ctrl+Space` will play the entire progression with the same voicing. Unreadable input will not be played back as MML but can still be saved. Pressing `Enter` confirms and saves the progression, removing leading/trailing spaces, while `Esc` discards changes.

Inside the editing screen, `Ctrl+T` opens the patch list, similar to the regular MML overlay. Moving through candidates will not change the patch until `Enter` is pressed to confirm, or `Esc` to revert to the original patch. The Chord Chart's patch is global for the entire screen and is saved to `history.json` separately from the regular `Ctrl+P` MML overlay's patch. If you discard progression edits with `Esc` after confirming a patch, the confirmed patch will remain.

Changes are auto-saved. The save location is `clap-mml-render-tui/history/chord_chart.json` under your configuration directory (on Windows, this is `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If the save file is missing or unreadable, when the screen is first opened, a single draw operation (like pressing `g`) is performed to start with one section (it remains empty if a draw is not possible).

The chord progression catalog used for `g` / `r` draws is fetched from the network. Only the first time, when the cache is not yet available, there will be a wait (the waiting time is recorded in `log.txt` as `chord-chart: event=catalog-first-load elapsed_ms=...`). If the data cannot be fetched, "コード進行データがありません" (Chord progression data is not available) will appear at the bottom of the screen.

### DAW Screen Effect Chain

In NORMAL mode on the DAW screen, placing the cursor on a playback track and pressing `x` opens the EFFECT CHAIN overlay for that track.
After the instrument (patch), you can insert any number of factory presets from TONE3000 / Surge XT Effects in series.

| Key | Action |
|---|---|
| `x` | Open EFFECT CHAIN overlay for the current track (invalid for chord or conductor lines) |
| `j` `k` | Move chain stage |
| `a` | Open add overlay. All factory presets for all effects are listed in a single column; select with `j` `k`, add to end with `Enter`, return with `ESC`. |
| `dd` | Delete current stage |
| `Enter` | Write back to the init column's JSON and close (the track's cached WAV will be re-rendered) |
| `ESC` | Discard changes and close |

- The chain is saved in the `init` column's JSON under `"effects after instrument"` (array order = signal order). Editing the `init` column directly achieves the same result.
- Effects are baked into the cache WAV (applied on the render-server side).
- Each effect's preset is loaded from its built-in default location (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). If a plugin is not available, it won't appear as a candidate.
- The chain runs for the same duration as the note, so reverb tails are cut off at the end of the cell.

### Settings

A `config.toml` file is automatically created on first launch. The location is under the OS's standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In NORMAL mode of the TUI / DAW, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example configuration:

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi and output_wav are automatically saved under
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ in the config directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Offline rendering is performed by the render-server child process.
# Number of concurrent workers (1-16) / port / startup command (empty means search for executable)
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# Real-time playback backend ("cache_player" / "play_server")
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# Whether to autoplay on startup
# notepad mode: immediately plays the current line. DAW mode: starts playback from song beginning (measure 0).
autoplay_on_startup = true

# List of directories to search for in the WAV loop browser
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
|---|---|---|
| `plugins."Surge XT".plugin_path` | OS-specific standard Surge XT CLAP path | Path if Surge XT is installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates to try in order from left to right. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline render-server workers. |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Render-server startup command. If empty, the executable is searched for. |
| `realtime_audio_backend` | `cache_player` | Real-time playback backend (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directory | List of directories to search for Surge XT patches. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. Run `cmrt scan-loops` after making changes. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories that can be assigned to loop directories. The key for the category overlay is determined by unused English letters in the category name. |

The default `plugin_path` for each OS is as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The default `patches_dirs` for each OS is as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines where no patch is specified is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<Name>]` and used on lines where the patch is explicitly specified.

The built-in profiles are as follows, with paths set to the standard installation location for each OS:

| Name | plugin_id | patches_dirs | Usage Category |
|---|---|---|---|
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default. Please specify `patches_dirs`** | Vaporizer2 category names (`Pad` / `Bass` / `Arpeggio`, etc.) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to define `[plugins.<Name>]` if you've installed a plugin in a non-standard location, want to supplement its patch directory, or are using a plugin not built-in. **Only the items you write will override built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For a plugin not built-in, define everything.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
|---|---|
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Patch directory for that plugin. To clear built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filtering for automatic patch selection by usage. You can specify 7 key names (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the specified items will take effect for that plugin. If not specified, the plugin's default value (Surge XT uses category names, others use "no filtering") will be used. |

- `active_plugin` is deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error rather than silent ignoring. Please delete `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<Name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default value; others become candidates in the mixed catalog.
- Dexed patches are '1 cartridge `.syx` file = 32 programs', so in the list, cartridges are treated like directories, and programs are listed individually as `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 0-indexed, two digits). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not a patch property, and its default is POLY. Therefore, all Dexed patches are treated as chord-friendly in the grid sequencer's chord lines.
- Vaporizer2 patches are 1 `.vvp` file = 1 patch, selectable like Surge's `.fxp` files. The category appearing in the list's heading is determined by **the first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 is the only one that doesn't have a default `patches_dirs`. This is because its preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` arbitrarily reading or writing there could damage your DAW environment. Please add a single line as shown below. Until you do, it will appear as 0 patches in the catalog.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly setting varies per patch and is read from the `.vvp` content (`m_uPolyMode`). Therefore, only patches that play chords will appear as candidates in the grid sequencer's chord lines (unreadable patches are excluded from chord line candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These patches assume MPE (per-note pitch and pressure) performance information, which `cmrt` does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for all lines). This is because Dexed cartridges do not use "directory name = usage," and the patch directory structure for non-built-in plugins is unknown, so no filtering is applied. If you wish to change this, add the 7 items to `[plugins.<Name>]` (Surge XT's default values are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific categories should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in that plugin's table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for usage-based auto-selection is only applied to Surge XT patch determination.
- Rendering result caches are stored in separate directories for each plugin, preventing accidental use of patches from different plugins even when mixed (no manual deletion is needed). The two locations are as follows, where `<Plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<Plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<Plugin>\*.wav` (DAW track WAVs)

All offline rendering is done via the render-server. The TUI (`cmrt.exe`) does not load CLAP plugins directly; instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If the connection to the render-server fails, `cmrt` will launch a child process and, in case of communication errors, retry once after restarting it. If `offline_render_server_command` is empty, the child process executable is searched for in the following order: 'same directory as `cmrt.exe` -> release build of sibling repo `clap-mml-play-server`', and **PATH is not consulted**.

### update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Interacts with the bluesky-text-to-audio Chrome extension.
  - When an MML is found in a Bluesky post, it can be played back with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play Do, Re, Mi.

```
cmrt CM7
```

- Typing `CM7` will play C major seventh.
- It supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- Displays how many patch candidates are available for selection with the wheel in the PATCH column for each line in the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). The screen does not launch.
- Use this after changing plugins, `patches_dirs`, or usage categories (`chord_patch_categories`, etc.), to check if the 'wheel is unresponsive'.
- If any line has 0 candidates, it will list that line and exit with exit code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test how changes to settings would affect behavior without modifying your current `config.toml`.
- When patches from multiple plugins are listed, the breakdown by plugin will also be shown for each usage category's candidate count. This is because relying solely on the total count might not reveal that 'a certain plugin has no patches available for that line'.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified patch and displays length, volume (`peak` / `rms`), whether it's silent, and an audio digest value on a single line. The screen does not launch.
- While `patch-roles` counts whether a patch appears in the list, this command checks whether that patch actually produces sound.
- You can list any number of `--patch` arguments. A summary line showing 'N / M different outputs' will indicate if **changing the patch resulted in the same sound as before**.
- Adding `--out-dir <directory>` writes WAV files (no bytes are written if omitted). Use this when you want to audition the sound.
- Adding `--poly-check` compares chord and monophonic playback to determine if the patch can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Breaking changes are made frequently, on a daily basis.

# Future Plans
- It makes sense to obtain Surge XT patches via an API, so that will be implemented (currently, they are inefficiently searched for based on toml specifications). Implementation timing is deferred, prioritizing other features.

# Concept Notes
- アトミック小節 (Atomic Measures)
    - This concept is inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while this imposes constraints,
    - it offers various benefits.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing high-functional DAWs would be more appropriate.
    - *(Note: The term 'atomic measure' might be confused with a physics term, so for now, the Japanese 'アトミック小節' is used without direct translation.)*

# Out of Scope
- Since effects are essential for editing, they are deliberately considered out of scope and postponed significantly. One reason for this is that in Surge XT, patches inherently contain effects (effects are carved out from patches).