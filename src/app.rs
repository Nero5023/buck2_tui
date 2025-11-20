use anyhow::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::event::{self};
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

use crate::buck::BuckProject;
use crate::events::EventHandler;
use crate::scheduler::Scheduler;
use crate::ui::UI;
use crate::ui::Pane;

#[derive(Debug, Clone, PartialEq)]
pub enum SearchPane {
    CurrentDirectory,
    Targets,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub active: bool,
    pub query: String,
    pub current_match_idx: usize,
    pub total_matches: usize,
    pub matches: Vec<usize>,  // indices of matching items in current pane
    pub searching_in_pane: SearchPane,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            current_match_idx: 0,
            total_matches: 0,
            matches: Vec::new(),
            searching_in_pane: SearchPane::CurrentDirectory,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.query.clear();
        self.current_match_idx = 0;
        self.total_matches = 0;
        self.matches.clear();
    }

    pub fn activate(&mut self, pane: Pane, current_selection: usize) {
        self.active = true;
        // Don't clear query - keep previous search string
        // self.query.clear();
        self.current_match_idx = current_selection;  // Start from current position
        // Don't clear total_matches and matches yet - will be recalculated if query exists
        // self.total_matches = 0;
        // self.matches.clear();

        // Determine which pane we're searching in
        self.searching_in_pane = match pane {
            Pane::CurrentDirectory | Pane::ParentDirectory => SearchPane::CurrentDirectory,
            Pane::Targets | Pane::Details => SearchPane::Targets,
            Pane::SelectedDirectory => SearchPane::CurrentDirectory,
        };
    }

    pub fn next_match(&mut self) {
        if self.total_matches > 0 {
            self.current_match_idx = (self.current_match_idx + 1) % self.total_matches;
        }
    }

    pub fn prev_match(&mut self) {
        if self.total_matches > 0 {
            if self.current_match_idx == 0 {
                self.current_match_idx = self.total_matches - 1;
            } else {
                self.current_match_idx -= 1;
            }
        }
    }
}

pub struct App {
    project: BuckProject,
    ui: UI,
    event_handler: EventHandler,
    scheduler: Scheduler,
    pub search_state: SearchState,
    should_quit: bool,
    show_actions: bool,
    selected_action: usize,
}

impl App {
    pub async fn new(project_path: String) -> Result<Self> {
        let project = BuckProject::new(project_path).await?;
        let ui = UI::new();
        let event_handler = EventHandler::new();
        let scheduler = Scheduler::new();
        let search_state = SearchState::new();

        Ok(Self {
            project,
            ui,
            event_handler,
            scheduler,
            search_state,
            should_quit: false,
            show_actions: false,
            selected_action: 0,
        })
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub async fn initialize(&mut self) {
        self.project
            .update_targets_for_selected_directory(&self.scheduler);
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            // Check for completed target loading results
            self.project
                .update_loaded_target_results(&self.scheduler)
                .await;

            terminal.draw(|f| {
                self.ui.draw(f, &self.project, &self.search_state);

                if self.show_actions {
                    self.ui.draw_actions_popup(f, self.selected_action);
                }
            })?;

            if event::poll(Duration::from_millis(100))? {
                let event = event::read()?;
                self.handle_event(event).await?;
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => {
                    // Only quit if not in search mode
                    if !self.search_state.active {
                        self.should_quit = true;
                    } else {
                        // In search mode, 'q' is treated as a regular character
                        self.event_handler
                            .handle_key_event(
                                key,
                                &mut self.project,
                                &mut self.ui,
                                &self.scheduler,
                                &mut self.search_state,
                                &mut self.show_actions,
                                &mut self.selected_action,
                            )
                            .await?;
                    }
                }
                KeyCode::Esc => {
                    // Esc handled by event handler (exits search or actions mode)
                    // Only quit app if not in any mode
                    if !self.search_state.active && !self.show_actions {
                        self.should_quit = true;
                    } else {
                        self.event_handler
                            .handle_key_event(
                                key,
                                &mut self.project,
                                &mut self.ui,
                                &self.scheduler,
                                &mut self.search_state,
                                &mut self.show_actions,
                                &mut self.selected_action,
                            )
                            .await?;
                    }
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                _ => {
                    self.event_handler
                        .handle_key_event(
                            key,
                            &mut self.project,
                            &mut self.ui,
                            &self.scheduler,
                            &mut self.search_state,
                            &mut self.show_actions,
                            &mut self.selected_action,
                        )
                        .await?;
                }
            },
            _ => {}
        }
        Ok(())
    }
}
