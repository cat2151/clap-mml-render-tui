# clap-mml-render-tui

### Overview
A TUI DAW-like application for MML. Easily enjoy the rich sounds of Surge XT / Dexed / Vaporizer2 / Floe / Sforzando with MML. Written in Rust.

### Usage

- For playing around with sounds using MML
- For casual installation. Just having Rust is enough

### Technical Stack
- Plugin host library
  - https://github.com/prokopyl/clack

### Preparation

Please install [Surge XT](https://surge-synthesizer.github.io/).

```
winget install "Surge XT"
```

### Installation

``` 
cargo install --force --git https://github.com/cat2151/clap-mml-render-tui
```

### Run

```
cmrt
```

You can play by entering MML in the TUI screen.

#### The play server

The sound is played by a separate process, the play server. Its executable is determined in the following order, and the first one found is used.

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it will stop with an error without proceeding to search).
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`).

The PATH environment variable is not used. A debug build server is 4-5 times slower in pre-loading, causing playback to break at the beginning of measures. A warning will appear in the top right corner of the screen when a debug build or an executable of unknown origin is being used.

```
cmrt --play-server "X:/projects/clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe"
```

### Supported Audio Plugins
- ※Limited to those available for free without account registration, on Windows, and supporting CLAP.
- Surge XT
- Dexed
- Vaporizer2
- Floe
- Sforzando

### AI Generated Documentation
- The following sections, added by AI, may be difficult to read. We will maintain them occasionally.

### keyboard screen

Press the `v` key to move to the keyboard screen.

- `c d e f g a b` keys: Play the Do-Re-Mi-Fa-Sol-La-Si notes.

### chord chart screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to overview and edit the "structure" of a song's chord progression. Moving the cursor plays the chord progression of that row. Traversing chords one by one within a row with `h` `l` plays only the pointed chord.

- Left pane (Sections): Define named chord progressions as building blocks.
- Right pane (Arrangement): The sequence of defined sections forms the song. The same section can be used multiple times.
- If you fix a section's progression, all references to it within the song will change simultaneously.
- The header's single line directly displays the `chord2mml` specification (e.g., `Key=C BPM120`) placed at the beginning of the song.
- Neither the progression nor the header is interpreted by this screen. It simply stores the entered string as-is.
- Only the single section on the cursor's row plays (the entire song does not play). You can select the overall timbre for the Chord Chart from the progression editing screen.
- You can audition chords one by one within a row using `h` `l`. The currently pointed chord is highlighted within the progression.
- Only the Key in the header is passed for auditioning (BPM remains at its default).

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

These are the key bindings. Only `q` and `?` are displayed in the bottom line of the screen. The full list appears on screen by pressing the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Move pane (toggle Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. The chord progression of the new row plays entirely.) |
| `h` `l` `←` `→` | Common | Move chord within row (Plays only the pointed chord. Wraps to adjacent rows at row ends.) |
| `PgUp` `PgDn` | Common | Move cursor by 10 rows. |
| `dd` | Common | Delete cursor row (Deleting in Sections also deletes its references in Arrangement.) |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down. |
| `b` | Common | Rewrite header's Key / BPM via single-line input. |
| `Shift+P` `Space` | Common | Audition the section on the cursor row (stops if already playing). |
| `?` | Common | Toggle help (also closes with `Esc`). |
| `q` | Common | Exit application. |
| `g` | Sections | Add a section by drawing from the chord progression catalog. |
| `r` | Sections | Redraw the progression of the cursor row, preserving its name. |
| `i` | Sections | Edit progression using a single-line MML overlay for Chord Chart. |
| `n` | Sections | Edit name via single-line input. |
| `1`〜`9` | Arrangement | Insert the section of that number after the cursor.

The editing screen opened with `i` in Sections initializes with the current progression and places the cursor at the end. While typing, the chord at the cursor position plays immediately as it forms or changes, and `Ctrl+Space` plays the entire progression. Unreadable input will not be played as alternative MML but can be saved as is. Pressing `Enter` confirms and saves the progression, stripping leading/trailing spaces; `Esc` discards changes.

Inside the editing screen, `Ctrl+T` opens the same timbre list as the regular MML overlay. Moving through candidates does not change the timbre; `Enter` confirms it, and `Esc` reverts to the original timbre. The Chord Chart's timbre is global to the screen and saved to `history.json` separately from the timbre of the regular `Ctrl+P` MML overlay. Even if you discard progression edits with `Esc` after confirming a timbre, the confirmed timbre remains.

It is automatically saved every time you edit. The save location is `clap-mml-render-tui/history/chord_chart.json` under the configuration directory (on Windows, `%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time. If no save file exists or it's unreadable, when the screen is first opened, it performs a single draw (like `g`) to start with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. You'll only wait on the first run when the cache isn't present yet (wait time is logged in `log.txt` under `chord-chart: event=catalog-first-load elapsed_ms=...`). If it fails to fetch, 'No chord progression data' will appear at the bottom of the screen.

