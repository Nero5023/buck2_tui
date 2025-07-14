use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::buck::BuckProject;
use crate::ui::{Pane, PaneGroup, UI};
use tracing::debug;

pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle_key_event(
        &mut self,
        key: KeyEvent,
        project: &mut BuckProject,
        ui: &mut UI,
    ) -> Result<()> {
        if ui.search_mode {
            self.handle_search_mode(key, project, ui).await?;
        } else {
            self.handle_normal_mode(key, project, ui).await?;
        }
        Ok(())
    }

    async fn handle_search_mode(
        &mut self,
        key: KeyEvent,
        project: &mut BuckProject,
        ui: &mut UI,
    ) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                ui.search_mode = false;
                project.set_search_query(String::new());
            }
            KeyCode::Enter => {
                ui.search_mode = false;
            }
            KeyCode::Backspace => {
                let mut query = project.search_query.clone();
                query.pop();
                project.set_search_query(query);
            }
            KeyCode::Char(c) => {
                let mut query = project.search_query.clone();
                query.push(c);
                project.set_search_query(query);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_normal_mode(
        &mut self,
        key: KeyEvent,
        project: &mut BuckProject,
        ui: &mut UI,
    ) -> Result<()> {
        match key.code {
            KeyCode::Char('/') => {
                ui.search_mode = true;
            }
            KeyCode::Tab => {
                // Switch between Explorer and Inspector groups
                ui.current_group = match ui.current_group {
                    PaneGroup::Explorer => PaneGroup::Inspector,
                    PaneGroup::Inspector => PaneGroup::Explorer,
                };
                // Set appropriate pane for the group
                ui.current_pane = match ui.current_group {
                    PaneGroup::Explorer => Pane::CurrentDirectory,
                    PaneGroup::Inspector => Pane::Targets,
                };
            }
            KeyCode::Char('h') | KeyCode::Left => {
                match ui.current_group {
                    PaneGroup::Explorer => {
                        // In explorer mode, 'h' goes to parent directory, but keeps focus on current dir pane
                        if let Some(parent) = project.current_path.parent() {
                            let previous_current = project.current_path.clone();
                            project.navigate_to_directory(parent.to_path_buf());
                            // Select the directory we came from (previous current directory)
                            project.selected_directory = previous_current;
                            // Update targets for the newly selected directory
                            project.update_targets_for_selected_directory();
                        }
                        // Always keep focus on current directory pane (never focus on parent pane)
                        ui.current_pane = Pane::CurrentDirectory;
                    }
                    PaneGroup::Inspector => {
                        // In inspector mode, 'h' moves left within inspector panes
                        ui.current_pane = match ui.current_pane {
                            Pane::Details => Pane::Targets,
                            _ => ui.current_pane,
                        };
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                match ui.current_group {
                    PaneGroup::Explorer => {
                        // In explorer mode, 'l' enters selected directory, keeps focus on current dir pane
                        if project.selected_directory != project.current_path {
                            project.navigate_to_directory(project.selected_directory.clone());
                        }
                        // Always keep focus on current directory pane
                        ui.current_pane = Pane::CurrentDirectory;
                    }
                    PaneGroup::Inspector => {
                        // In inspector mode, 'l' moves right within inspector panes
                        ui.current_pane = match ui.current_pane {
                            Pane::Targets => Pane::Details,
                            _ => ui.current_pane,
                        };
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                debug!("trigger next (down)");
                match ui.current_pane {
                    Pane::ParentDirectory => {
                        // Never focus on parent directory - this shouldn't happen
                    }
                    Pane::CurrentDirectory => {
                        // Navigate through current directories
                        let current_dirs = project.get_current_directories();
                        if let Some(next_dir) =
                            current_dirs.select_next_directory(&project.selected_directory)
                        {
                            project.selected_directory = next_dir.clone();
                            // Update targets for the newly selected directory
                            project.update_targets_for_selected_directory();
                        }
                    }
                    Pane::Targets => project.next_target(),
                    Pane::Details => {}
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match ui.current_pane {
                    Pane::ParentDirectory => {
                        // Never focus on parent directory - this shouldn't happen
                    }
                    Pane::CurrentDirectory => {
                        // Navigate through current directories
                        let current_dirs = project.get_current_directories();
                        if let Some(prev_dir) =
                            current_dirs.select_prev_directory(&project.selected_directory)
                        {
                            project.selected_directory = prev_dir.clone();
                            // Update targets for the newly selected directory
                            project.update_targets_for_selected_directory();
                        }
                    }
                    Pane::Targets => project.prev_target(),
                    Pane::Details => {}
                }
            }
            KeyCode::Enter => {
                match ui.current_pane {
                    Pane::ParentDirectory => {
                        // Never focus on parent directory - this shouldn't happen
                    }
                    Pane::CurrentDirectory => {
                        // Navigate into selected directory or switch to inspector
                        if project.selected_directory != project.current_path {
                            project.navigate_to_directory(project.selected_directory.clone());
                        } else {
                            // If current directory is selected, switch to inspector
                            ui.current_group = PaneGroup::Inspector;
                            ui.current_pane = Pane::Targets;
                        }
                    }
                    Pane::Targets => {
                        ui.current_pane = Pane::Details;
                    }
                    Pane::Details => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
}
