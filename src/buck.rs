use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuckTarget {
    pub name: String,
    pub rule_type: String,
    pub path: PathBuf,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone)]
struct TargetDetails {
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

impl BuckDirectory {
    fn abs_path(&self) -> PathBuf {
        self.path.canonicalize().unwrap_or(self.path.clone())
    }
}

pub struct BuckProject {
    pub root_path: PathBuf,
    pub directories: Vec<BuckDirectory>,
    pub all_targets: Vec<BuckTarget>,
    pub selected_directory: usize,
    pub selected_target: usize,
    pub search_query: String,
    pub filtered_targets: Vec<BuckTarget>,
    pub cells: HashMap<String, PathBuf>,
    pub target_loader_tx: Option<mpsc::UnboundedSender<(usize, PathBuf, CancellationToken)>>,
    pub target_result_rx: Option<mpsc::UnboundedReceiver<(usize, Result<Vec<BuckTarget>>)>>,
    current_load_token: Option<CancellationToken>,
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

        // Spawn background task for loading targets
        tokio::spawn(Self::target_loader_task(loader_rx, result_tx));

        let mut project = Self {
            root_path,
            directories: Vec::new(),
            all_targets: Vec::new(),
            selected_directory: 0,
            selected_target: 0,
            search_query: String::new(),
            filtered_targets: Vec::new(),
            cells: HashMap::new(),
            target_loader_tx: Some(loader_tx),
            target_result_rx: Some(result_rx),
            current_load_token: None,
        };

        project.scan_directories().await?;
        project.load_cells().await?;
        project.update_filtered_targets();

        // Request targets for the initial directory
        if !project.directories.is_empty() {
            project.request_targets_for_directory(0);
        }

        Ok(project)
    }

