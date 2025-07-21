use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::buck::{BuckProject, BuckTarget};

pub struct UI {
    pub search_mode: bool,
    pub current_pane: Pane,
    pub current_group: PaneGroup,
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
            search_mode: false,
            current_pane: Pane::CurrentDirectory,
            current_group: PaneGroup::Explorer,
        }
    }

    pub fn draw(&self, f: &mut Frame, project: &BuckProject) {
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
        self.draw_current_directory(f, content_chunks[1], project);
        self.draw_targets(f, targets_chunks[0], project);
        self.draw_selected_directory(f, targets_chunks[1], project);
        self.draw_details(f, content_chunks[3], project);

        if self.search_mode {
            self.draw_search_popup(f, project);
        }
    }

    fn draw_parent_directory(&self, f: &mut Frame, area: Rect, project: &BuckProject) {
        let parent_dirs = project.get_parent_directories();

        let directories: Vec<ListItem> = parent_dirs
            .iter()
            .map(|dir| {
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

        f.render_widget(directories_list, area);
    }

    fn draw_current_directory(&self, f: &mut Frame, area: Rect, project: &BuckProject) {
        let current_dirs = project.get_current_directories();

        let directories: Vec<ListItem> = current_dirs
            .sub_directories
            .iter()
            .map(|dir| {
                let style = if dir.path == project.selected_directory {
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
                let text = format!("{} {} ({})", buck_indicator, display_path, target_count);

                ListItem::new(text).style(style)
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

        f.render_widget(directories_list, area);
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

    fn draw_targets(&self, f: &mut Frame, area: Rect, project: &BuckProject) {
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
                        let icon = Span::styled(
                            icon,
                            Style::default().fg(Color::from_u32(
                                u32::from_str_radix(&color[1..], 16).unwrap_or(0x888888),
                            )),
                        );
                        let text = Line::from(vec![
                            Span::raw(" "),
                            icon,
                            Span::raw(" "),
                            Span::raw(target.display_title()),
                        ]);
                        ListItem::new(text).style(style)
                    })
                    .collect()
            }
        } else {
            vec![]
        };

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
        let title = if project.search_query.is_empty() {
            format!("Targets ({})", package_name)
        } else {
            format!(
                "Targets ({}) - Search: {}",
                package_name, project.search_query
            )
        };

        let targets_list = List::new(targets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(block_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(targets_list, area);
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

    fn draw_search_popup(&self, f: &mut Frame, project: &BuckProject) {
        let popup_area = self.centered_rect(60, 20, f.area());
        f.render_widget(Clear, popup_area);

        let search_text = vec![Line::from(vec![
            Span::raw("Search: "),
            Span::styled(&project.search_query, Style::default().fg(Color::Yellow)),
        ])];

        let search_popup = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Fuzzy Search")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(search_popup, popup_area);
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

    pub fn draw_actions_popup(&self, f: &mut Frame, selected_action: usize) {
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

        let actions_list = List::new(action_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Actions")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(actions_list, popup_area);
    }
}
