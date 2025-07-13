use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

use crate::buck::BuckProject;
use crate::events::EventHandler;
use crate::ui::UI;

pub struct App {
    project: BuckProject,
    ui: UI,
    event_handler: EventHandler,
    should_quit: bool,
}

impl App {
    pub async fn new(project_path: String) -> Result<Self> {
        let project = BuckProject::new(project_path).await?;
        let ui = UI::new();
        let event_handler = EventHandler::new();

        Ok(Self {
            project,
            ui,
            event_handler,
            should_quit: false,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            // Check for completed target loading results
            self.project.update_loaded_target_results();

            terminal.draw(|f| {
                self.ui.draw(f, &self.project);
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
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.should_quit = true;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                _ => {
                    self.event_handler
                        .handle_key_event(key, &mut self.project, &mut self.ui)
                        .await?;
                }
            },
            _ => {}
        }
        Ok(())
    }
}