    async fn scan_directories(&mut self) -> Result<()> {
        for entry in WalkDir::new(&self.root_path)
            .min_depth(0)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let buck_file = entry.path().join("BUCK");
                let buck2_file = entry.path().join("BUCK2");
                let has_buck_file = buck_file.exists() || buck2_file.exists();

                self.directories.push(BuckDirectory {
                    path: entry.path().to_path_buf(),
                    targets: Vec::new(),
                    has_buck_file,
                    targets_loaded: false,
                    targets_loading: false,
                });
            }
        }
        Ok(())
    }

    async fn target_loader_task(
        mut loader_rx: mpsc::UnboundedReceiver<(usize, PathBuf, CancellationToken)>,
        result_tx: mpsc::UnboundedSender<(usize, Result<Vec<BuckTarget>>)>,
    ) {
        while let Some((dir_index, path, cancel_token)) = loader_rx.recv().await {
            let result = tokio::select! {
                _ = cancel_token.cancelled() => {
                    continue; // Skip if cancelled
                }
                result = Self::load_targets_from_directory_static(&path) => {
                    result
                }
            };

            if !cancel_token.is_cancelled() {
                let _ = result_tx.send((dir_index, result));
            }
        }
    }

    pub fn request_targets_for_directory(&mut self, dir_index: usize) {
        if dir_index >= self.directories.len() {
            return;
        }

        let dir = &mut self.directories[dir_index];
        if dir.targets_loaded || dir.targets_loading || !dir.has_buck_file {
            return;
        }

        // Cancel previous request if any
        if let Some(token) = &self.current_load_token {
            // TODO: trigger BuckDirectory.targets_loading to false
            token.cancel();
        }

        // Create new cancellation token
        let token = CancellationToken::new();
        self.current_load_token = Some(token.clone());

        // Mark as loading
        dir.targets_loading = true;

        // Send request to background task
        if let Some(tx) = &self.target_loader_tx {
            let _ = tx.send((dir_index, dir.path.clone(), token));
        }
    }

    pub fn update_loaded_target_results(&mut self) {
        let mut results_to_process = Vec::new();

        if let Some(rx) = &mut self.target_result_rx {
            while let Ok((dir_index, result)) = rx.try_recv() {
                results_to_process.push((dir_index, result));
            }
        }

        for (dir_index, result) in results_to_process {
            if dir_index < self.directories.len() {
                let dir = &mut self.directories[dir_index];
                dir.targets_loading = false;

                let should_update_filtered = dir_index == self.selected_directory;

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

                // Update filtered targets if this is the selected directory
                if should_update_filtered {
                    self.update_filtered_targets();
                }
            }
        }
    }

    async fn load_targets_from_directory_static(dir_path: &Path) -> Result<Vec<BuckTarget>> {
        // Use buck2 targets command to get actual target information
        let output = Command::new("buck2")
            .arg("targets")
            .arg(":")
            .current_dir(dir_path)
            .output()?;

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
                    eprintln!("Warning: Failed to parse buck2 audit cell output: {}", e);
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

            // Buck2 targets output format: //path/to/target:target_name
            if let Some(colon_pos) = line.rfind(':') {
                let target_name = &line[colon_pos + 1..];

                // Try to get more details about the target
                if let Ok(details) = Self::get_target_details_static(line) {
                    targets.push(BuckTarget {
                        name: target_name.to_string(),
                        rule_type: details.rule_type,
                        path: dir_path.to_path_buf(),
                        deps: details.deps,
                    });
                } else {
                    // Fallback with basic info
                    targets.push(BuckTarget {
                        name: target_name.to_string(),
                        rule_type: "unknown (error)".to_string(),
                        path: dir_path.to_path_buf(),
                        deps: Vec::new(),
                    });
                }
            }
        }

        Ok(targets)
    }

    fn get_target_details_static(target_label: &str) -> Result<TargetDetails> {
        // Try to get detailed information about the target
        let output = Command::new("buck2")
            .arg("query")
            .arg("-A")
            .arg(target_label)
            .output();

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
        // Get targets from the currently selected directory
        let selected_dir_targets = if let Some(selected_dir) = self.get_selected_directory() {
            selected_dir.targets.clone()
        } else {
            Vec::new()
        };

        if self.search_query.is_empty() {
            self.filtered_targets = selected_dir_targets;
        } else {
            self.filtered_targets = selected_dir_targets
                .iter()
                .filter(|target| {
                    target
                        .name
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

        // Reset selected target when directory changes
        self.selected_target = 0;
    }

    pub fn get_selected_directory(&self) -> Option<&BuckDirectory> {
        self.directories.get(self.selected_directory)
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
            Some(format!("{}//", cell))
        } else {
            // Convert path separators to forward slashes for Buck format
            let path_str = relative_path.to_string_lossy().replace('\\', "/");
            Some(format!("{}//{}", cell, path_str))
        }
    }

    pub fn get_selected_target(&self) -> Option<&BuckTarget> {
        self.filtered_targets.get(self.selected_target)
    }

    pub fn next_directory(&mut self) {
        if !self.directories.is_empty() {
            self.selected_directory = (self.selected_directory + 1) % self.directories.len();
            self.update_filtered_targets();
            self.request_targets_for_directory(self.selected_directory);
        }
    }

    pub fn prev_directory(&mut self) {
        if !self.directories.is_empty() {
            self.selected_directory = if self.selected_directory > 0 {
                self.selected_directory - 1
            } else {
                self.directories.len() - 1
            };
            self.update_filtered_targets();
            self.request_targets_for_directory(self.selected_directory);
        }
    }

    pub fn next_target(&mut self) {
        if !self.filtered_targets.is_empty() {
            self.selected_target = (self.selected_target + 1) % self.filtered_targets.len();
        }
    }

    pub fn prev_target(&mut self) {
        if !self.filtered_targets.is_empty() {
            self.selected_target = if self.selected_target > 0 {
                self.selected_target - 1
            } else {
                self.filtered_targets.len() - 1
            };
        }
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.update_filtered_targets();
    }
}
