# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Buck TUI is a terminal user interface for Buck2 that provides a yazi-inspired workflow for navigating build targets, monitoring build progress, and debugging errors interactively.

## Core Architecture

The application follows a modular Rust architecture with async support:

- **`src/main.rs`**: Entry point with CLI argument parsing using clap
- **`src/app.rs`**: Main application loop and terminal management using ratatui + crossterm
- **`src/buck.rs`**: Buck2 project integration, target parsing, and data management
- **`src/ui.rs`**: UI rendering with three-pane layout (directories, targets, details)
- **`src/events.rs`**: Event handling for keyboard navigation and search functionality

## Key Features

### Three-Pane Layout
- **Left Pane**: Directory tree showing folders with BUCK files
- **Middle Pane**: Target list for selected directory with fuzzy search
- **Right Pane**: Detailed information about selected target

### Navigation Controls
- `h/j/k/l` or arrow keys: Navigate between panes and within lists
- `Tab`: Cycle through panes
- `/`: Enter fuzzy search mode
- `Enter`: Select directory/target or enter details view
- `q` or `Esc`: Exit application or search mode

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

## Buck2 Integration

The application provides comprehensive Buck2 integration:

### Directory Scanning
- Scans all directories in the project (not just those with Buck files)
- Identifies directories containing `BUCK` or `TARGETS` files with 📦 icon
- Regular directories shown with 📁 icon for full navigation capability

### Target Discovery
- **Optimized**: Uses `buck2 targets : -A` command to get comprehensive target information in a single query
- **Comprehensive**: Retrieves target names, rule types, dependencies, visibility, and metadata in one request
- **Fallback**: Manual BUCK file parsing when buck2 commands are unavailable

### Target Information
- **Comprehensive details**: Target names, rule types, and target paths
- **Package metadata**: Package information, oncall team details, and platform targets  
- **Dependency analysis**: Complete dependency lists with smart truncation for readability
- **Visibility rules**: Target visibility configuration and access controls
- **Technical details**: Build platform information, loading status, and language detection
- **Rich formatting**: Organized sections with visual indicators and color coding
- Graceful degradation when buck2 is not installed or accessible

## File Structure

```
src/
├── main.rs          # Entry point and CLI setup
├── app.rs           # Main application and terminal management
├── buck.rs          # Buck2 project handling and target parsing
├── ui.rs            # UI rendering and layout management
├── events.rs        # Event handling and user input
└── scheduler/       # Task scheduling and management
    ├── mod.rs       # Module exports
    ├── task.rs      # Task structure and lifecycle
    ├── hooks.rs     # Cleanup hook system
    └── scheduler.rs # Main scheduler implementation
```

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

## Buck2 Integration

The scheduler system is specifically designed to handle Buck2 commands efficiently:

### Target Loading
```rust
// Create task to load targets from a directory
let task = Task::new(
    Priority::Normal,
    vec!["buck2".to_owned(), "targets".to_owned(), ":".to_owned()],
    dir_path.clone(),
    success_callback,
);
```

### Target Details
```rust
// Create task to query target details
let task = Task::new(
    Priority::Normal,
    vec!["buck2".to_owned(), "query".to_owned(), "-A".to_owned(), target_label],
    dir_path.clone(),
    details_callback,
);
```

### Key Features
- **Automatic cancellation** - Previous Buck2 commands are cancelled when new ones are needed
- **Proper stdio handling** - Buck2 output doesn't interfere with the TUI
- **Async callbacks** - Results are processed asynchronously and update the UI
- **Error handling** - Failed commands are handled gracefully with fallback behavior