### Configuration

`config.toml` is automatically created on the first launch. It is located under the OS's standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is an example of the current configuration.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried in order from left)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi   = "input.mid"

# output_midi, output_wav are automatically saved to
# clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/ under the configuration directory.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent offline rendering tasks for DAW (1-16)
offline_render_workers = 2

# Offline rendering backend
# in_process: Renders within the cmrt main process.
# render_server: Renders by POSTing to /render on a render-server child process.
offline_render_backend = "in_process"
offline_render_server_workers = 4
offline_render_server_port = 62153
offline_render_server_command = ""

# Real-time playback backend
realtime_audio_backend = "in_process"
realtime_play_server_port = 62154

# Whether to autoplay on startup
# notepad mode: immediately plays the current line. DAW mode: starts playback from song beginning (measure 0).
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
| `plugins."Surge XT".plugin_path` | OS-specific Surge XT CLAP standard path | Path for Surge XT if installed in a non-standard location. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried in order from left. |
| `input_midi` | `input.mid` | Input MIDI file name for internal processing. |
| `output_midi` | `output.mid` | Output MIDI file name for internal processing. |
| `output_wav` | `output.wav` | Output WAV file name for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent in-process rendering tasks. |
| `offline_render_backend` | `in_process` | Destination for offline rendering execution. |
| `offline_render_server_workers` | `4` | Number of concurrent render_server tasks. |
| `offline_render_server_port` | `62153` | localhost port for render_server. |
| `offline_render_server_command` | Empty string | Command to launch render_server. |
| `realtime_audio_backend` | `in_process` | Destination for real-time audio playback. |
| `realtime_play_server_port` | `62154` | localhost port for play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific Surge XT patches standard directories | List of directories to search for Surge XT timbre selection. |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories that can be assigned to loop dirs. The key for the category overlay is determined from an unused English letter in the category name. |

Default `plugin_path` values by OS are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

