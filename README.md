# clap-mml-render-tui

### Overview
An MML TUI DAW (of sorts). Easily enjoy the rich sounds of Surge XT with MML. Written in Rust.

### Usage

- For playing around with MML sounds.
- For casual installation. Rust is the only prerequisite.

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

You can play around by entering MML in the TUI screen.

### Keyboard Screen

Press the `v` key to switch to the keyboard screen.

- `c d e f g a b` keys: Play C D E F G A B (Do Re Mi Fa Sol La Si).

### Configuration

`config.toml` is automatically created on first launch. Its location is under the OS standard configuration directory.

- Windows: `%LOCALAPPDATA%\clap-mml-render-tui\config.toml`
- Linux: `~/.config/clap-mml-render-tui/config.toml`
- macOS: `~/Library/Application Support/clap-mml-render-tui/config.toml`

In TUI / DAW NORMAL mode, pressing `e` opens `config.toml` in an editor. After closing the editor, restart the application.

Here is a current configuration example.

```toml
# [Required] CLAP plugin to use
plugin_path = 'C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap'

# Editor candidates to open config.toml (tried from left to right)
editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

input_midi  = "input.mid"

# output_midi, output_wav are automatically saved under the config directory
# in clap-mml-render-tui/phrase/ or clap-mml-render-tui/daw/.
# The following values are used internally.
output_midi = "output.mid"
output_wav  = "output.wav"

sample_rate = 48000
buffer_size = 512

# Number of concurrent DAW offline renders (1-16)
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
realtime_play_server_command = ""

# Whether to autoplay on startup
# Notepad mode: Plays the current line immediately. DAW mode: Starts playback from the beginning of the song (measure 0).
autoplay_on_startup = true

# List of directories to search for Surge XT patches
patches_dirs = [
  'C:\ProgramData\Surge XT\patches_factory',
  'C:\ProgramData\Surge XT\patches_3rdparty',
]

# List of directories to search for WAV loops in the loop browser
loop_dirs = []

# List of categories that can be assigned to loop directories
loop_categories = ["guitar", "drum", "bass", "spoken", "sequence"]
```

Configuration items are as follows.

| Item | Default Value | Description |
| --- | --- | --- |
| `plugin_path` | OS-specific Surge XT CLAP standard path | Path to the CLAP plugin to use. |
| `editors` | `["fresh", "zed", "code", "edit", "nano", "vim"]` | Editor candidates, tried from left to right. |
| `input_midi` | `input.mid` | Input MIDI filename for internal processing. |
| `output_midi` | `output.mid` | Output MIDI filename for internal processing. |
| `output_wav` | `output.wav` | Output WAV filename for internal processing. |
| `sample_rate` | `48000` | Sample rate for rendering. |
| `buffer_size` | `512` | Buffer size for rendering. |
| `offline_render_workers` | `2` | Number of concurrent `in_process` renders. |
| `offline_render_backend` | `in_process` | Destination for offline rendering. |
| `offline_render_server_workers` | `4` | Number of concurrent `render_server` instances. |
| `offline_render_server_port` | `62153` | Localhost port for `render_server`. |
| `offline_render_server_command` | Empty string | Startup command for `render_server`. |
| `realtime_audio_backend` | `in_process` | Destination for real-time playback. |
| `realtime_play_server_port` | `62154` | Localhost port for `play_server`. |
| `realtime_play_server_command` | Empty string | Startup command for `play_server`. |
| `autoplay_on_startup` | `true` | Whether to autoplay immediately after startup. |
| `patches_dirs` | OS-specific Surge XT patches standard directory | List of directories to search for sound patches. |
| `loop_dirs` | `[]` | List of directories to search for in the WAV loop browser. After changing, run `cmrt scan-loops`. |
| `loop_categories` | `["guitar", "drum", "bass", "spoken", "sequence"]` | List of categories to assign to loop directories. Keys for category overlay are determined from unused English letters within the category names. |

Default `plugin_path` values by OS are as follows.

- Windows: `C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap`
- Linux: `/usr/lib/clap/Surge XT.clap`
- macOS: `/Library/Audio/Plug-Ins/CLAP/Surge XT.clap`

Default `patches_dirs` values by OS are as follows.

- Windows: `C:\ProgramData\Surge XT\patches_factory`, `C:\ProgramData\Surge XT\patches_3rdparty`
- Linux: `$XDG_DATA_HOME/surge-data/patches_factory`, `$XDG_DATA_HOME/surge-data/patches_3rdparty` (if `XDG_DATA_HOME` is unset, `~/.local/share` is used)
- macOS: `/Library/Application Support/Surge XT/patches_factory`, `/Library/Application Support/Surge XT/patches_3rdparty`

#### Using multiple plugins

You can switch with a single line for `active_plugin`. `Surge XT` and `Dexed` are **built-in**, so if they are installed in their standard locations, you only need to write this one line.

```toml
active_plugin = 'Dexed'
```

