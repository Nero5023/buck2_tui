# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Buck TUI is a terminal user interface for Buck2 that provides a yazi-inspired workflow for navigating build targets, monitoring build progress, and debugging errors interactively.

## Core Architecture

The application follows a modular Rust architecture with async support:

- **`src/main.rs`**: Entry point with CLI argument parsing using clap
- **`src/app.rs`**: Main application loop and terminal management using ratatui + crossterm
- **`src/buck.rs`**: Buck2 project integration, target parsing, and data management
- **`src/ui.rs`**: UI rendering with five-pane layout (directories, targets, details, selected directory)
- **`src/events.rs`**: Event handling for keyboard navigation and search functionality

## Key Features

### Five-Pane Layout with Path Bar
- **Path Bar**: Clean yellow text showing current directory (~/path/format) at the top
- **Left Pane**: Parent directory tree showing sibling folders
- **Second Pane**: Current directory contents with target counts and Buck file indicators
- **Third Pane**: Split vertically into:
  - **Top**: Target list for selected directory with fuzzy search and language icons
  - **Bottom**: Selected directory contents (display-only, non-focusable)
- **Right Pane**: Detailed information about selected target

### Navigation Controls
- `h/j/k/l` or arrow keys: Navigate between panes and within lists
- `Tab`: Cycle between Explorer (directories) and Inspector (targets/details) groups
- `/`: Enter fuzzy search mode
- `Enter`: Select directory/target or enter details view
- `q` or `Esc`: Exit application or search mode

### State Indicators
- **Target counts**: Shows actual counts for loaded directories, "loading..." for directories being processed, "—" for unloaded directories
- **Buck indicators**: 📦 for directories with Buck files, 📁 for regular directories
- **Language icons**: Nerd font icons with colors for different target types (Rust, Python, C++, etc.)

## Development Commands

### Build and Run
```bash
cargo build                # Build the application
cargo run                  # Run with current directory
cargo run -- --path /path # Run with specific project path
```

### Development
```bash
cargo check                # Fast compile check
cargo clippy              # Linting
cargo fmt                 # Format code
```

### Testing
```bash
cargo test                # Run tests (when implemented)
```

## Dependencies

The project uses these key libraries:
- **ratatui**: Terminal UI framework
- **crossterm**: Cross-platform terminal handling
- **tokio**: Async runtime
- **clap**: CLI argument parsing
- **walkdir**: Directory traversal
- **fuzzy-matcher**: Fuzzy search functionality
- **serde**: Serialization for Buck file parsing
- **dirs**: Home directory detection for path display
- **nerd-font-symbols**: Language-specific icons for targets

## Buck2 Integration

The application provides comprehensive Buck2 integration with performance optimizations:

### Directory Scanning
- Scans all directories in the project (not just those with Buck files)
- Identifies directories containing `BUCK` or `TARGETS` files with 📦 icon
- Regular directories shown with 📁 icon for full navigation capability
- Uses absolute paths consistently for reliable parent directory navigation

### Target Discovery (Optimized)
- **Single Query Optimization**: Uses `buck2 targets : -A` to get comprehensive target information in one command
- **Enhanced Metadata**: Retrieves target names, rule types, dependencies, package info, oncall, visibility, and platform details
- **Fallback Support**: Manual BUCK file parsing when buck2 commands are unavailable

### Target Information Display
- **Organized Sections**: Target Information, Visibility (truncated), Dependencies (limited), Technical Details
- **Rich Metadata**: Full target labels, package info, oncall contacts, platform details
- **Smart Truncation**: Shows first 5 visibility rules and 10 dependencies with "... and X more" indicators

## File Structure

```
src/
├── main.rs          # Entry point and CLI setup
├── app.rs           # Main application and terminal management
├── buck.rs          # Buck2 project handling and target parsing
├── ui.rs            # UI rendering and layout management (5-pane + path bar)
├── events.rs        # Event handling and user input
└── scheduler/       # Task scheduling and management
    ├── mod.rs       # Module exports
    ├── task.rs      # Task structure and lifecycle
    ├── hooks.rs     # Cleanup hook system
    └── scheduler.rs # Main scheduler implementation
```

## UI Layout Structure

```
~/current/directory/path                                    ← Path bar (yellow)
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

### Navigation Flow
- **Explorer Group**: Parent Directory ↔ Current Directory
- **Inspector Group**: Targets ↔ Details
- **Selected Directory**: Display-only pane showing contents of selected directory
- **Vertical Navigation**: Within targets list (loops normally)
- **Tab**: Switches between Explorer and Inspector groups

## Future Enhancements

Planned features include:
- Buck2 query integration for advanced target information
- Build progress monitoring
- Error debugging interface
- Target dependency visualization

## Scheduler System

The Buck TUI includes a comprehensive task scheduling system inspired by Yazi's scheduler architecture. This system enables asynchronous task management with priority-based execution, cancellation mechanisms, and proper resource cleanup.

### Task Structure
- **Task stages**: `Pending` → `Dispatched` → `Hooked`
- **Priority levels**: `LOW`, `NORMAL`, `HIGH`
- **Command-based**: Tasks execute external commands (like `buck2`) with proper stdio handling
- **Cancellation**: Each task has its own `CancellationToken` for responsive cancellation

### Scheduler Architecture
- **Command execution**: Tasks run external commands with captured stdout/stderr
- **Priority queues**: `async_priority_channel` for task scheduling
- **Shared state**: `Arc<Mutex<Ongoing>>` for thread-safe task tracking
- **Integration**: Available through `App::scheduler()` method
- **Stdio isolation**: All external commands use piped stdio to prevent TUI corruption

## Cancellation Mechanisms

### 1. Task-Level Cancellation
```rust
// Each task has its own cancellation token
pub struct Task {
    pub cancel_token: CancellationToken,
    // ... other fields
}

