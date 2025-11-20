use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::app::SearchState;
use crate::buck::BuckProject;
use crate::buck::BuckTarget;

pub struct UI {
    pub current_pane: Pane,
    pub current_group: PaneGroup,
    parent_list_state: ListState,
    current_list_state: ListState,
    targets_list_state: ListState,
    actions_list_state: ListState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pane {
    ParentDirectory,
    CurrentDirectory,
    SelectedDirectory,
    Targets,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaneGroup {
    Explorer,  // Parent + Current directory panes
    Inspector, // Targets + Details panes
}

impl UI {
    pub fn new() -> Self {
        Self {
            current_pane: Pane::CurrentDirectory,
            current_group: PaneGroup::Explorer,
            parent_list_state: ListState::default(),
            current_list_state: ListState::default(),
            targets_list_state: ListState::default(),
            actions_list_state: ListState::default(),
        }
    }

    pub fn draw(&mut self, f: &mut Frame, project: &BuckProject, search_state: &SearchState) {
        // Split main area into top path bar and main content
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Path bar (no border, just text)
                Constraint::Min(0),    // Main content
            ])
            .split(f.area());

        // Draw path bar at the top
        self.draw_path_bar(f, main_chunks[0], project);

        // Split main content into four horizontal panes
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // Parent directory
                Constraint::Percentage(25), // Current directory
                Constraint::Percentage(30), // Target list + Selected directory (vertical split)
                Constraint::Percentage(25), // Target details
            ])
            .split(main_chunks[1]);

        // Split the targets column vertically to add selected directory underneath
        let targets_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Targets pane
                Constraint::Percentage(40), // Selected directory pane
            ])
            .split(content_chunks[2]);

        self.draw_parent_directory(f, content_chunks[0], project);
        self.draw_current_directory(f, content_chunks[1], project, search_state);
        self.draw_targets(f, targets_chunks[0], project, search_state);
        self.draw_selected_directory(f, targets_chunks[1], project);
        self.draw_details(f, content_chunks[3], project);

        // Draw search popup if active
        if search_state.active {
            self.draw_search_popup(f, search_state);
        }
    }

    fn draw_parent_directory(&mut self, f: &mut Frame, area: Rect, project: &BuckProject) {
        let parent_dirs = project.get_parent_directories();

        let directories: Vec<ListItem> = parent_dirs
            .iter()
            .enumerate()
            .map(|(idx, dir)| {
                let is_current = dir.path == project.current_path;
                let style = if is_current {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };

                let display_path = dir
                    .path
                    .file_name()
                    .unwrap_or_else(|| dir.path.as_os_str())
                    .to_string_lossy();

                let buck_indicator = if dir.has_buck_file { "📦" } else { "📁" };
                let text = format!("{} {}", buck_indicator, display_path);

                // Update list state to select current directory
                if is_current {
                    self.parent_list_state.select(Some(idx));
                }

                ListItem::new(text).style(style)
            })
            .collect();

        let block_style = if self.current_pane == Pane::ParentDirectory {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let title = format!(
            "Parent: {}",
            project
                .current_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| "Root".into())
        );

        let directories_list = List::new(directories)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(block_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(directories_list, area, &mut self.parent_list_state);
    }

    fn draw_current_directory(&mut self, f: &mut Frame, area: Rect, project: &BuckProject, search_state: &SearchState) {
        let current_dirs = project.get_current_directories();

        // Check if we should highlight matches in this pane
        // Highlight as long as there's a search query, even if popup is closed
        let should_highlight = !search_state.query.is_empty()
            && matches!(search_state.searching_in_pane, crate::app::SearchPane::CurrentDirectory);

        let directories: Vec<ListItem> = current_dirs
            .sub_directories
            .iter()
            .enumerate()
            .map(|(idx, dir)| {
                let is_selected = dir.path == project.selected_directory;
                let style = if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };

                let display_path = if dir.path == project.current_path {
                    ".".to_string()
                } else {
                    dir.path
                        .file_name()
                        .unwrap_or_else(|| dir.path.as_os_str())
                        .to_string_lossy()
                        .to_string()
                };

                let target_count = if let Some(project_dir) = project.directories.get(&dir.path) {
                    if project_dir.targets_loading {
                        "loading...".to_string()
                    } else {
                        project_dir.targets.len().to_string()
                    }
                } else if dir.targets_loading {
                    "loading...".to_string()
                } else if dir.has_buck_file {
                    "—".to_string() // Not loaded yet but has Buck files
                } else {
                    "—".to_string() // Not loaded and no Buck files
                };
                let buck_indicator = if dir.has_buck_file { "📦" } else { "📁" };

                // Update list state to select the selected directory
                if is_selected {
                    self.current_list_state.select(Some(idx));
                }

                // Determine if this is the current match
                let is_current_match = should_highlight
                    && search_state.matches.get(search_state.current_match_idx) == Some(&idx);

                // Create the item with highlighting if needed
                let item = if should_highlight && display_path.to_lowercase().contains(&search_state.query.to_lowercase()) {
                    // Use highlight_matches for the directory name
                    let mut spans = vec![Span::raw(format!("{} ", buck_indicator))];
                    spans.extend(Self::highlight_matches(&display_path, &search_state.query, is_current_match));
                    spans.push(Span::raw(format!(" ({})", target_count)));
                    ListItem::new(Line::from(spans)).style(style)
                } else {
                    // No highlighting, use plain text
                    let text = format!("{} {} ({})", buck_indicator, display_path, target_count);
                    ListItem::new(text).style(style)
                };

                item
            })
            .collect();

        let block_style = if self.current_pane == Pane::CurrentDirectory {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let title = format!(
            "Current: {}",
            project
                .current_path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| ".".into())
        );

        let directories_list = List::new(directories)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(block_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(directories_list, area, &mut self.current_list_state);
    }

    fn draw_selected_directory(&self, f: &mut Frame, area: Rect, project: &BuckProject) {
        // Get contents of the selected directory from current directory pane
        let selected_dirs = if project.selected_directory != project.current_path {
            // Show contents of the selected directory
            let selected_ui_dir = crate::buck::UICurrentDirectory::new(&project.selected_directory);
            selected_ui_dir.sub_directories
        } else {
            // If current directory is selected, show empty or current contents
            Vec::new()
        };

        let directories: Vec<ListItem> = selected_dirs
            .iter()
            .map(|dir| {
                let display_path = if dir.path == project.selected_directory {
                    ".".to_string()
                } else {
                    dir.path
                        .file_name()
                        .unwrap_or_else(|| dir.path.as_os_str())
                        .to_string_lossy()
                        .to_string()
                };

                let target_count = if let Some(project_dir) = project.directories.get(&dir.path) {
                    if project_dir.targets_loading {
                        "loading...".to_string()
                    } else {
                        project_dir.targets.len().to_string()
                    }
                } else if dir.targets_loading {
                    "loading...".to_string()
                } else if dir.has_buck_file {
                    "—".to_string() // Not loaded yet but has Buck files
                } else {
                    "—".to_string() // Not loaded and no Buck files
                };

                let buck_indicator = if dir.has_buck_file { "📦" } else { "📁" };
                let text = format!("{} {} ({})", buck_indicator, display_path, target_count);

                ListItem::new(text)
            })
            .collect();

        // Selected Directory pane is never focused, so always use default style
        let block_style = Style::default();

        let directories_list = List::new(directories)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(block_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(directories_list, area);
    }

    fn draw_targets(&mut self, f: &mut Frame, area: Rect, project: &BuckProject, search_state: &SearchState) {
        // Check if we should highlight matches in this pane
        // Highlight as long as there's a search query, even if popup is closed
        let should_highlight = !search_state.query.is_empty()
            && matches!(search_state.searching_in_pane, crate::app::SearchPane::Targets);

        let targets: Vec<ListItem> = if let Some(selected_dir) = project.get_selected_directory() {
            if selected_dir.targets_loading {
                vec![ListItem::new("Loading targets...").style(Style::default().fg(Color::Yellow))]
            } else {
                project
                    .filtered_targets
                    .iter()
                    .enumerate()
                    .map(|(i, target)| {
                        let style = if i == project.selected_target {
                            Style::default().bg(Color::Blue).fg(Color::White)
                        } else {
                            Style::default()
                        };

                        let (icon, color) = target.get_language_icon();
                        let icon_span = Span::styled(
                            icon,
                            Style::default().fg(Color::from_u32(
                                u32::from_str_radix(&color[1..], 16).unwrap_or(0x888888),
                            )),
                        );

                        let target_name = target.display_title();

                        // Determine if this is the current match
                        let is_current_match = should_highlight
                            && search_state.matches.get(search_state.current_match_idx) == Some(&i);

                        // Create the line with highlighting if needed
                        let text = if should_highlight && target_name.to_lowercase().contains(&search_state.query.to_lowercase()) {
                            let mut spans = vec![
                                Span::raw(" "),
                                icon_span,
                                Span::raw(" "),
                            ];
                            spans.extend(Self::highlight_matches(&target_name, &search_state.query, is_current_match));
                            Line::from(spans)
                        } else {
                            Line::from(vec![
                                Span::raw(" "),
                                icon_span,
                                Span::raw(" "),
                                Span::raw(target_name),
                            ])
                        };

                        ListItem::new(text).style(style)
                    })
                    .collect()
            }
        } else {
            vec![]
        };

        // Update list state to track selected target
        self.targets_list_state.select(Some(project.selected_target));

        let block_style = if self.current_pane == Pane::Targets {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let package_name = project
            .get_selected_buck_package_name()
            .map(|name| format!("{}: ", name))
            .unwrap_or("No package selected".to_string());

        // TODO: use package path like fbcode//buck2/app:
        let title = format!("Targets ({})", package_name);

        let targets_list = List::new(targets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(block_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(targets_list, area, &mut self.targets_list_state);
    }

    fn draw_details(&self, f: &mut Frame, area: Rect, project: &BuckProject) {
        let block_style = if self.current_pane == Pane::Details {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let details_text = if let Some(target) = project.get_selected_target() {
            self.format_target_details(target)
        } else {
            vec![Line::from("No target selected")]
        };

        let details = Paragraph::new(details_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Details")
                    .border_style(block_style),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(details, area);
    }

    fn format_target_details<'a>(&self, target: &'a BuckTarget) -> Vec<Line<'a>> {
        let mut lines = vec![];

        // Basic Information Section
        lines.push(Line::from(vec![Span::styled(
            "▶ Target Information",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(&target.full_target_label_name),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Rule Type: ", Style::default().fg(Color::Cyan)),
            Span::raw(&target.rule_type),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Target Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(&target.name),
        ]));

        // Package Information
        if let Some(package) = &target.package {
            lines.push(Line::from(vec![
                Span::styled("Package: ", Style::default().fg(Color::Cyan)),
                Span::raw(package),
            ]));
        }

        // Oncall Information
        if let Some(oncall) = &target.oncall {
            lines.push(Line::from(vec![
                Span::styled("Oncall: ", Style::default().fg(Color::Cyan)),
                Span::styled(oncall, Style::default().fg(Color::Yellow)),
            ]));
        }

        // Platform Information
        if let Some(platform) = &target.default_target_platform {
            lines.push(Line::from(vec![
                Span::styled("Default Platform: ", Style::default().fg(Color::Cyan)),
                Span::raw(platform),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(""));

        // Visibility Section
        if !target.visibility.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "▶ Visibility",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            for (i, visibility) in target.visibility.iter().enumerate() {
                if i < 5 {
                    // Show first 5 visibility rules
                    lines.push(Line::from(vec![Span::raw("  • "), Span::raw(visibility)]));
                } else if i == 5 {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("... and {} more", target.visibility.len() - 5),
                            Style::default().fg(Color::Gray),
                        ),
                    ]));
                    break;
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        }

        // Dependencies Section
        if !target.deps.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                format!("▶ Dependencies ({})", target.deps.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            for (i, dep) in target.deps.iter().enumerate() {
                if i < 10 {
                    // Show first 10 dependencies
                    lines.push(Line::from(vec![Span::raw("  • "), Span::raw(dep)]));
                } else if i == 10 {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("... and {} more", target.deps.len() - 10),
                            Style::default().fg(Color::Gray),
                        ),
                    ]));
                    break;
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "▶ Dependencies",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("No dependencies", Style::default().fg(Color::Gray)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        }

        // Technical Details Section
        lines.push(Line::from(vec![Span::styled(
            "▶ Technical Details",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Cyan)),
            Span::raw(target.path.display().to_string()),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Details Loaded: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                if target.details_loaded { "✓" } else { "✗" },
                if target.details_loaded {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]));

        // Language/Icon information
        let (icon, _) = target.get_language_icon();
        if !icon.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Language Icon: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} ({})", icon, target.get_rule_language())),
            ]));
        }

        lines
    }

    fn draw_search_popup(&self, f: &mut Frame, search_state: &SearchState) {
        // Create a compact search popup (smaller than before - just one line height)
        // Use centered position but with minimal vertical space
        let popup_width = 40;  // Fixed width in columns
        let popup_height = 3;   // 3 lines: top border, content, bottom border

        // Calculate horizontal centering
        let area = f.area();
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area
        f.render_widget(Clear, popup_area);

        // Build the search text with counter
        let counter_text = if search_state.total_matches > 0 {
            format!(" {}/{}", search_state.current_match_idx + 1, search_state.total_matches)
        } else {
            String::new()
        };

        // Calculate available width for query (leaving space for "Find next: " and counter)
        let prefix = "Find next: ";
        let available_width = popup_width.saturating_sub(4) as usize; // 4 for borders and padding
        let counter_len = counter_text.len();
        let prefix_len = prefix.len();
        let query_max_len = available_width.saturating_sub(prefix_len).saturating_sub(counter_len);

        // Truncate query if too long
        let display_query = if search_state.query.len() > query_max_len {
            format!("{}...", &search_state.query[..query_max_len.saturating_sub(3)])
        } else {
            search_state.query.clone()
        };

        // Build the content line
        let mut spans = vec![
            Span::raw(prefix),
            Span::styled(&display_query, Style::default().fg(Color::Yellow)),
        ];

        // Add counter on the right if there are matches
        if !counter_text.is_empty() {
            // Calculate padding to right-align the counter
            let content_len = prefix_len + display_query.len();
            let padding_len = available_width.saturating_sub(content_len).saturating_sub(counter_len);
            if padding_len > 0 {
                spans.push(Span::raw(" ".repeat(padding_len)));
            }
            spans.push(Span::styled(&counter_text, Style::default().fg(Color::Cyan)));
        }

        let search_text = vec![Line::from(spans)];

        let search_popup = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        f.render_widget(search_popup, popup_area);
    }

    /// Helper function to highlight matching text in search results
    /// Returns a vector of Spans with matches underlined and optionally highlighted
    /// Note: Returns owned Spans to avoid lifetime issues
    fn highlight_matches(text: &str, query: &str, is_current_match: bool) -> Vec<Span<'static>> {
        if query.is_empty() {
            return vec![Span::raw(text.to_string())];
        }

        let mut spans = Vec::new();
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();
        let mut last_end = 0;

        // Find all occurrences of the query in the text
        for (idx, _) in text_lower.match_indices(&query_lower) {
            // Add text before the match
            if idx > last_end {
                spans.push(Span::raw(text[last_end..idx].to_string()));
            }

            // Add the matched text with underline and optional background
            let match_text = text[idx..idx + query.len()].to_string();
            let match_style = if is_current_match {
                // Current match: yellow background + underline + black text
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .bg(Color::Yellow)
                    .fg(Color::Black)
            } else {
                // Other matches: yellow text + underline
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow)
            };
            spans.push(Span::styled(match_text, match_style));

            last_end = idx + query.len();
        }

        // Add remaining text after the last match
        if last_end < text.len() {
            spans.push(Span::raw(text[last_end..].to_string()));
        }

        // If no matches were found, just return the original text
        if spans.is_empty() {
            vec![Span::raw(text.to_string())]
        } else {
            spans
        }
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn draw_path_bar(&self, f: &mut Frame, area: Rect, project: &BuckProject) {
        // Convert path to a more readable format, similar to yazi
        let current_path = &project.current_path;

        // Try to make path relative to home directory for cleaner display
        let display_path = if let Some(home) = dirs::home_dir() {
            if let Ok(relative) = current_path.strip_prefix(&home) {
                format!("~/{}", relative.display())
            } else {
                current_path.display().to_string()
            }
        } else {
            current_path.display().to_string()
        };

        let path_text = vec![Line::from(vec![Span::styled(
            display_path,
            Style::default().fg(Color::Yellow),
        )])];

        let path_bar = Paragraph::new(path_text);

        f.render_widget(path_bar, area);
    }

    pub fn draw_actions_popup(&mut self, f: &mut Frame, selected_action: usize) {
        let popup_area = self.centered_rect(30, 40, f.area());
        f.render_widget(Clear, popup_area);

        let actions = vec!["Build", "Test"];

        let action_items: Vec<ListItem> = actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if i == selected_action {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                ListItem::new(*action).style(style)
            })
            .collect();

        // Update list state for selected action
        self.actions_list_state.select(Some(selected_action));

        let actions_list = List::new(action_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Actions")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(actions_list, popup_area, &mut self.actions_list_state);
    }
}
