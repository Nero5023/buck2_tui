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

The application scans for `BUCK` and `BUCK2` files in the project directory and parses them to extract target information. Currently supports basic target parsing with plans for enhanced Buck2 query integration.

## File Structure

```
src/
├── main.rs          # Entry point and CLI setup
├── app.rs           # Main application and terminal management
├── buck.rs          # Buck2 project handling and target parsing
├── ui.rs            # UI rendering and layout management
└── events.rs        # Event handling and user input
```

## Future Enhancements

Planned features include:
- Buck2 query integration for advanced target information
- Build progress monitoring
- Error debugging interface
- Target dependency visualization