Default `patches_dirs` values by OS are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, then `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without explicit timbre settings is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to a mixed catalog via `[plugins.<name>]` and used for lines where the timbre is explicitly specified.

The contents of the built-in profiles are as follows, with paths pointing to the OS-specific standard installation locations:

| Name | `plugin_id` | `patches_dirs` | Category by Usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values in table above | Surge XT category names |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | **No default value. Please specify `patches_dirs`** | Vaporizer2 category names (e.g., `Pad` / `Bass` / `Arpeggio`) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you've installed plugins in a non-standard location, need to supplement timbre directories, or are using plugins not built-in. **Only specified items will overwrite built-in values**, so if you only want to change Surge XT's path, a single `plugin_path` line within its table is sufficient.

```toml
# Only override the path. plugin_id and patches_dirs remain the built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For plugins not built-in, specify all relevant fields.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<Name>.plugin_path` | Path to that plugin. |
| `plugins.<Name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<Name>.patches_dirs` | Timbre directory for that plugin. To remove built-in values, write `patches_dirs = []`. |
| `plugins.<Name>.<Usage>_patch_categories` / `<Role>_patch_keywords` | Filtering for automated patch selection by usage. Seven key names can be used (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only specified items will take effect for that plugin. If not specified, the default value for that plugin will be used (Surge XT uses category names, others use 'no filtering'). |

- `active_plugin` has been deprecated. Writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific categories at the top level will result in a configuration error, rather than being silently ignored. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only profiles with the same name as Surge XT will override fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are structured as 'one cartridge `.syx` file = 32 programs', so in the list, cartridges are treated as directories, displaying programs one by one like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digits, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly setting is an instance configuration (`MonoMode`), not part of the timbre, and its default is POLY. Therefore, all Dexed timbres are treated as chord-suitable in the grid sequencer's chord rows.
- Vaporizer2 timbres are one `.vvp` file = one timbre, selectable just like Surge's `.fxp` files. The category displayed in the list's heading is the **first two characters of the filename** (e.g., for `AR Accent Arp.vvp`, `AR` = `Arpeggio`).
- Only Vaporizer2 does not have a default `patches_dirs` value. This is because the preset location is an environment-dependent value determined by the plugin's global settings (e.g., `%APPDATA%\Vaporizer2\VASTvaporizerSettings.xml`), and `cmrt` reading/writing to it unilaterally could damage your DAW environment. Please add a single line as shown below. Until you do, it will not appear in the catalog as having 0 timbres.

```toml
[plugins.Vaporizer2]
patches_dirs = ['D:\Vaporizer2\Presets']
```

- Vaporizer2's mono/poly varies per timbre and is read from the contents of the `.vvp` file (`m_uPolyMode`). Therefore, only timbres that play chords will appear as candidates in the grid sequencer's chord rows (unreadable timbres will not be shown as candidates for chord rows).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in `cmrt`. These timbres are designed to use MPE (per-note pitch and pressure) performance information, which `cmrt` does not send.
- The default category settings for filtering candidates by line usage (chord / bass / arpeggio / 4 drum roles / others) **differ for each plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use 'no filtering' (= all programs are candidates for all lines). This is because Dexed cartridges do not follow a 'directory name = usage' convention, and for non-built-in plugins, the timbre storage system is unknown, so no filtering is applied. If you want to change it, add the 7 items to `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific categories should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in that plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by usage is only used for Surge XT's timbre determination.
- Rendering results are cached in separate directories per plugin, so even if mixed, sounds from different plugins are not misused (no manual deletion required). The two cache locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"`, the TUI side does not directly load CLAP plugins; instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAV data. If connecting to the render-server fails, `cmrt` will launch a child process and, in case of a communication error, restart and retry once.

### update command

```
cmrt update
```

### server mode

```
cmrt --server
```

- Works with the bluesky-text-to-audio Chrome extension.
- When MML is found in a Bluesky post, it can be played with Surge XT.

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

### patch-roles command

```
cmrt patch-roles
```

- Displays how many timbre candidates are available for selection using the wheel in the PATCH column for each row of the grid sequencer (chord / bass / arpeggio / 4 drum roles / others). No screen is launched.
- Use this to check if the wheel has become 'unresponsive' after changing plugins, `patches_dirs`, or usage-specific categories (e.g., `chord_patch_categories`).
- If any row has 0 candidates, it will list that row and exit with exit code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test the effects of configuration changes without modifying your current `config.toml`.
- When multiple plugin timbres are listed, the candidate count for each usage will also show a breakdown by plugin. This is because relying solely on the total might not reveal if 'a specific plugin has no timbres available for that row'.

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and a digest of the sound output, all on a single line. No screen is launched.
- `patch-roles` counts whether a timbre appears in the list, while this command checks whether that timbre actually produces sound.
- `--patch` can be specified multiple times. A summary line showing "N / M different outputs" will appear, allowing you to check if **changing the timbre didn't accidentally keep the previous sound**.
- Adding `--out-dir <directory>` writes WAV files (otherwise, no bytes are written). Use this when you want to audition the output.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre plays chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Breaking changes are made frequently on a daily basis.

# Future Plans
- It is proper to acquire Surge XT patches via API, so this will be implemented (currently, they are discovered from paths specified in `toml`, which is inefficient. Implementation timing is deferred; other priorities come first).

# Concept Notes
- Atomic Measures
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering in 1-measure units,"
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more advanced editing, existing feature-rich DAWs would be more appropriate.
    - ※As 'atomic measure' leans into a physics term, for now, 'アトミック小節' will be retained without direct English translation.

# Out of Scope
- Effects are essential for editing, so we've decided to consider them out of scope and defer them significantly. Another reason is that in Surge XT, patches inherently include effects (effects are derived from patches).