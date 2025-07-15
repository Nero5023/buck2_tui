use anyhow::{Result, anyhow};
use nerd_font_symbols::dev as dev_symbols;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;

#[derive(Debug)]
struct ActiveLoadRequest {
    dir_path: PathBuf,
    token: CancellationToken,
}

#[derive(Debug)]
struct ActiveDetailRequest {
    dir_path: PathBuf,
    target_index: usize,
    token: CancellationToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuckTarget {
    pub name: String,
    pub rule_type: String,
    pub path: PathBuf,
    pub deps: Vec<String>,
    pub details_loaded: bool,
}

impl BuckTarget {
    pub fn target_name(&self) -> String {
        self.name
            .split("//")
            .last()
            .unwrap()
            .split(":")
            .last()
            .unwrap()
            .to_string()
    }

    fn get_rule_language(&self) -> &str {
        // Remove prefix underscore and split by underscore to get the first part
        let rule_type = self.rule_type.strip_prefix('_').unwrap_or(&self.rule_type);
        rule_type.split('_').next().unwrap_or("unknown")
    }

    pub fn get_language_icon(&self) -> (&str, &str) {
        match self.get_rule_language() {
            "rust" => (dev_symbols::DEV_RUST, "#dea584"), // Rust
            "python" => (dev_symbols::DEV_PYTHON, "#ffbc03"), // Python
            "cpp" | "cxx" => (dev_symbols::DEV_CPLUSPLUS, "#519aba"), // C++
            "c" => (dev_symbols::DEV_C_LANG, "#599eff"),  // C
            "java" => (dev_symbols::DEV_JAVA, "#cc3e44"), // Java
            "javascript" | "js" => (dev_symbols::DEV_JAVASCRIPT, "#cbcb41"), // JavaScript
            "go" => (dev_symbols::DEV_GO, "#00add8"),     // Go
            "swift" => (dev_symbols::DEV_SWIFT, "#e37933"), // Swift
            "kotlin" => (dev_symbols::DEV_KOTLIN, "#7f52ff"), // Kotlin
            "scala" => (dev_symbols::DEV_SCALA, "#cc3e44"), // Scala
            "haskell" => (dev_symbols::DEV_HASKELL, "#a074c4"), // Haskell
            "clojure" => (dev_symbols::DEV_CLOJURE, "#8dc149"), // Clojure
            "erlang" => (dev_symbols::DEV_ERLANG, "#b83998"), // Erlang
            "elixir" => (dev_symbols::DEV_ELIXIR, "#a074c4"), // Elixir
            "ruby" => (dev_symbols::DEV_RUBY, "#701516"), // Ruby
            "php" => (dev_symbols::DEV_PHP, "#a074c4"),   // PHP
            "dart" => (dev_symbols::DEV_DART, "#03589c"), // Dart
            "lua" => (dev_symbols::DEV_LUA, "#51a0cf"),   // Lua
            "shell" | "bash" => (dev_symbols::DEV_BASH, "#89e051"), // Shell
            "docker" => (dev_symbols::DEV_DOCKER, "#458ee6"), // Docker
            "vim" => (dev_symbols::DEV_VIM, "#019833"),   // Vim
            "web" | "html" => (dev_symbols::DEV_HTML5, "#e44d26"), // HTML5
            "css" => (dev_symbols::DEV_CSS3, "#663399"),  // CSS3
            "git" => (dev_symbols::DEV_GIT, "#f14c28"),   // Git
            "angular" => (dev_symbols::DEV_ANGULAR, "#e23f67"), // Angular
            "vue" => (dev_symbols::DEV_VUEJS, "#8dc149"), // Vue
            _ => ("", "#888888"),                        // default: gear
        }
    }

