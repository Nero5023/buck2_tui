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
- **Primary**: Uses `buck2 targets :` command to get accurate target information
- **Enhanced**: Uses `buck2 query -A` for detailed target metadata
- **Fallback**: Manual BUCK file parsing when buck2 commands are unavailable

### Target Information
- Target names and rule types from buck2 commands
- Dependency information when available
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
- **Task types**: `User` (user-initiated), `Preload` (background)
- **Priority levels**: `HIGH`, `NORMAL`, `LOW`

### Scheduler Architecture
- **Two-tier workers**: Micro (quick tasks) + Macro (heavyweight operations)
- **Priority queues**: `async_priority_channel` for task scheduling
- **Shared state**: `Arc<Mutex<Ongoing>>` for thread-safe task tracking
- **Integration**: Available through `App::scheduler()` method

## Cancellation Mechanisms

### 1. Basic Cancellation
```rust
// scheduler.rs:cancel()
pub fn cancel(&self, id: Id) -> bool {
    let mut ongoing = self.ongoing.lock();
    ongoing.all.remove(&id).is_some()
}
```

### 2. Hook System
- Tasks register cleanup functions via `Hooks`
- Supports both sync and async cleanup: `Hook::Sync` | `Hook::Async`
- Runs automatically on task completion or cancellation

### 3. Process Cancellation
```rust
// For background processes
select! {
    _ = cancel.recv() => {
        child.start_kill().ok();
        cancel.close();
        break;
    }
    // ... handle process output
}
```

### 4. CancellationToken
- Used for preview operations and plugins
- Lightweight cancellation for short-lived tasks

## Task Interruption Flow

1. **New task arrival** → Scheduler evaluates priority
2. **Existing task check** → Cancel if lower priority or conflicting
3. **Cleanup execution** → Run registered hooks
4. **Resource deallocation** → Kill processes, close channels
5. **New task dispatch** → Start replacement task

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
- **Multi-level cancellation** - Different strategies for different task types
- **Resource safety** - Proper cleanup prevents leaks
- **Priority-based** - Important tasks can interrupt less critical ones
- **Thread-safe** - Concurrent access handled via Arc<Mutex<>>
- **Async-friendly** - Built on Rust's async ecosystem

