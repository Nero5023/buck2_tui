use anyhow::Result;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

use crate::app::SearchState;
use crate::buck::BuckProject;
use crate::scheduler::Scheduler;
use crate::ui::Pane;
use crate::ui::PaneGroup;
use crate::ui::UI;
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
        scheduler: &Scheduler,
        search_state: &mut SearchState,
        show_actions: &mut bool,
        selected_action: &mut usize,
    ) -> Result<()> {
        if *show_actions {
            self.handle_actions_mode(key, project, ui, scheduler, show_actions, selected_action)
                .await?;
        } else if search_state.active {
            self.handle_search_mode(key, project, ui, search_state, scheduler).await?;
        } else {
            self.handle_normal_mode(key, project, ui, scheduler, search_state, show_actions, selected_action)
                .await?;
        }
        Ok(())
    }

    /// Get the current selection index for the active search pane
    fn get_current_selection(&self, project: &BuckProject, search_state: &SearchState) -> usize {
        match search_state.searching_in_pane {
            crate::app::SearchPane::CurrentDirectory => {
                // Find current selected directory index
                let current_dirs = project.get_current_directories();
                current_dirs
                    .sub_directories
                    .iter()
                    .position(|dir| dir.path == project.selected_directory)
                    .unwrap_or(0)
            }
            crate::app::SearchPane::Targets => project.selected_target,
        }
    }

    /// Update search matches and navigate to the nearest match
    ///
    /// This combines two operations:
    /// 1. Find all items matching the search query
    /// 2. Navigate to the closest match from current position
    fn update_and_navigate(
        &self,
        project: &mut BuckProject,
        ui: &mut UI,
        search_state: &mut SearchState,
        scheduler: &Scheduler,
    ) {
        let current_selection = self.get_current_selection(project, search_state);
        self.update_search_matches(project, ui, search_state, current_selection);

        // Navigate to the matched item if there are matches
        if search_state.total_matches > 0 {
            self.navigate_to_current_match(project, ui, search_state, scheduler);
        }
    }

    async fn handle_search_mode(
        &mut self,
        key: KeyEvent,
        project: &mut BuckProject,
        ui: &mut UI,
        search_state: &mut SearchState,
        scheduler: &Scheduler,
    ) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                search_state.reset();
            }
            KeyCode::Enter => {
                // Exit search mode without resetting (keep highlights)
                search_state.active = false;
            }
            KeyCode::Backspace => {
                search_state.query.pop();
                self.update_and_navigate(project, ui, search_state, scheduler);
            }
            KeyCode::Char(c) => {
                search_state.query.push(c);
                self.update_and_navigate(project, ui, search_state, scheduler);
            }
            _ => {}
        }
        Ok(())
    }

    /// Find all items matching the current search query
    ///
    /// Searches either directory names or target names based on `search_state.searching_in_pane`.
    /// Updates `search_state.matches` with indices of matching items and calculates the
    /// closest match from `current_selection`.
    fn update_search_matches(
        &self,
        project: &BuckProject,
        ui: &UI,
        search_state: &mut SearchState,
        current_selection: usize,
    ) {
        if search_state.query.is_empty() {
            search_state.matches.clear();
            search_state.current_match_idx = 0;
            search_state.total_matches = 0;
            return;
        }

        let query_lower = search_state.query.to_lowercase();

        // Find matches based on the pane we're searching in
        search_state.matches = match search_state.searching_in_pane {
            crate::app::SearchPane::CurrentDirectory => {
                // Search in current directory list
                let current_dirs = project.get_current_directories();
                current_dirs
                    .sub_directories
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, dir)| {
                        let display_path = if dir.path == project.current_path {
                            ".".to_string()
                        } else {
                            dir.path
                                .file_name()
                                .unwrap_or_else(|| dir.path.as_os_str())
                                .to_string_lossy()
                                .to_string()
                        };
                        if display_path.to_lowercase().contains(&query_lower) {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            crate::app::SearchPane::Targets => {
                // Search in targets list
                project
                    .filtered_targets
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, target)| {
                        if target.display_title().to_lowercase().contains(&query_lower) {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        };

        search_state.total_matches = search_state.matches.len();

        if search_state.total_matches == 0 {
            search_state.current_match_idx = 0;
            return;
        }

        // Find the closest match from current position
        // First check if current item matches
        if search_state.matches.contains(&current_selection) {
            // Current item matches, use it
            search_state.current_match_idx = search_state
                .matches
                .iter()
                .position(|&idx| idx == current_selection)
                .unwrap_or(0);
        } else {
            // Find the next match after current position
            let next_match = search_state
                .matches
                .iter()
                .position(|&idx| idx > current_selection);

            if let Some(pos) = next_match {
                search_state.current_match_idx = pos;
            } else {
                // No match after current position, wrap to first match
                search_state.current_match_idx = 0;
            }
        }
    }

    /// Navigate the UI to show the current search match
    ///
    /// For directory searches: updates `selected_directory` and loads targets
    /// For target searches: updates `selected_target` index
    fn navigate_to_current_match(
        &self,
        project: &mut BuckProject,
        ui: &mut UI,
        search_state: &SearchState,
        scheduler: &Scheduler,
    ) {
        if search_state.matches.is_empty() {
            return;
        }

        let current_match_idx = search_state.matches[search_state.current_match_idx];

        match search_state.searching_in_pane {
            crate::app::SearchPane::CurrentDirectory => {
                // Navigate to the matched directory
                let current_dirs = project.get_current_directories();
                if let Some(dir) = current_dirs.sub_directories.get(current_match_idx) {
                    project.selected_directory = dir.path.clone();
                    project.update_targets_for_selected_directory(scheduler);
                }
            }
            crate::app::SearchPane::Targets => {
                // Navigate to the matched target
                project.selected_target = current_match_idx;
            }
        }
    }

    /// Refresh search matches when directory/target list changes
    /// This is called when the user navigates to a different directory
    fn refresh_search_if_active(
        &self,
        project: &BuckProject,
        ui: &UI,
        search_state: &mut SearchState,
    ) {
        if search_state.query.is_empty() {
            return;
        }

        let current_selection = self.get_current_selection(project, search_state);
        self.update_search_matches(project, ui, search_state, current_selection);
    }

    async fn handle_normal_mode(
        &mut self,
        key: KeyEvent,
        project: &mut BuckProject,
        ui: &mut UI,
        scheduler: &Scheduler,
        search_state: &mut SearchState,
        show_actions: &mut bool,
        selected_action: &mut usize,
    ) -> Result<()> {
        match key.code {
            KeyCode::Char('/') => {
                // Get current selection based on current pane
                let current_selection = match ui.current_pane {
                    Pane::CurrentDirectory | Pane::ParentDirectory | Pane::SelectedDirectory => {
                        // Find current selected directory index
                        let current_dirs = project.get_current_directories();
                        current_dirs
                            .sub_directories
                            .iter()
                            .position(|dir| dir.path == project.selected_directory)
                            .unwrap_or(0)
                    }
                    Pane::Targets | Pane::Details => project.selected_target,
                };
                search_state.activate(ui.current_pane, current_selection);

                // If there's a previous query, recalculate matches for the current pane
                if !search_state.query.is_empty() {
                    self.update_search_matches(project, ui, search_state, current_selection);
                    // Navigate to the matched item
                    if search_state.total_matches > 0 {
                        self.navigate_to_current_match(project, ui, search_state, scheduler);
                    }
                }
            }
            KeyCode::Char('n') if search_state.total_matches > 0 => {
                search_state.next_match();
                self.navigate_to_current_match(project, ui, search_state, scheduler);
            }
            KeyCode::Char('N') if search_state.total_matches > 0 => {
                search_state.prev_match();
                self.navigate_to_current_match(project, ui, search_state, scheduler);
            }
            KeyCode::Char('a') => {
                if ui.current_pane == Pane::Targets && project.get_selected_target().is_some() {
                    *show_actions = true;
                    *selected_action = 0;
                }
            }
            KeyCode::Char('o') => {
                if ui.current_pane == Pane::Targets {
                    project.open_target_definition(scheduler);
                }
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
                            project.navigate_to_directory(parent.to_path_buf(), scheduler);
                            // Select the directory we came from (previous current directory)
                            project.selected_directory = previous_current;
                            // Update targets for the newly selected directory
                            project.update_targets_for_selected_directory(scheduler);
                            // Refresh search matches for new directory
                            self.refresh_search_if_active(project, ui, search_state);
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
                            project.navigate_to_directory(
                                project.selected_directory.clone(),
                                scheduler,
                            );
                            // Refresh search matches for new directory
                            self.refresh_search_if_active(project, ui, search_state);
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
                            project.update_targets_for_selected_directory(scheduler);
                            // Refresh search matches for new directory's targets
                            if matches!(search_state.searching_in_pane, crate::app::SearchPane::Targets) {
                                self.refresh_search_if_active(project, ui, search_state);
                            }
                        }
                    }
                    Pane::SelectedDirectory => {
                        // For now, no navigation within selected directory
                    }
                    Pane::Targets => project.next_target(scheduler),
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
                            project.update_targets_for_selected_directory(scheduler);
                            // Refresh search matches for new directory's targets
                            if matches!(search_state.searching_in_pane, crate::app::SearchPane::Targets) {
                                self.refresh_search_if_active(project, ui, search_state);
                            }
                        }
                    }
                    Pane::SelectedDirectory => {
                        // This pane should never be focused, but handle it gracefully
                        ui.current_pane = Pane::Targets;
                    }
                    Pane::Targets => project.prev_target(scheduler),
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
                            project.navigate_to_directory(
                                project.selected_directory.clone(),
                                scheduler,
                            );
                            // Refresh search matches for new directory
                            self.refresh_search_if_active(project, ui, search_state);
                        } else {
                            // If current directory is selected, switch to inspector
                            ui.current_group = PaneGroup::Inspector;
                            ui.current_pane = Pane::Targets;
                        }
                    }
                    Pane::SelectedDirectory => {
                        // This pane should never be focused, but handle it gracefully
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

    async fn handle_actions_mode(
        &mut self,
        key: KeyEvent,
        project: &mut BuckProject,
        _ui: &mut UI,
        _scheduler: &Scheduler,
        show_actions: &mut bool,
        selected_action: &mut usize,
    ) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                *show_actions = false;
                *selected_action = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let action_count = 2; // build, test
                *selected_action = (*selected_action + 1) % action_count;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let action_count = 2; // build, test
                *selected_action = (*selected_action + action_count - 1) % action_count;
            }
            KeyCode::Enter => {
                if let Some(target) = project.get_selected_target() {
                    let target_name = &target.full_target_label_name;
                    match *selected_action {
                        0 => {
                            debug!("Building target: {}", target_name);
                            // TODO: Execute build command via scheduler
                        }
                        1 => {
                            debug!("Testing target: {}", target_name);
                            // TODO: Execute test command via scheduler
                        }
                        _ => {}
                    }
                }
                *show_actions = false;
                *selected_action = 0;
            }
            _ => {}
        }
        Ok(())
    }
}