The contents of the built-in profiles are as follows, with paths being the standard installation locations for each OS.

| Name | plugin_id | patches_dirs |
| --- | --- | --- |
| `Surge XT` | `org.surge-synth-team.surge-xt` | OS-specific default values from the table above |
| `Dexed` | `com.digital-suburban.dexed` | Dexed cartridge location (Windows: `%APPDATA%\DigitalSuburban\Dexed\Cartridges`) |

Names are matched case-insensitively and ignoring differences in spaces and underscores (`Dexed` / `dexed`, `Surge XT` / `surge_xt` / `SurgeXT` are all treated as the same).

You only need to write `[plugins.<name>]` if you have installed a plugin in a non-standard location or are using a plugin not built-in. **Only the specified items will override the built-in values**, so if you just want to change the path, a single `plugin_path` line is sufficient.

```toml
active_plugin = 'Surge XT'

# Just replace the path. plugin_id and patches_dirs remain built-in values.
[plugins."Surge XT"]
plugin_path = 'D:\my\clap\Surge XT.clap'

# For non-built-in plugins, specify all details.
[plugins.my_synth]
plugin_path  = 'D:\my\clap\MySynth.clap'
patches_dirs = ['D:\my\patches']
```

| Item | Description |
| --- | --- |
| `active_plugin` | Name of the profile to use. Specify either a built-in name or a `[plugins.*]` name. If omitted, the top-level `plugin_path` / `patches_dirs` will be used as is. |
| `plugins.<name>.plugin_path` | Path to that plugin. |
| `plugins.<name>.plugin_id` | Expected CLAP plugin ID. Can be omitted. |
| `plugins.<name>.patches_dirs` | Patch location for that plugin. To clear built-in values, write `patches_dirs = []`. |

- If `active_plugin` is specified, the top-level `plugin_path` / `patches_dirs` will not be used (no error, the profile takes precedence).
- If the `active_plugin` name is not found in either built-in profiles or `[plugins.*]`, the application will fail to start with an error. All available names from both sources will be displayed.
- Dexed sounds are organized as '1 cartridge `.syx` file = 32 programs', so in the list, cartridges are treated as directories, and programs are listed individually like `SynprezFM/SynprezFM_01.syx/00 Say Again.` (numbers are 2-digit, starting from 0). If you specify the cartridge location in `patches_dirs`, you can select them just like Surge's `.fxp` files.
- For Dexed, automatic sound selection based on line purpose (grid sequencer's chord / bass / drum lines) does not work. This is because Dexed's mono/poly is an instance setting, not a sound characteristic, so it cannot determine if a sound is suitable for chords. Manual sound selection works as usual.
- When switching plugins, please manually clear the rendering cache. **Only for lines where no sound patch is specified** (lines without `{"Surge XT patch": ...}` at the beginning of the MML), the cache key remains the same before and after the switch, causing the sound from the previous plugin to play. Lines with a specified sound patch include the patch name in the key, so they won't be mixed. The two locations to clear are (for Windows):
  - `%LOCALAPPDATA%\clap-mml-render-tui\notepad_cache\*.wav` (Notepad / MML input overlay cache)
  - `%LOCALAPPDATA%\clap-mml-render-tui\daw\*.wav` (DAW track WAV)

If `offline_render_backend = "render_server"` is set, the TUI will not directly load CLAP plugins, but instead send MML to `127.0.0.1:<offline_render_server_port>/render` and receive WAV data. If the connection to the render-server fails, cmrt will launch a child process, and in case of a communication error, it will restart and retry once.

### Update Command

```
cmrt update
```

### Server Mode

```
cmrt --server
```

- Works with the bluesky-text-to-audio Chrome extension.
- When an MML is found in a Bluesky post, it can be played with Surge XT.

### CLI Mode

```
cmrt cde
```

- Typing `cde` will play C-D-E (Do-Re-Mi).

```
cmrt CM7
```

- Typing `CM7` will play a C major seventh chord.
- It supports various chord progression notations (some are not yet supported).

# Breaking Changes
- Frequent breaking changes are made daily.

# Future Plans
- It is logical to retrieve Surge XT patches via an API, so this will be implemented (currently, they are inefficiently searched via toml. Implementation timing is deferred, prioritizing other features).

# Concept Notes
- Atomic Measure
    - Inspired by Obsidian's atomic notes.
    - By making the unit of all processing 'offline rendering in 1-measure units,'
    - while incurring some constraints,
    - various benefits can be gained.
    - This is suitable for sketching and rapid editing cycles.
    - For more serious editing, existing feature-rich DAWs would be more appropriate.
    - Note: 'atomic measure' tends to sound like a physics term, so for now, the Japanese original opted to keep 'アトミック小節' without directly translating it.

# Out of Scope
- Effects are deemed outside the scope and postponed significantly, as they require dedicated editing. One reason for this is that in Surge XT, patches already encapsulate effects (effects are derived from patches).