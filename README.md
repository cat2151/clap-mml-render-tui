# clap-mml-render-tui

### Overview
A TUI DAW (of sorts) for MML. Easily enjoy the rich sounds of [Surge XT](https://surge-synthesizer.github.io/) / [Dexed](https://asb2m10.github.io/dexed/) / [Vaporizer2](https://www.vast-dynamics.com/?q=Vaporizer2) / [Floe](https://floe.audio/) / [Sforzando](https://www.plogue.com/products/sforzando.html) / [Six Sines](https://github.com/baconpaul/six-sines) / [TyrellN6](https://u-he.com/products/tyrelln6/) / [TONE3000](https://www.tone3000.com/) / [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/) using MML. Written in Rust.

### Usage

- For playing around with sounds using MML
- For casual installation. Just having Rust is sufficient.

### Tech Stack
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

### Execution

```
cmrt
```

You can play by inputting MML in the TUI screen.

#### Play Server Implementation

The audio is played by a separate process, the play server. Its executable is determined in the following order, and the first one found is used:

1. The full path specified by `--play-server <PATH>` (if the specified path does not exist, it stops with an error without searching further).
2. `clap-mml-realtime-play-server` in the same directory as `cmrt`.
3. The release build of the sibling repository (`../clap-mml-play-server/target/release/`).

The PATH environment variable is not used. Debug build servers are 4-5 times slower at preloading, causing playback to cut off at the beginning of measures.
A warning will appear in the top right corner of the screen when a debug build or an executable of unknown origin is being used.

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
- Effects (can be inserted in series into a track in DAW mode)
  - [TONE3000](https://www.tone3000.com/)
  - Surge XT Effects (included with Surge XT)
  - [Dragonfly Reverb](https://michaelwillis.github.io/dragonfly-reverb/)
- Download screens for each (for those who get lost)
  - Surge XT: Easier to get with winget: [Fastest! How even a cat can install a DAW and audio plugins (up to playing sound with a virtual MIDI keyboard)](https://cat2151.hatenadiary.jp/entry/2026/03/12/225148)
  - [Dexed (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/asb2m10/dexed)
  - [Vaporizer2 (introduction page on studiorack-site)](https://studiorack.github.io/studiorack-site/plugins/vastdynamics/vaporizer2)
  - [Floe download screen](https://floe.audio/download/)
  - [Sforzando download screen](https://www.plogue.com/downloads.html#sforzando)
  - [Six Sines GitHub releases page](https://github.com/baconpaul/six-sines/releases)
  - [TyrellN6 download screen](https://u-he.com/products/tyrelln6/)
  - [TONE3000 download screen](https://www.tone3000.com/plugin/download)
  - [Dragonfly Reverb GitHub releases page](https://github.com/michaelwillis/dragonfly-reverb/releases)

### AI-Generated Documentation
- From now on, sections appended by AI may be difficult to read. I will maintain them occasionally.

### Keyboard Screen

Press the `v` key to move to the keyboard screen.

- ``c d e f g a b` keys: Play C, D, E, F, G, A, B notes.`

### Chord Chart Screen

Press `Ctrl+G` then `C` to move to the chord chart screen.

This screen allows you to view and edit the "structure" of a song's chord progression. Moving the cursor plays the chord progression of that row. Tracing chords one by one within a row using `h` `l` will play only the pointed chord.

- Left pane (Sections): Define named chord progressions that serve as material.
- Right pane (Arrangement): The arrangement of defined sections forms the song. The same section can be arranged multiple times.
- If you fix a section's progression, all references to it in the song will change together.
- The first line of the header shows the `chord2mml` specification (e.g., `Key=C BPM120`) to be placed at the beginning of the song, as is.
- This screen does not interpret progressions or headers. It retains the typed strings as they are.
- Only the section on the cursor line plays (the entire song does not). You can choose the overall timbre for the Chord Chart from the progression editing screen.
- You can preview chords one by one within a row using `h` `l`. The currently pointed chord appears inverted within the progression.
- Only the Key from the header is passed for preview (BPM remains at its default).
- Voicing and octave for preview are always auto-voiced. In Sections, they are determined within the section; in Arrangement, they are determined by the overall song flow, and the same voicing is maintained even in single-chord previews using `h` `l`.

```
┌ Chord Chart ─────────────────────────────────────────────────────────────────┐
│ Key=C BPM120                                                                 │
│┌ Sections ───────────────────────┐┌ Arrangement ────────────────────────────┐│
││> 1 Intro    I-V                 ││  1 Intro    I-V                         ││
││  2 A        I-V-VIm-IV          ││  2 A        I-V-VIm-IV                  ││
││  3 B        IIm-V-I-VIm         ││> 3 A        I-V-VIm-IV                  ││
││                                 ││  4 B        IIm-V-I-VIm                 ││
│└─────────────────────────────────┘└─────────────────────────────────────────┘│
│ q:終了 ?:help                                                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

These are the keybindings. Only `q` and `?` are displayed in the bottom line of the screen. The full list appears on-screen by pressing the `?` key.

| Key | Pane | Action |
|---|---|---|
| `Tab` | Common | Toggle pane (Sections ⇔ Arrangement) |
| `j` `k` `↓` `↑` | Common | Move cursor (row. The chord progression of the new row plays entirely) |
| `h` `l` `←` `→` | Common | Move chord within row (Plays only the pointed chord. Wraps to adjacent rows at row ends) |
| `PgUp` `PgDn` | Common | Move cursor by 10 rows |
| `dd` | Common | Delete cursor row (Deleting in Sections also removes all references in Arrangement) |
| `Alt+↑` `Alt+↓` | Common | Move cursor row up / down |
| `b` | Common | Rewrite header's Key / BPM with single-line input |
| `Shift+P` `Space` | Common | Preview cursor row's section (Stops if already playing) |
| `?` | Common | Open/close help (also closes with `Esc`) |
| `q` | Common | Exit application |
| `g` | Sections | Add a section by drawing from the chord progression catalog |
| `r` | Sections | Redraw the progression of the cursor row, retaining its name |
| `i` | Sections | Edit progression using a single-line MML overlay for Chord Chart |
| `n` | Sections | Edit name with single-line input |
| `1`〜`9` | Arrangement | Insert the section with that number after the cursor |

The editing screen opened by `i` in Sections initializes with the current progression and places the cursor at the end.
While typing, the chord at the cursor position plays with auto-voicing each time it is formed or changed, and `Ctrl+Space` plays the entire progression with
the same voicing. Unreadable input will not be played as alternative MML, but can be saved as is.
`Enter` confirms and saves the progression, stripping leading/trailing whitespace, while `Esc` discards changes.

In the editing screen, `Ctrl+T` opens the same timbre list as the normal MML overlay. Moving through candidates alone will not
change the timbre; `Enter` confirms, `Esc` reverts to the original timbre. The Chord Chart's timbre is global (one for the entire screen), and
is saved to `history.json` separately from the timbre of the regular `Ctrl+P` MML overlay. If you discard progression edits with `Esc` after confirming a timbre,
the confirmed timbre will remain.

It is automatically saved every time you edit. The save location is under the configuration directory at
`clap-mml-render-tui/history/chord_chart.json` (on Windows, it's
`%LOCALAPPDATA%\clap-mml-render-tui\history\chord_chart.json`). Only one song is saved at a time.
If the save file is missing or unreadable, a single draw (same as `g`) is performed when the screen is first opened,
starting with one section (if drawing fails, it remains empty).

The chord progression catalog used for `g` / `r` draws is fetched from the network. Only the first time, when there's no cache yet,
there will be a wait (the wait time is recorded in log.txt under
``chord-chart: event=catalog-first-load elapsed_ms=...`). If it fails to fetch,
"No chord progression data" will appear at the bottom of the screen.

### DAW Screen Effect Chain

In NORMAL mode of the DAW screen, pressing `x` with the cursor on a performance track opens that track's EFFECT CHAIN overlay.
You can insert any number of TONE3000 / Surge XT Effects / Dragonfly Reverb factory presets in series after the instrument (timbre).

| Key | Action |
|---|---|
| `x` | Opens the EFFECT CHAIN overlay for the cursor track (invalid on chord or conductor rows) |
| `j` `k` | Move chain stage |
| `a` | Opens the add overlay. All factory presets for all effects are listed in one column; select with `j` `k`, add to the end with `Enter`, return with `ESC`. |
| `dd` | Delete the cursor stage |
| `Enter` | Writes back to the init column's JSON and closes (the cache WAV for that track is re-rendered) |
| `ESC` | Discard changes and close |

- The chain is saved in the `init` column's JSON as `"effects after instrument"` (array order = signal order). Editing the `init` column directly has the same effect.
- Effects are baked into the cache WAV (applied on the render-server side).
- Each effect's presets are loaded from their built-in default locations (`%ProgramData%\TONE3000\Presets`, `%ProgramData%\Surge XT\fx_presets`). Dragonfly Reverb presets are built into the plugin itself, so if the plugin exists at `C:\Program Files\Common Files\CLAP\dragonfly-reverb\`, it will appear as a candidate. If the plugin is not present, it will not appear as a candidate.
- Since the chain runs only for the duration of the notes, reverb tails are cut off at the end of the cell.

### Configuration

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory:

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. Restart the application after closing the editor.

Here is an example configuration.

```toml
# 【必須】使用する CLAP プラグイン
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# config.toml を開く editor 候補（左から順に試す）
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav は自動的に設定ディレクトリ配下の
# clap-mml-render-tui/phrase/ または clap-mml-render-tui/daw/ に保存されます。
# 以下の値は内部的に使用されます。
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# オフラインレンダリングは render-server 子プロセスで行います。
# 同時実行数（1〜16）・port・起動コマンド（空なら実体を探索）
offline_render_server_workers = 4
offline_render_server_port = 42153
offline_render_server_command = ""

# リアルタイム再生 backend（"cache_player" / "play_server"）
realtime_audio_backend = "cache_player"
realtime_play_server_port = 42154

# 起動時に自動再生するかどうか
# notepad モード: 現在行を即座に再生します。DAW モード: 曲先頭（measure 0）から演奏開始します。
autoplay_on_startup = true

# WAV ループブラウザーの検索対象ディレクトリ一覧
loop_dirs = []

# WAV ループディレクトリへ付与できるカテゴリ一覧
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]

# Surge XT の標準値を変える場合だけ書く
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
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Candidate editors to try in order from left. |
| `input_midi` | `input.mid` | Internal input MIDI file name. |
| `output_midi` | `output.mid` | Internal output MIDI file name. |
| `output_wav` | `output.wav` | Internal output WAV file name. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent offline rendering (render-server) tasks. |
| `offline_render_server_port` | `42153` | Localhost port for the render-server. |
| `offline_render_server_command` | Empty string | Command to launch the render-server. If empty, it searches for the executable. |
| `realtime_audio_backend` | `cache_player` | Real-time playback backend (`cache_player` / `play_server`). |
| `realtime_play_server_port` | `42154` | Localhost port for the play_server. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately on startup. |
| `plugins."Surge XT".patches_dirs` | OS-specific standard Surge XT patches directory | List of directories to search for Surge XT timbre selection. |
| `loop_dirs` | `[]` | List of directories to search in the WAV loop browser. Run `cmrt scan-loops` after changing. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop dirs. The key for the category overlay is determined from an unused English letter in the category name. |

The OS-specific default `plugin_path` values are as follows:

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

The OS-specific default `patches_dirs` values are as follows:

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is not set, `~/.local/share`)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Fixed Default Plugin and Multiple Plugins

The default plugin for lines without a specified timbre is **fixed to Surge XT**. There is no switching via `active_plugin`. Other plugins like Dexed are added to the mixed catalog via `[plugins.<name>]` and used on lines where their timbre is explicitly stated.

The contents of the built-in profiles are as follows, with paths set to the standard installation location for each OS:

| Name | plugin_id | patches_dirs | Category for usage |
| --- | --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above | Surge XT category name |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) | All empty (= no filtering) |
| `Vaporizer2` | `com.vastdynamics.VAST2` | `Presets` under the registry key (`HKLM\SOFTWARE\VAST Dynamics\Vaporizer2\Settings`'s `InstallPath`). `patches_dirs` in config is ignored | Vaporizer2 category name (`Pad` / `Bass` / `Arpeggio` etc.) |
| `Six Sines` | `org.baconpaul.six-sines` | `%LOCALAPPDATA%\clap-mml-render-tui\vendor-patches\six-sines-factory` (factory timbre acquisition path). `patches_dirs` in config is ignored | All empty (= no filtering) |
| `TyrellN6` | `com.u-he.TyrellN6` | `Presets\TyrellN6` under the registry key (`HKCU\Software\u-he\TyrellN6`'s `DataPath`). `patches_dirs` in config is ignored | All empty (= no filtering) |

Names are matched ignoring differences in case, spaces, and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only write `[plugins.<name>]` if you've installed it in a non-standard location, need to supplement the timbre location, or are using a plugin not built-in. **Only the written items override the built-in values**, so if you just want to change Surge XT's path, a single `plugin_path` line within that table is sufficient.

```toml
# パスだけ差し替える。plugin_id と patches_dirs は組み込みの値のまま。
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# 組み込みに無いプラグインは全部書く。
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Timbre location for that plugin. Write `patches_dirs = []` to clear built-in values. |
| `plugins.<name>.<usage>_patch_categories` / `<role>_patch_keywords` | Filtering for automatic patch selection by usage. Seven key names can be written (`chord_patch_categories` / `bass_patch_categories` / `arpeggio_patch_categories` / `drum_patch_categories` / `kick_patch_keywords` / `snare_patch_keywords` / `hihat_patch_keywords`). Only the written items take effect for that plugin. If not written, that plugin's default value (Surge XT uses category names, others use "no filtering") is used. |

- `active_plugin` is deprecated. Also, writing `plugin_path` / `plugin_id` / `patches_dirs` and the 7 usage-specific category items at the top level will result in a configuration error rather than silent ignoring. Please remove `active_plugin` and move other values to `[plugins."Surge XT"]`.
- Adding `[plugins.<name>]` does not change the default plugin. Only a profile with the same name as Surge XT will override the fixed default values; others become candidates in the mixed catalog.
- Dexed timbres are "1 `.syx` cartridge = 32 programs," so in the list, cartridges are treated like directories, and programs are listed individually like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- Dexed's mono/poly is an instance setting (`MonoMode`), not a timbre setting, and its default value is POLY. Therefore, all Dexed timbres are treated as chord-friendly in the grid sequencer's chord rows.
- Vaporizer2 timbres are 1 `.vvp` file = 1 timbre, and can be selected just like Surge's `.fxp` files. The category that appears in the list's heading is the **first two characters of the filename** (e.g., `AR` = `Arpeggio` for `AR Accent Arp.vvp`).
- Vaporizer2 and Floe's timbre locations are read from the plugin's own settings (Vaporizer2 from `InstallPath\Presets` in the registry, Floe from `extra-presets-folder` in `%PUBLIC%\Floe\Preferences\floe.ini`; if unset for Floe, `%PUBLIC%\Floe\Presets`). Reinstallation or changes in the plugin's settings do not require rewriting `config.toml`. `patches_dirs` specified in `[plugins.Vaporizer2]` / `[plugins.Floe]` will be ignored.
- Six Sines factory timbres are embedded in the plugin itself and not on disk, so `cmrt build-patch-catalog-cache` fetches the same version from GitHub as the installed Six Sines (it won't communicate if already fetched and the version is the same). Mono/poly is read from the contents of the `.sxsnp` file (play mode).
- TyrellN6 timbres are 1 `.h2p` file = 1 timbre, and subfolders under the timbre location (e.g., `01 Basses`) become categories in the list. `UserPresets` are not enumerated. Mono/poly is not read from the timbre, and all TyrellN6 timbres are treated as chord-friendly in the grid sequencer's chord rows.

- Vaporizer2's mono/poly differs per timbre and is read from the `.vvp` file's contents (`m_uPolyMode`). Therefore, only timbres that can play chords appear as candidates in the grid sequencer's chord rows (unreadable timbres are not listed as chord row candidates).
- Among Vaporizer2's factory presets, those with `MPE` in their name will not produce sound in cmrt. This is because these timbres are designed with MPE (per-note pitch and pressure) performance information in mind, which cmrt does not send.
- The default category settings for filtering candidates by row usage (chord / bass / arpeggio / drum) **differ per plugin**. Surge XT uses Surge's category names, Vaporizer2 uses Vaporizer2's category names, and Dexed and non-built-in plugins use "no filtering" (meaning all programs are candidates for any row). This is because Dexed cartridges do not use "directory name = usage," and the timbre organization of non-built-in plugins is unknown, so they are not filtered. If you wish to change this, write the 7 items in `[plugins.<name>]` (the default values for Surge XT are included as comments at the end of the generated `config.toml`).
- The 7 usage-specific category items should also only be written within the plugin profile. For Surge XT, place them in `[plugins."Surge XT"]`; for other plugins, place them in the plugin's own table.
- The shared mono/poly determination data (`voicing_shared_source` / `voicing_override_source`) used for automatic selection by usage is only for Surge XT timbre determination.
- Rendering result caches are placed in separate directories per plugin, so even if mixed, sounds from different plugins will not be misused (no need to manually delete them). The two locations are as follows, where `<plugin>` is the filename (without extension) of the resolved `plugin_path` (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\<plugin>\*.wav` (cache for notepad / MML input overlay)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw_cache\<plugin>\*.wav` (DAW track WAV)

All offline rendering is done via the render-server. The TUI side (`cmrt.exe`) does not load CLAP plugins; instead, it sends MML to `127.0.0.1:<offline_render_server_port>/render` and receives WAVs. If connection to the render-server fails, cmrt launches a child process and, on communication error, restarts and retries once. If `offline_render_server_command` is empty, the child process executable is searched for in the order of "same directory as `cmrt.exe` -> release build of sibling repo `clap-mml-play-server`", and **the PATH environment variable is not used**.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works with the bluesky-text-to-audio Chrome extension.
  - When there is MML in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing "cde" plays Do-Re-Mi.

```
cmrt CM7
```

- Typing "CM7" plays a C major seventh.
- Also supports various chord progression notations (some are not yet supported).

### patch-roles Command

```
cmrt patch-roles
```

- For each row in the grid sequencer (chord / bass / arpeggio / 4 drum roles / others),
  this displays how many timbre candidates are available for selection with the PATCH wheel. The screen does not launch.
- After changing plugins, `patches_dirs`, or usage-specific categories (`chord_patch_categories`, etc.),
  use this to check if the "wheel is unresponsive."
- If any row has 0 candidates, it will list that row and exit with code 1.
- Adding `--config <path>` reads that `config.toml`. This allows you to test how changes in settings
  affect things without modifying your current `config.toml`.
- When multiple plugin timbres are listed, the candidate count per usage also shows a breakdown by plugin.
  This is because the total count alone might not reveal that "a specific plugin's timbre is not appearing in a particular row at all."

```
cmrt patch-roles --config C:\tmp\try.toml
```

### render-mml Command

```
cmrt render-mml --patch "AR Accent Arp.vvp"
```

- Offline renders MML with the specified timbre and displays its length, volume (`peak` / `rms`), whether it's silent, and
  an audio digest value on a single line. The screen does not launch.
- While `patch-roles` counts whether timbres appear in a list, this command checks "whether that timbre actually produces sound."
- `--patch` can be specified multiple times. A summary line shows "N / M distinct outputs," which helps you
  determine if the **timbre changed but the sound remained the same**.
- Adding `--out-dir <directory>` writes WAV files (if not specified, no bytes are written).
  Use this when you want to verify by ear.
- Adding `--poly-check` compares playing chords and single notes to determine if the timbre can play chords.
- `--config <path>` is the same as for `patch-roles`.

```
cmrt render-mml --config C:\tmp\try.toml --out-dir C:\tmp\wav --patch "PD Juno Dream Pad.vvp" --poly-check
```

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It makes sense to acquire Surge XT patches via API, so that will be implemented (currently, they are inefficiently searched from toml specifications. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing "offline rendering per measure,"
    - while accepting constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more suitable.
    - *Leaving "アトミック小節" untranslated for now, as "atomic measure" sounds like a physics term.

# Out of Scope
- Effects are essential for editing, so they are intentionally deemed out of scope and pushed far back in priority. One reason for this is that Surge XT's patches encapsulate effects (effects are derived from patches).