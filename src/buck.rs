use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuckTarget {
    pub name: String,
    pub rule_type: String,
    pub path: PathBuf,
    pub deps: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuckDirectory {
    pub path: PathBuf,
    pub targets: Vec<BuckTarget>,
}

pub struct BuckProject {
    pub root_path: PathBuf,
    pub directories: Vec<BuckDirectory>,
    pub all_targets: Vec<BuckTarget>,
    pub selected_directory: usize,
    pub selected_target: usize,
    pub search_query: String,
    pub filtered_targets: Vec<BuckTarget>,
}

impl BuckProject {
    pub async fn new(project_path: String) -> Result<Self> {
        let root_path = PathBuf::from(project_path);
        
        if !root_path.exists() {
            return Err(anyhow!("Project path does not exist: {}", root_path.display()));
        }

        let mut project = Self {
            root_path,
            directories: Vec::new(),
            all_targets: Vec::new(),
            selected_directory: 0,
            selected_target: 0,
            search_query: String::new(),
            filtered_targets: Vec::new(),
        };

        project.scan_directories().await?;
        project.load_targets().await?;
        project.update_filtered_targets();

        Ok(project)
    }

    async fn scan_directories(&mut self) -> Result<()> {
        for entry in WalkDir::new(&self.root_path)
            .min_depth(1)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let buck_file = entry.path().join("BUCK");
                let buck2_file = entry.path().join("BUCK2");
                
                if buck_file.exists() || buck2_file.exists() {
                    self.directories.push(BuckDirectory {
                        path: entry.path().to_path_buf(),
                        targets: Vec::new(),
                    });
                }
            }
        }
        Ok(())
    }

    async fn load_targets(&mut self) -> Result<()> {
        let mut all_targets = Vec::new();
        let paths: Vec<PathBuf> = self.directories.iter().map(|d| d.path.clone()).collect();
        
        for (i, path) in paths.iter().enumerate() {
            let targets = self.load_targets_from_directory(path).await?;
            self.directories[i].targets = targets.clone();
            all_targets.extend(targets);
        }
        self.all_targets = all_targets;
        Ok(())
    }

    async fn load_targets_from_directory(&self, dir_path: &Path) -> Result<Vec<BuckTarget>> {
        let buck_file = dir_path.join("BUCK");
        let buck2_file = dir_path.join("BUCK2");
        
        let file_to_read = if buck2_file.exists() {
            buck2_file
        } else if buck_file.exists() {
            buck_file
        } else {
            return Ok(Vec::new());
        };

        let content = fs::read_to_string(&file_to_read).await?;
        let targets = self.parse_buck_file(&content, dir_path)?;
        Ok(targets)
    }

    fn parse_buck_file(&self, content: &str, dir_path: &Path) -> Result<Vec<BuckTarget>> {
        let mut targets = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }
            
            if let Some(name_start) = line.find("name = \"") {
                let name_start = name_start + 8;
                if let Some(name_end) = line[name_start..].find('"') {
                    let name = &line[name_start..name_start + name_end];
                    
                    let rule_type = if line.contains("rust_binary") {
                        "rust_binary"
                    } else if line.contains("rust_library") {
                        "rust_library"
                    } else if line.contains("rust_test") {
                        "rust_test"
                    } else if line.contains("java_binary") {
                        "java_binary"
                    } else if line.contains("java_library") {
                        "java_library"
                    } else {
                        "unknown"
                    };

                    targets.push(BuckTarget {
                        name: name.to_string(),
                        rule_type: rule_type.to_string(),
                        path: dir_path.to_path_buf(),
                        deps: Vec::new(),
                        outputs: Vec::new(),
                    });
                }
            }
        }
        
        Ok(targets)
    }

    pub fn update_filtered_targets(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_targets = self.all_targets.clone();
        } else {
            self.filtered_targets = self
                .all_targets
                .iter()
                .filter(|target| {
                    target.name.to_lowercase().contains(&self.search_query.to_lowercase())
                        || target.rule_type.to_lowercase().contains(&self.search_query.to_lowercase())
                })
                .cloned()
                .collect();
        }
    }

    pub fn get_selected_directory(&self) -> Option<&BuckDirectory> {
        self.directories.get(self.selected_directory)
    }

    pub fn get_selected_target(&self) -> Option<&BuckTarget> {
        self.filtered_targets.get(self.selected_target)
    }

    pub fn next_directory(&mut self) {
        if !self.directories.is_empty() {
            self.selected_directory = (self.selected_directory + 1) % self.directories.len();
        }
    }

    pub fn prev_directory(&mut self) {
        if !self.directories.is_empty() {
            self.selected_directory = if self.selected_directory > 0 {
                self.selected_directory - 1
            } else {
                self.directories.len() - 1
            };
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
        self.selected_target = 0;
    }
}