    pub fn display_title(&self) -> String {
        format!(" {} ({})", self.target_name(), self.rule_type)
    }
}

#[derive(Debug, Clone)]
pub struct TargetDetails {
    pub rule_type: String,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuckDirectory {
    pub path: PathBuf,
    pub targets: Vec<BuckTarget>,
    pub has_buck_file: bool,
    pub targets_loaded: bool,
    pub targets_loading: bool,
}

pub struct UICurrentDirectory {
    path: PathBuf,
    pub sub_directories: Vec<BuckDirectory>,
    dir_to_index: HashMap<PathBuf, usize>,
}

impl UICurrentDirectory {
    pub fn new(current_path: &PathBuf) -> Self {
        let mut sub_directories = Vec::new();
        let mut dir_to_index = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(current_path) {
            // Add current directory as "."
            let buck_file = current_path.join("BUCK");
            let targets_file = current_path.join("TARGETS");
            let has_buck_file = buck_file.exists() || targets_file.exists();

            let current_dir = BuckDirectory {
                path: current_path.clone(),
                targets: Vec::new(),
                has_buck_file,
                targets_loaded: false,
                targets_loading: false,
            };

            sub_directories.push(current_dir);

            // Add subdirectories
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let path = entry.path();
                    let buck_file = path.join("BUCK");
                    let buck2_file: PathBuf = path.join("BUCK2");
                    let has_buck_file = buck_file.exists() || buck2_file.exists();

                    let dir = BuckDirectory {
                        path: path.clone(),
                        targets: Vec::new(),
                        has_buck_file,
                        targets_loaded: false,
                        targets_loading: false,
                    };

                    sub_directories.push(dir);
                }
            }

            // Sort directories with "." always first
            sub_directories.sort_by(|a, b| {
                // "." always comes first
                if a.path == *current_path {
                    std::cmp::Ordering::Less
                } else if b.path == *current_path {
                    std::cmp::Ordering::Greater
                } else {
                    a.path.file_name().cmp(&b.path.file_name())
                }
            });

            // Rebuild the index map after sorting
            for (index, dir) in sub_directories.iter().enumerate() {
                dir_to_index.insert(dir.path.clone(), index);
            }
        }

        Self {
            path: current_path.clone(),
            sub_directories,
            dir_to_index,
        }
    }

    pub fn select_next_directory(&self, dir: &PathBuf) -> Option<&PathBuf> {
        if let Some(index) = self.dir_to_index.get(dir) {
            let next_index = (index + 1) % self.sub_directories.len();
            Some(&self.sub_directories[next_index].path)
        } else {
            None
        }
    }

    pub fn select_prev_directory(&self, dir: &PathBuf) -> Option<&PathBuf> {
        if let Some(index) = self.dir_to_index.get(dir) {
            let prev_index = if *index > 0 {
                index - 1
            } else {
                self.sub_directories.len() - 1
            };
            Some(&self.sub_directories[prev_index].path)
        } else {
            None
        }
    }

    pub fn get_directory(&self, dir: &PathBuf) -> Option<&BuckDirectory> {
        if let Some(index) = self.dir_to_index.get(dir) {
            Some(&self.sub_directories[*index])
        } else {
            None
        }
    }
}

impl BuckDirectory {
    fn abs_path(&self) -> PathBuf {
        self.path.canonicalize().unwrap_or(self.path.clone())
    }
}

pub struct BuckProject {
    pub root_path: PathBuf,
    pub current_path: PathBuf,
    // pub directories: Vec<BuckDirectory>,
    pub directories: HashMap<PathBuf, BuckDirectory>,
    pub selected_directory: PathBuf,
    pub selected_target: usize,
    pub search_query: String,
    // used in the UI to display for the list of targets in the targets panel
    pub filtered_targets: Vec<BuckTarget>,

    pub cells: HashMap<String, PathBuf>,
    pub target_loader_tx: Option<mpsc::UnboundedSender<(PathBuf, CancellationToken)>>,
    pub target_result_rx: Option<mpsc::UnboundedReceiver<(PathBuf, Result<Vec<BuckTarget>>)>>,
    pub target_detail_loader_tx:
        Option<mpsc::UnboundedSender<(PathBuf, usize, String, CancellationToken)>>,
    pub target_detail_result_rx:
        Option<mpsc::UnboundedReceiver<(PathBuf, usize, Result<TargetDetails>)>>,
    active_load_request: Option<ActiveLoadRequest>,
    active_detail_request: Option<ActiveDetailRequest>,
}

