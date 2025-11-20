# Buck2 TUI

A terminal user interface for Buck2 that provides a yazi-inspired workflow for navigating build targets, monitoring build progress, and debugging errors interactively.

## Features

- 🗂️ **Five-pane layout** for efficient navigation
- 🔍 **Interactive search** with real-time highlighting
- 📦 **Buck2 integration** with target discovery and metadata
- ⌨️ **Vim-style keybindings** for fast navigation
- 🎨 **Syntax highlighting** for different target types

## Installation

### Prerequisites

- Rust 1.70 or higher
- Buck2 installed and available in PATH

### Building from Source

```bash
git clone <repository-url>
cd buck2_tui
cargo build --release
```

The binary will be available at `target/release/buck_tui`.


### Install by cargo
```bash
cargo install --git https://github.com/Nero5023/buck2_tui
```

## Usage

### Basic Usage

Run in the current directory:
```bash
buck_tui
```

Run in a specific Buck2 project:
```bash
buck_tui --path /path/to/buck2/project
```

### Interface Overview

Buck2 TUI uses a five-pane layout with a path bar at the top:

```
~/current/directory/path                                    ← Path bar
┌─────────────────────────────────────────────────────────┐
│ Parent │ Current  │ ┌─────────────┐ │ Details            │
│  Dir   │   Dir    │ │   Targets   │ │                    │
│ (20%)  │  (25%)   │ │    (60%)    │ │ (25%)              │
│        │          │ ├─────────────┤ │                    │
│        │          │ │  Selected   │ │                    │
│        │          │ │ Directory   │ │                    │
│        │          │ │    (40%)    │ │                    │
│        │          │ │(display only)│ │                    │
└─────────────────────────────────────────────────────────┘
```

### Pane Descriptions

#### 1. **Path Bar** (Top)
- Shows the current directory in `~/path/format`
- Updates when you navigate between directories
- Color: Yellow text

#### 2. **Parent Directory** (Left, 20%)
- Shows sibling directories of the current directory
- Helps you understand your position in the directory tree
- Each directory shows:
  - 📁 Icon for regular directories
  - 📦 Icon for directories with BUCK/TARGETS files
  - Target count: `(5)` or `loading...` or `—` (not loaded)

#### 3. **Current Directory** (Center-Left, 25%)
- Shows subdirectories of the current directory
- **This is your main navigation pane**
- Selected directory is highlighted in blue
- Same icons and target counts as Parent Directory pane

#### 4. **Targets Pane** (Center, Top 60%)
- Shows Buck2 targets in the selected directory
- Only visible if the directory has a BUCK or TARGETS file
- Each target shows:
  - Language-specific icon (🦀 for Rust, 🐍 for Python, etc.)
  - Target name
  - Colored by language
- Selected target is highlighted in blue
- Supports fuzzy search with `/`

#### 5. **Selected Directory** (Center, Bottom 40%)
- **Display-only pane** (cannot be focused)
- Shows contents of the directory you've selected in Current Directory
- Helps preview what's inside before entering
- Useful for quick exploration without changing directories

#### 6. **Details Pane** (Right, 25%)
- Shows detailed information about the selected target
- Includes:
  - **Target Information**: Full label, rule type
  - **Package Info**: Package name, oncall contact
  - **Visibility**: Access rules (first 5 shown)
  - **Dependencies**: Target dependencies (first 10 shown)
  - **Platform**: Build platform details
  - **Technical Details**: Execution platform, configuration

### Navigation Modes

The TUI has two navigation groups that you can switch between with `Tab`:

#### **Explorer Group** (Parent Directory + Current Directory)
- Navigate directory structure
- Select directories to view their targets
- Use `h/l` to go up/down the directory tree
- Use `j/k` to select different directories

#### **Inspector Group** (Targets + Details)
- Browse and inspect Buck2 targets
- Navigate between targets and their details
- Use `h/l` to switch between targets list and details
- Use `j/k` to select different targets

## Keybindings

### Global Keys

| Key | Action |
|-----|--------|
| `q` | Quit application (except in search mode) |
| `Ctrl+C` | Force quit application |
| `Esc` | Quit (or exit search mode if searching) |
| `Tab` | Switch between Explorer and Inspector groups |

### Navigation Keys

| Key | Action |
|-----|--------|
| `h` or `←` | Go to parent directory (Explorer) / Move left (Inspector) |
| `j` or `↓` | Move down in current pane |
| `k` or `↑` | Move up in current pane |
| `l` or `→` | Enter selected directory (Explorer) / Move right (Inspector) |
| `Enter` | Enter directory or view details |

### Search Keys

| Key | Action |
|-----|--------|
| `/` | Start search (in current pane) |
| `n` | Jump to next match |
| `N` | Jump to previous match |
| `Enter` | Close search popup (keep highlights) |
| `Esc` | Close search and clear highlights |
| `Backspace` | Delete character from search query |

### Target Actions

| Key | Action |
|-----|--------|
| `a` | Open actions menu (build/test) |
| `o` | Open target definition file in editor |

## Search Feature

The search feature allows you to quickly find directories or targets:

### How to Use Search

1. **Start Search**: Press `/`
   - A centered popup appears: `Find next: ____`
   - Previous search query is preserved

2. **Type to Search**:
   - As you type, matching items are highlighted in yellow
   - Current match has yellow background
   - Other matches have yellow text + underline
   - Counter shows position: `3/7` (3rd of 7 matches)

3. **Navigate Matches**:
   - Press `n` to jump to next match
   - Press `N` to jump to previous match
   - Matches wrap around (after last → first)

4. **Exit Search**:
   - `Enter`: Close popup, keep highlights (can still use `n`/`N`)
   - `Esc`: Close popup and clear all highlights

### Search Behavior

- **Scope**: Searches only the currently focused pane
  - In Current Directory pane → searches directory names
  - In Targets pane → searches target names

- **Case-insensitive**: "buck" matches "Buck", "BUCK", "buck2"

- **Smart positioning**: Starts from current selection
  - If current item matches → highlights it
  - Otherwise → jumps to next match after cursor
  - No match after cursor → wraps to first match

- **Persistent across directories**:
  - Search query is preserved when changing directories
  - Matches are recalculated for new directory contents
  - `n`/`N` navigation updates automatically


## Directory Indicators

- **📁** Regular directory (no Buck files)
- **📦** Directory with BUCK or TARGETS file
- **(5)** Number of targets in directory
- **loading...** Targets are being loaded from Buck2
- **—** Directory not yet loaded

## Target Language Icons

Targets are displayed with language-specific icons:

- 🦀 Rust (`rust_*` rules)
- 🐍 Python (`python_*` rules)
- ⚡ C/C++ (`cxx_*` rules)
- ☕ Java (`java_*` rules)
- 📜 Shell scripts (`sh_*` rules)
- 📦 Generic targets



## Acknowledgments

- Inspired by [yazi](https://github.com/sxyazi/yazi) for the TUI design
- Built with [ratatui](https://github.com/ratatui-org/ratatui) for terminal UI
- Uses [Buck2](https://buck2.build/) for build system integration
