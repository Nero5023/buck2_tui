use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::buck::BuckProject;
use crate::ui::{Pane, UI};

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
                ui.current_pane = match ui.current_pane {
                    Pane::Directories => Pane::Targets,
                    Pane::Targets => Pane::Details,
                    Pane::Details => Pane::Directories,
                };
            }
            KeyCode::Char('h') | KeyCode::Left => {
                ui.current_pane = match ui.current_pane {
                    Pane::Targets => Pane::Directories,
                    Pane::Details => Pane::Targets,
                    _ => ui.current_pane,
                };
            }
            KeyCode::Char('l') | KeyCode::Right => {
                ui.current_pane = match ui.current_pane {
                    Pane::Directories => Pane::Targets,
                    Pane::Targets => Pane::Details,
                    _ => ui.current_pane,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match ui.current_pane {
                    Pane::Directories => project.next_directory(),
                    Pane::Targets => project.next_target(),
                    Pane::Details => {}
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match ui.current_pane {
                    Pane::Directories => project.prev_directory(),
                    Pane::Targets => project.prev_target(),
                    Pane::Details => {}
                }
            }
            KeyCode::Enter => {
                match ui.current_pane {
                    Pane::Directories => {
                        ui.current_pane = Pane::Targets;
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