impl BuckProject {
    pub async fn new(project_path: String) -> Result<Self> {
        let root_path = PathBuf::from(project_path);

        if !root_path.exists() {
            return Err(anyhow!(
                "Project path does not exist: {}",
                root_path.display()
            ));
        }

        let (loader_tx, loader_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let (detail_loader_tx, detail_loader_rx) = mpsc::unbounded_channel();
        let (detail_result_tx, detail_result_rx) = mpsc::unbounded_channel();

        // Spawn background task for loading targets
        tokio::spawn(Self::target_loader_task(loader_rx, result_tx));
        // Spawn background task for loading target details
        tokio::spawn(Self::target_detail_loader_task(
            detail_loader_rx,
            detail_result_tx,
        ));

        let current_path = root_path.clone();
        let selected_directory = current_path.clone();

        let mut project = Self {
            root_path,
            current_path,
            directories: HashMap::new(),
            selected_directory,
            selected_target: 0,
            search_query: String::new(),
            filtered_targets: Vec::new(),
            cells: HashMap::new(),
            target_loader_tx: Some(loader_tx),
            target_result_rx: Some(result_rx),
            target_detail_loader_tx: Some(detail_loader_tx),
            target_detail_result_rx: Some(detail_result_rx),
            active_load_request: None,
            active_detail_request: None,
        };

        project.load_cells().await?;

        // Request targets for the initial current directory if it has Buck files
        project.update_targets_for_selected_directory();

        Ok(project)
    }

    async fn target_loader_task(
        mut loader_rx: mpsc::UnboundedReceiver<(PathBuf, CancellationToken)>,
        result_tx: mpsc::UnboundedSender<(PathBuf, Result<Vec<BuckTarget>>)>,
    ) {
        while let Some((path, cancel_token)) = loader_rx.recv().await {
            let result = tokio::select! {
                _ = cancel_token.cancelled() => {
                    continue; // Skip if cancelled
                }
                result = Self::load_targets_from_directory_static(&path) => {
                    debug!("get targets for {} , result: {:?}", path.display(), result);
                    result
                }
            };

            if !cancel_token.is_cancelled() {
                let _ = result_tx.send((path, result));
            }
        }
    }

    async fn target_detail_loader_task(
        mut detail_loader_rx: mpsc::UnboundedReceiver<(PathBuf, usize, String, CancellationToken)>,
        detail_result_tx: mpsc::UnboundedSender<(PathBuf, usize, Result<TargetDetails>)>,
    ) {
        while let Some((dir_path, target_index, target_label, cancel_token)) =
            detail_loader_rx.recv().await
        {
            let result = tokio::select! {
                _ = cancel_token.cancelled() => {
                    continue; // Skip if cancelled
                }
                result = Self::get_target_details_static_async(&target_label) => {
                    result
                }
            };

            if !cancel_token.is_cancelled() {
                let _ = detail_result_tx.send((dir_path, target_index, result));
            }
        }
    }

    // request targets for the currently selected directory, if loaded, update the filtered targets
    // which is used to display the list of targets in the targets panel
    pub fn request_targets_for_directory(&mut self, dir: PathBuf) {
        // Check early if we should skip this request
        {
            if let Some(dir) = &self.directories.get(&dir)
                && (dir.targets_loaded || dir.targets_loading || !dir.has_buck_file)
            {
                self.update_filtered_targets_with_reset(true);
                return;
            }
        }

        // Cancel previous request if any and reset its loading state
        if let Some(active_request) = &self.active_load_request {
            active_request.token.cancel();
            // Reset loading state for the previously loading directory
            self.directories.get_mut(&dir).unwrap().targets_loading = true;
        }

        // Create new load request
        let token = CancellationToken::new();
        self.active_load_request = Some(ActiveLoadRequest {
            dir_path: dir.clone(),
            token: token.clone(),
        });

        // Mark as loading
        self.directories.get_mut(&dir).unwrap().targets_loading = true;

        // Send request to background task
        if let Some(tx) = &self.target_loader_tx {
            let _ = tx.send((dir, token));
        }
    }

    pub fn request_target_details(&mut self, dir_path: PathBuf, target_index: usize) {
        let dir = self.directories.get(&dir_path).unwrap();
        if target_index >= dir.targets.len() {
            return;
        }

        let target = &dir.targets[target_index];
        if target.details_loaded {
            return; // Already loaded
        }

        // Cancel previous detail request if any
        if let Some(active_request) = &self.active_detail_request {
            active_request.token.cancel();
        }

        let target_label = target.name.clone();

        // Create new detail request
        let token = CancellationToken::new();
        self.active_detail_request = Some(ActiveDetailRequest {
            dir_path: dir_path.clone(),
            target_index,
            token: token.clone(),
        });

        // Send request to background task
        if let Some(tx) = &self.target_detail_loader_tx {
            let _ = tx.send((dir_path, target_index, target_label, token));
        }
    }

    pub fn update_loaded_target_results(&mut self) {
        // Process target list results
        let mut target_results_to_process = Vec::new();
        if let Some(rx) = &mut self.target_result_rx {
            while let Ok((dir_path, result)) = rx.try_recv() {
                target_results_to_process.push((dir_path, result));
            }
        }

        for (dir_path, result) in target_results_to_process {
            debug!(
                "update loaded target results for dir index: {}, result: {:?}",
                dir_path.display(),
                result
            );
            debug!("self.directories.len(): {}", self.directories.len());

            let dir = self.directories.get_mut(&dir_path).unwrap();
            debug!("dir: {:?}", dir);
            dir.targets_loading = false;

            // Clear active load request if this is the one that was loading
            if let Some(active_request) = &self.active_load_request
                && active_request.dir_path == dir.path
            {
                self.active_load_request = None;
            }

            let current_selected_dir = dir.path == self.selected_directory;

            match result {
                Ok(targets) => {
                    dir.targets = targets;
                    dir.targets_loaded = true;
                }
                Err(_) => {
                    // Keep empty targets on error
                    dir.targets = Vec::new();
                    dir.targets_loaded = true;
                }
            }

            debug!(
                "is current selected dir: {}, dir_indxe: {}, self.selected_directory: {}",
                current_selected_dir,
                dir_path.display(),
                self.selected_directory.display()
            );

            // Update filtered targets if this is the selected directory
            if current_selected_dir {
                self.update_filtered_targets();
                // Trigger detail loading for the first target (which is now selected)
                if !self.filtered_targets.is_empty() {
                    self.request_target_details_for_selected();
                }
            }
        }

        // Process target detail results
        let mut detail_results_to_process = Vec::new();
        if let Some(rx) = &mut self.target_detail_result_rx {
            while let Ok((dir_index, target_index, result)) = rx.try_recv() {
                detail_results_to_process.push((dir_index, target_index, result));
            }
        }

        for (dir_path, target_index, result) in detail_results_to_process {
            let dir = self.directories.get_mut(&dir_path).unwrap();
            if target_index < dir.targets.len() {
                let target = &mut dir.targets[target_index];

                // Clear active detail request if this is the one that was loading
                if let Some(active_request) = &self.active_detail_request
                    && active_request.dir_path == dir_path
                    && active_request.target_index == target_index
                {
                    self.active_detail_request = None;
                }

                match result {
                    Ok(details) => {
                        target.rule_type = details.rule_type;
                        target.deps = details.deps;
                        target.details_loaded = true;
                    }
                    Err(_) => {
                        // Mark as loaded even on error to avoid retrying
                        target.rule_type = "error".to_string();
                        target.details_loaded = true;
                    }
                }

                // Update filtered targets if this affects the currently displayed targets
                if dir_path == self.selected_directory {
                    self.update_filtered_targets_with_reset(false);
                }
            }
        }
    }

    async fn load_targets_from_directory_static(dir_path: &Path) -> Result<Vec<BuckTarget>> {
        // Use buck2 targets command to get actual target information
        let output = tokio::process::Command::new("buck2")
            .arg("targets")
            .arg(":")
            .current_dir(dir_path)
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Self::parse_buck2_targets_output_static(&stdout, dir_path)
        } else {
            // If no BUCK or TARGET file exists, return empty target list
            let buck_file = dir_path.join("BUCK");
            let target_file = dir_path.join("TARGET");

            if !buck_file.exists() && !target_file.exists() {
                return Ok(Vec::new());
            }
            Err(anyhow!(
                "Failed to get targets from directory: {}\nError: {}",
                dir_path.display(),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    async fn load_cells(&mut self) -> Result<()> {
        let output = Command::new("buck2")
            .arg("audit")
            .arg("cell")
            .arg("--json")
            .current_dir(&self.root_path)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<HashMap<String, String>>(&stdout) {
                Ok(cells_data) => {
                    self.cells = cells_data
                        .into_iter()
                        .map(|(name, path)| (name, PathBuf::from(path)))
                        .collect();
                }
                Err(e) => {
                    // If we can't parse the cells, just leave it empty and continue
                    eprintln!("Warning: Failed to parse buck2 audit cell output: {e}");
                }
            }
        } else {
            // If the command fails, just leave cells empty and continue
            eprintln!(
                "Warning: Failed to get buck2 cells: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    fn parse_buck2_targets_output_static(output: &str, dir_path: &Path) -> Result<Vec<BuckTarget>> {
        let mut targets = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Only store basic info initially, defer detailed query until target is selected
            targets.push(BuckTarget {
                name: line.to_string(),
                rule_type: "unknown".to_string(), // Will be loaded on demand
                path: dir_path.to_path_buf(),
                deps: Vec::new(), // Will be loaded on demand
                details_loaded: false,
            });
        }

        Ok(targets)
    }

    async fn get_target_details_static_async(target_label: &str) -> Result<TargetDetails> {
        // Try to get detailed information about the target
        let output = tokio::process::Command::new("buck2")
            .arg("query")
            .arg("-A")
            .arg(target_label)
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Self::parse_target_query_output_static(&stdout, target_label)
            }
            _ => Err(anyhow!("Failed to get target details")),
        }
    }

    fn parse_target_query_output_static(output: &str, target_label: &str) -> Result<TargetDetails> {
        // Parse JSON output from buck2 query
        match serde_json::from_str::<serde_json::Value>(output) {
            Ok(json) => match json.get(target_label) {
                Some(json) => {
                    let rule_type = json
                        .get("buck.type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let deps = json
                        .get("buck.deps")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_else(Vec::new);

                    Ok(TargetDetails { rule_type, deps })
                }
                None => Err(anyhow!("Target not found: {}", target_label)),
            },
            Err(_) => Err(anyhow!("Failed to parse target query output")),
        }
    }

    pub fn update_filtered_targets(&mut self) {
        self.update_filtered_targets_with_reset(true);
    }

    fn update_filtered_targets_with_reset(&mut self, reset_selection: bool) {
        // Get targets from the currently selected directory
        let selected_dir_targets = if let Some(selected_dir) = self.get_selected_directory() {
            selected_dir.targets.clone()
        } else {
            Vec::new()
        };

        if self.search_query.is_empty() {
            self.filtered_targets = selected_dir_targets;
        } else {
            // TODO: maybe use fzf to filter targets
            self.filtered_targets = selected_dir_targets
                .iter()
                .filter(|target| {
                    target
                        .display_title()
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                        || target
                            .rule_type
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase())
                })
                .cloned()
                .collect();
        }

        // Only reset selected target when explicitly requested (directory/search changes)
        if reset_selection {
            self.selected_target = 0;
        } else {
            // Clamp selected target to valid range if list shortened
            if self.selected_target >= self.filtered_targets.len()
                && !self.filtered_targets.is_empty()
            {
                self.selected_target = self.filtered_targets.len() - 1;
            }
        }
    }

    pub fn get_selected_directory(&self) -> Option<&BuckDirectory> {
        self.directories.get(&self.selected_directory)
    }

    pub fn current_cell(&self) -> Option<&str> {
        let selected_dir = self.get_selected_directory()?;

        // Get the absolute path of the selected directory
        let current_path = selected_dir.abs_path();

        let mut best_match: Option<(&str, usize)> = None;

        for (cell_name, cell_path) in &self.cells {
            // Check if cell_path is a prefix of current_path
            if current_path.starts_with(cell_path) {
                // Get the number of components in the cell_path
                let cell_components_count = cell_path.components().count();

                match best_match {
                    None => best_match = Some((cell_name, cell_components_count)),
                    Some((_, best_len)) if cell_components_count > best_len => {
                        best_match = Some((cell_name, cell_components_count));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(name, _)| name)
    }

    pub fn get_selected_buck_package_name(&self) -> Option<String> {
        let cell = self.current_cell()?;
        let cell_path = self.cells.get(cell)?;
        let selected_dir = self.get_selected_directory()?;
        let current_path = selected_dir.abs_path();

        // Strip the cell path from the current path
        let relative_path = current_path.strip_prefix(cell_path).ok()?;

        // Convert to string and format as cell//path
        if relative_path.as_os_str().is_empty() {
            // If we're at the cell root, just return the cell name
            Some(format!("{cell}//"))
        } else {
            // Convert path separators to forward slashes for Buck format
            let path_str = relative_path.to_string_lossy().replace('\\', "/");
            Some(format!("{cell}//{path_str}"))
        }
    }

    pub fn get_selected_target(&self) -> Option<&BuckTarget> {
        self.filtered_targets.get(self.selected_target)
    }

    pub fn next_target(&mut self) {
        if !self.filtered_targets.is_empty() {
            self.selected_target = (self.selected_target + 1) % self.filtered_targets.len();
            // Request target details for the newly selected target
            self.request_target_details_for_selected();
        }
    }

    pub fn prev_target(&mut self) {
        if !self.filtered_targets.is_empty() {
            self.selected_target = if self.selected_target > 0 {
                self.selected_target - 1
            } else {
                self.filtered_targets.len() - 1
            };
            // Request target details for the newly selected target
            self.request_target_details_for_selected();
        }
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.update_filtered_targets();
    }

    fn request_target_details_for_selected(&mut self) {
        if let Some(selected_target) = self.get_selected_target() {
            // Find the actual index of the selected target in the directory's target list
            if let Some(selected_dir) = self.get_selected_directory()
                && let Some(actual_target_index) = selected_dir
                    .targets
                    .iter()
                    .position(|t| t.name == selected_target.name && t.path == selected_target.path)
            {
                self.request_target_details(self.selected_directory.clone(), actual_target_index);
            }
        }
    }

    pub fn get_parent_directories(&self) -> Vec<BuckDirectory> {
        if let Some(parent) = self.current_path.parent()
            && let Ok(entries) = std::fs::read_dir(parent)
        {
            let mut dirs = Vec::new();
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let path = entry.path();
                    let buck_file = path.join("BUCK");
                    let targets_file = path.join("TARGETS");
                    let has_buck_file = buck_file.exists() || targets_file.exists();

                    dirs.push(BuckDirectory {
                        path,
                        targets: Vec::new(),
                        has_buck_file,
                        targets_loaded: false,
                        targets_loading: false,
                    });
                }
            }
            dirs.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name()));
            return dirs;
        }
        Vec::new()
    }

    pub fn get_current_directories(&self) -> UICurrentDirectory {
        UICurrentDirectory::new(&self.current_path)
    }

    pub fn navigate_to_directory(&mut self, dir_path: PathBuf) {
        self.current_path = dir_path.clone();
        self.selected_directory = dir_path;
        self.selected_target = 0;
        self.filtered_targets.clear();

        // Request targets for the new current directory
        self.update_targets_for_selected_directory();
    }

    pub fn update_targets_for_selected_directory(&mut self) {
        // TODO: it is no need to get the current directories here, we can use BuckDirectory for
        // self.selected_directory
        let current_dirs = self.get_current_directories();

        if let Some(selected_dir) = current_dirs.get_directory(&self.selected_directory) {
            if selected_dir.has_buck_file {
                // Find or add directory to our internal list for async loading
                self.find_or_add_directory(&selected_dir.path);
                self.request_targets_for_directory(self.selected_directory.clone());
            } else {
                // Clear targets if directory doesn't have Buck files
                self.filtered_targets.clear();
                self.selected_target = 0;
            }
        }
    }

    fn find_or_add_directory(&mut self, path: &PathBuf) {
        // First, try to find existing directory
        if self.directories.contains_key(path) {
            return;
        }

        // If not found, add it
        let buck_file = path.join("BUCK");
        let targets_file = path.join("TARGETS");
        let has_buck_file = buck_file.exists() || targets_file.exists();

        let new_dir = BuckDirectory {
            path: path.clone(),
            targets: Vec::new(),
            has_buck_file,
            targets_loaded: false,
            targets_loading: false,
        };
        self.directories.insert(path.clone(), new_dir);
    }
}
