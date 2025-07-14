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
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // Parent directory
                Constraint::Percentage(25), // Current directory
                Constraint::Percentage(30), // Target list
                Constraint::Percentage(25), // Target details
            ])
            .split(f.area());

        self.draw_parent_directory(f, chunks[0], project);
        self.draw_current_directory(f, chunks[1], project);
        self.draw_targets(f, chunks[2], project);
        self.draw_details(f, chunks[3], project);

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

                let target_count = if dir.targets_loading {
                    "loading...".to_string()
                } else {
                    dir.targets.len().to_string()
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

                        let text = format!("{} ({})", target.name, target.rule_type);
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
        vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(&target.name),
            ]),
            Line::from(vec![
                Span::styled("Rule Type: ", Style::default().fg(Color::Cyan)),
                Span::raw(&target.rule_type),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::Cyan)),
                Span::raw(target.path.display().to_string()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Dependencies: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", target.deps.len())),
            ]),
        ]
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
}