pub fn cancel(&self, id: TaskId) -> bool {
    // Cancel the task's token to stop work immediately
    task.cancel();
}
```

### 2. Process Cancellation
```rust
// Commands are killed when tasks are cancelled
tokio::select! {
    _ = cancel_token.cancelled() => {
        let _ = child.kill().await;
        return Ok(());
    }
    result = async { /* command execution */ } => {
        result
    }
}
```

### 3. Hook System
- Tasks register cleanup functions via `Hooks`
- Runs automatically on task completion or cancellation
- Supports both sync and async cleanup operations

### 4. Stdio Isolation
- All external commands use `stdin(null)`, `stdout(piped)`, `stderr(piped)`
- Prevents command output from corrupting the TUI display
- Commands are properly isolated from the terminal interface

## Task Execution Flow

1. **Task creation** → Create task with commands, current directory, and success callback
2. **Task dispatch** → Add to priority queue based on task priority
3. **Command execution** → Spawn external process with isolated stdio
4. **Cancellation monitoring** → Use `tokio::select!` to monitor cancellation token
5. **Result processing** → Run success callback with command output
6. **Cleanup** → Execute cleanup hooks and remove from tracking

## Implementation Files

**Buck TUI Scheduler Implementation**
- `src/scheduler/scheduler.rs` - Core scheduler logic with priority queues
- `src/scheduler/task.rs` - Task structure, lifecycle, and priority management
- `src/scheduler/hooks.rs` - Cleanup system with sync/async hooks
- `src/app.rs` - Scheduler integration into main application

**Reference Implementation (Yazi)**
- `yazi-scheduler/src/scheduler.rs` - Core scheduler logic
- `yazi-scheduler/src/task.rs` - Task structure and states
- `yazi-scheduler/src/hooks.rs` - Cleanup system
- `yazi-actor/src/tasks/cancel.rs` - Actor cancellation handler

## Design Benefits
- **Command-based architecture** - Clean separation between task logic and command execution
- **Responsive cancellation** - Tasks can be cancelled immediately with process termination
- **TUI protection** - Stdio isolation prevents external commands from corrupting the display
- **Resource safety** - Proper cleanup prevents leaks and zombie processes
- **Priority-based** - Important tasks can interrupt less critical ones
- **Thread-safe** - Concurrent access handled via `Arc<Mutex<>>`
- **Async-friendly** - Built on Rust's async ecosystem with proper cancellation support

## Buck2 Integration with Scheduler

The scheduler system is specifically designed to handle Buck2 commands efficiently:

### Target Loading (Optimized)
```rust
// Create task to load targets from a directory using comprehensive query
let task = Task::new(
    Priority::Normal,
    vec!["buck2".to_owned(), "targets".to_owned(), ":".to_owned(), "-A".to_owned()],
    dir_path.clone(),
    success_callback,
);
```

### Key Features
- **Single Query Optimization** - Uses `buck2 targets : -A` instead of N+1 queries
- **Automatic cancellation** - Previous Buck2 commands are cancelled when new ones are needed
- **Proper stdio handling** - Buck2 output doesn't interfere with the TUI
- **Async callbacks** - Results are processed asynchronously and update the UI
- **Error handling** - Failed commands are handled gracefully with fallback behavior

## Recent Optimizations

### Performance Improvements
- **Eliminated N+1 queries**: Changed from `buck2 targets :` + individual `buck2 uquery target -A` calls to single `buck2 targets : -A` command
- **Enhanced data structure**: BuckTarget now includes package, oncall, visibility, and platform information from initial query
- **Removed target detail loading**: Eliminated entire async target detail loading subsystem (~150 lines of code)

### UI Enhancements
- **Path bar**: Clean yellow text display at top showing current directory in ~/path format
- **Better state indicators**: Shows "—" for unloaded directories instead of misleading "0"
- **Comprehensive target details**: Organized sections with smart truncation for visibility and dependencies
- **Absolute path navigation**: Consistent use of absolute paths for reliable parent directory navigation

### Layout Improvements
- **Five-pane design**: Added Selected Directory pane under targets for context
- **Display-only pane**: Selected Directory shows contents but cannot be focused
- **Vertical split**: Targets column split 60/40 between targets list and directory contents
- **Space efficient**: Optimized percentages for better use of terminal space

# important-instruction-reminders
Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.