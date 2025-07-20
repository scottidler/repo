use clap::{Parser, Subcommand, ValueEnum};
use eyre::{Result, WrapErr};
use log::{debug, info, warn};
use rand::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Once;
use uuid::Uuid;

static INIT: Once = Once::new();

// Get the git version from build.rs
const GIT_VERSION: &str = env!("GIT_DESCRIBE");

#[derive(Parser)]
#[command(name = "repo")]
#[command(about = "A Git workflow simulation tool")]
#[command(version = GIT_VERSION)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new repository
    Init {
        /// Repository name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Create files
    Create {
        /// Number of files to create
        #[arg(default_value = "3")]
        count: u32,
        /// Specific filename
        #[arg(short, long)]
        filename: Option<String>,
        /// File content
        #[arg(long)]
        content: Option<String>,
    },
    /// Modify existing files
    Modify {
        /// File path to modify
        #[arg(short, long)]
        filepath: Option<String>,
        /// Line number to modify
        #[arg(short, long)]
        lineno: Option<usize>,
        /// Type of modification
        #[arg(short, long, default_value = "append")]
        modify_type: String,
    },
    /// Change files (create and modify)
    Change {
        /// Number of changes to make
        #[arg(short, long, default_value = "1")]
        count: u32,
    },
    /// Create a new branch
    Branch {
        /// Branch name (random if not provided)
        #[arg(short, long)]
        name: Option<String>,
        /// Force branch creation
        #[arg(short, long)]
        force: bool,
        /// Delete the branch
        #[arg(short, long)]
        delete: bool,
    },
    /// Commit changes
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
        /// Amend the last commit
        #[arg(short, long)]
        amend: bool,
    },
    /// Create merge conflicts
    Conflict {
        /// File to create conflict in
        #[arg(short, long)]
        filename: Option<String>,
        /// Initial content
        #[arg(short, long)]
        content: Option<String>,
        /// Type of conflict to create
        #[arg(short = 't', long, default_value = "content")]
        conflict_type: ConflictType,
    },
    /// Reset repository state
    Reset,
    /// Merge branches (placeholder)
    Merge,
    /// Munge repository (placeholder)
    Munge,
    /// Rebase branches (placeholder)
    Rebase,
}

#[derive(Clone, Debug)]
pub enum ModifyType {
    Append,
    Prepend,
    Prefix,
    Suffix,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ConflictType {
    /// Simple content conflict - same lines modified differently
    Content,
    /// Delete/modify conflict - file deleted in one branch, modified in another
    DeleteModify,
    /// Rename conflict - same file renamed differently in both branches
    Rename,
    /// Add/add conflict - same filename added with different content
    AddAdd,
    /// Binary file conflict - binary files modified differently
    Binary,
    /// Mode conflict - file permissions changed differently
    Mode,
    /// Whitespace conflict - different whitespace/formatting
    Whitespace,
    /// Case sensitivity conflict - filename case differences
    Case,
    /// Structural conflict - file organization changes
    Structural,
}

pub struct RepoTool {
    pub home_branch: String,
    pub verbose: bool,
    pub command_count: u32,
    pub words: Vec<String>,
    pub working_directory: Option<PathBuf>,
}

impl RepoTool {
    pub fn new(home_branch: String, verbose: bool) -> Result<Self> {
        INIT.call_once(|| {
            let _ = env_logger::try_init();
        });

        let words = Self::load_words()?;

        Ok(RepoTool {
            home_branch,
            verbose,
            command_count: 0,
            words,
            working_directory: None,
        })
    }

    pub fn new_in_directory(home_branch: String, verbose: bool, working_directory: PathBuf) -> Result<Self> {
        let mut tool = Self::new(home_branch, verbose)?;
        tool.working_directory = Some(working_directory);
        Ok(tool)
    }

    fn load_words() -> Result<Vec<String>> {
        let word_files = ["/etc/words", "/usr/share/dict/words"];

        for word_file in &word_files {
            if Path::new(word_file).exists() {
                let content = fs::read_to_string(word_file)
                    .wrap_err_with(|| format!("Failed to read word file: {}", word_file))?;

                let words: Vec<String> = content
                    .lines()
                    .filter(|line| !line.is_empty() && !line.contains('\''))
                    .map(|line| line.to_lowercase())
                    .collect();

                if !words.is_empty() {
                    info!("Loaded {} words from {}", words.len(), word_file);
                    return Ok(words);
                }
            }
        }

        // Fallback to a small built-in word list
        warn!("No system word file found, using built-in words");
        Ok(vec![
            "apple".to_string(), "banana".to_string(), "cherry".to_string(),
            "dog".to_string(), "elephant".to_string(), "fox".to_string(),
            "grape".to_string(), "house".to_string(), "ice".to_string(),
            "jungle".to_string(), "kite".to_string(), "lemon".to_string(),
        ])
    }

    fn run_git(&mut self, args: &[&str]) -> Result<Output> {
        self.command_count += 1;

        let cmd = "git";
        if self.verbose {
            println!("#{}) {} {}", self.command_count, cmd, args.join(" "));
        }

        let mut full_args = vec!["--no-pager"];

        // Add -C flag if we have a working directory
        if let Some(ref work_dir) = self.working_directory {
            full_args.push("-C");
            full_args.push(work_dir.to_str().unwrap());
        }

        full_args.extend_from_slice(args);

        let output = Command::new(cmd)
            .args(full_args)
            .output()
            .wrap_err_with(|| format!("Failed to execute: {} {}", cmd, args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre::eyre!("Git command failed: {}", stderr));
        }

        Ok(output)
    }

    fn gen_word(&self) -> String {
        self.words.choose(&mut rand::rng()).unwrap().clone()
    }

    fn gen_words(&self, count: u32) -> Vec<String> {
        (0..count).map(|_| self.gen_word()).collect()
    }

    fn gen_filepath(&self, max_depth: u32, min_depth: u32, prefix: Option<&str>) -> PathBuf {
        let depth = rand::rng().random_range(min_depth..=max_depth);
        let words = self.gen_words(depth);

        let mut path = PathBuf::new();
        if let Some(p) = prefix {
            path.push(p);
        }
        // Don't add "src" by default - let create_file handle the src directory

        for word in words {
            path.push(word);
        }

        // Add a file extension
        if let Some(filename) = path.file_name() {
            let mut filename = filename.to_string_lossy().to_string();
            filename.push_str(".txt");
            path.set_file_name(filename);
        }

        path
    }

    fn gen_content(&self, max_lines: u32, min_lines: u32) -> String {
        let line_count = rand::rng().random_range(min_lines..=max_lines);
        let words_per_line = rand::rng().random_range(1..=8);

        (0..line_count)
            .map(|_| self.gen_words(words_per_line).join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn is_in_repo(&mut self) -> bool {
        self.run_git(&["rev-parse", "--git-dir"]).is_ok()
    }

    fn get_current_branch(&mut self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_src_path(&self) -> Result<PathBuf> {
        let base_path = if let Some(ref work_dir) = self.working_directory {
            work_dir.clone()
        } else {
            env::current_dir().wrap_err("Failed to get current directory")?
        };

        let src_path = base_path.join("src");
        if !src_path.exists() {
            fs::create_dir_all(&src_path)
                .wrap_err_with(|| format!("Failed to create src directory: {:?}", src_path))?;
        }
        Ok(src_path)
    }

    fn ensure_src_dir(&mut self) -> Result<PathBuf> {
        let src_path = self.get_src_path()?;
        fs::create_dir_all(&src_path)
            .wrap_err_with(|| format!("Failed to create src directory: {:?}", src_path))?;
        Ok(src_path)
    }

    fn find_files_in_src(&mut self) -> Result<Vec<PathBuf>> {
        let src_path = self.get_src_path()?;
        if !src_path.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        files.push(path);
                    } else if path.is_dir() {
                        visit_dir(&path, files)?;
                    }
                }
            }
            Ok(())
        }

        visit_dir(&src_path, &mut files)?;
        Ok(files)
    }

    fn get_random_file(&mut self) -> Result<Option<PathBuf>> {
        let files = self.find_files_in_src()?;
        if files.is_empty() {
            Ok(None)
        } else {
            Ok(Some(files.choose(&mut rand::rng()).unwrap().clone()))
        }
    }

    fn git_add_src(&mut self) -> Result<()> {
        let src_path = self.get_src_path()?;
        self.run_git(&["add", src_path.to_str().unwrap()])?;
        Ok(())
    }

    fn git_status(&mut self) -> Result<Vec<String>> {
        let src_path = self.get_src_path()?;
        let output = self.run_git(&["status", "-s", src_path.to_str().unwrap()])?;
        let status = String::from_utf8_lossy(&output.stdout);
        Ok(status.lines().map(|s| s.to_string()).collect())
    }

    // Command implementations
    pub fn init(&mut self, repo_name: Option<String>) -> Result<()> {
        let repo_name = repo_name.unwrap_or_else(|| format!("repo-{}", Uuid::new_v4()));

        let repo_path = if let Some(ref work_dir) = self.working_directory {
            work_dir.join(&repo_name)
        } else {
            env::current_dir()?.join(&repo_name)
        };

        // Create the repository directory
        fs::create_dir_all(&repo_path)
            .wrap_err_with(|| format!("Failed to create repository directory: {:?}", repo_path))?;

        // Update our working directory to the new repo
        self.working_directory = Some(repo_path.clone());

        // Initialize git repository
        self.run_git(&["init"])?;

        info!("Initialized repository: {}", repo_name);
        println!("Initialized repository: {}", repo_name);

        Ok(())
    }

    pub fn branch(&mut self, branch_name: Option<String>, home: bool, commit: bool) -> Result<()> {
        if home {
            let home_branch = self.home_branch.clone();
            info!("Switching to home branch: {}", home_branch);
            self.run_git(&["checkout", &home_branch])?;
        } else {
            let name = if let Some(name) = branch_name {
                name
            } else {
                let word = self.gen_word();
                format!("dev/{}", word)
            };
            info!("Creating branch: {}", name);
            self.run_git(&["checkout", "-b", &name])?;
        }

        if commit {
            self.commit(None, false)?;
        }

        Ok(())
    }

    pub fn change(&mut self, count: u32) -> Result<()> {
        let actual_count = if count == 0 {
            rand::rng().random_range(1..=5)
        } else {
            count
        };

        info!("Creating {} changes", actual_count);

        for i in 0..actual_count {
            debug!("Creating change {}/{}", i + 1, actual_count);

            let files = self.find_files_in_src()?;
            if files.is_empty() || rand::rng().random_bool(0.7) {
                // Create new file
                self.create(1, None, None)?;
            } else {
                // Modify existing file
                self.modify(None, None, ModifyType::Append)?;
            }
        }

        Ok(())
    }

    pub fn commit(&mut self, commit_name: Option<String>, branch: bool) -> Result<()> {
        if branch {
            self.branch(None, false, false)?;
        }

        let name = commit_name.unwrap_or_else(|| self.gen_word());
        info!("Creating commit: {}", name);

        // Ensure we have changes to commit
        let status = self.git_status()?;
        if status.is_empty() {
            debug!("No changes found, creating some");
            self.change(rand::rng().random_range(1..=3))?;
        }

        self.git_add_src()?;

        let changes = self.git_status()?;
        let change_summary = changes.join("\n  ");
        let commit_msg = format!("'{}' commit message for:\n  {}", name, change_summary);

        self.run_git(&["commit", "-m", &commit_msg])?;

        println!("Created commit: {}", name);
        Ok(())
    }

    pub fn conflict(&mut self, filepath: Option<String>, content: Option<String>, conflict_type: ConflictType) -> Result<()> {
        info!("Creating {} conflict scenario", format!("{:?}", conflict_type).to_lowercase());

        match conflict_type {
            ConflictType::Content => self.create_content_conflict(filepath, content),
            ConflictType::DeleteModify => self.create_delete_modify_conflict(filepath, content),
            ConflictType::Rename => self.create_rename_conflict(filepath, content),
            ConflictType::AddAdd => self.create_add_add_conflict(filepath, content),
            ConflictType::Binary => self.create_binary_conflict(filepath, content),
            ConflictType::Mode => self.create_mode_conflict(filepath, content),
            ConflictType::Whitespace => self.create_whitespace_conflict(filepath, content),
            ConflictType::Case => self.create_case_conflict(filepath, content),
            ConflictType::Structural => self.create_structural_conflict(filepath, content),
        }
    }

    fn create_content_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            self.gen_filepath(3, 1, None).to_string_lossy().to_string()
        });
        let initial_content = content.unwrap_or_else(|| self.gen_content(3, 1));

        // Get current branch
        let original_branch = self.get_current_branch()?;

        // Create initial file and commit
        self.create_file(&path, &initial_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial content for conflict"])?;

        // Create new branch and modify the file
        let conflict_branch = format!("conflict-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let modified_content = format!("{} {}", initial_content, self.gen_word());
        self.create_file(&path, &modified_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Modified content on conflict branch"])?;

        // Switch back to original branch and make conflicting change
        self.run_git(&["checkout", &original_branch])?;
        let conflicting_content = format!("{} {}", initial_content, self.gen_word());
        self.create_file(&path, &conflicting_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Conflicting content on original branch"])?;

        println!("Created content conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_delete_modify_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            self.gen_filepath(3, 1, None).to_string_lossy().to_string()
        });
        let initial_content = content.unwrap_or_else(|| self.gen_content(3, 1));

        let original_branch = self.get_current_branch()?;

        // Create initial file and commit
        self.create_file(&path, &initial_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial file for delete/modify conflict"])?;

        // Create new branch and delete the file
        let conflict_branch = format!("delete-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let src_path = self.get_src_path()?;
        let full_path = src_path.join(&path);
        fs::remove_file(&full_path).wrap_err_with(|| format!("Failed to delete file: {:?}", full_path))?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Deleted file on conflict branch"])?;

        // Switch back and modify the file
        self.run_git(&["checkout", &original_branch])?;
        let modified_content = format!("{}\n{}", initial_content, self.gen_content(2, 1));
        self.create_file(&path, &modified_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Modified file on original branch"])?;

        println!("Created delete/modify conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_rename_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let original_path = filepath.unwrap_or_else(|| {
            self.gen_filepath(3, 1, None).to_string_lossy().to_string()
        });
        let initial_content = content.unwrap_or_else(|| self.gen_content(3, 1));

        let original_branch = self.get_current_branch()?;

        // Create initial file and commit
        self.create_file(&original_path, &initial_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial file for rename conflict"])?;

        // Create new branch and rename file one way
        let conflict_branch = format!("rename-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let new_name1 = format!("{}-{}.txt", self.gen_word(), "version1");
        let src_path = self.get_src_path()?;
        let old_full_path = src_path.join(&original_path);
        let new_full_path1 = src_path.join(&new_name1);

        fs::rename(&old_full_path, &new_full_path1).wrap_err_with(|| format!("Failed to rename file from {:?} to {:?}", old_full_path, new_full_path1))?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Renamed file on conflict branch"])?;

        // Switch back and rename file differently
        self.run_git(&["checkout", &original_branch])?;
        let new_name2 = format!("{}-{}.txt", self.gen_word(), "version2");
        let old_full_path2 = src_path.join(&original_path);
        let new_full_path2 = src_path.join(&new_name2);

        fs::rename(&old_full_path2, &new_full_path2).wrap_err_with(|| format!("Failed to rename file from {:?} to {:?}", old_full_path2, new_full_path2))?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Renamed file differently on original branch"])?;

        println!("Created rename conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("File renamed to '{}' on {} and '{}' on {}", new_name1, conflict_branch, new_name2, original_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_add_add_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            format!("shared-{}.txt", self.gen_word())
        });
        let base_content = content.unwrap_or_else(|| "Base content".to_string());

        let original_branch = self.get_current_branch()?;

        // Create new branch and add file with one content
        let conflict_branch = format!("add-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let content1 = format!("{}\nContent added on branch {}", base_content, conflict_branch);
        self.create_file(&path, &content1)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Added file on conflict branch"])?;

        // Switch back and add same file with different content
        self.run_git(&["checkout", &original_branch])?;
        let content2 = format!("{}\nContent added on branch {}", base_content, original_branch);
        self.create_file(&path, &content2)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Added same file on original branch"])?;

        println!("Created add/add conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("Same file '{}' added with different content on both branches", path);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_binary_conflict(&mut self, filepath: Option<String>, _content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            format!("binary-{}.bin", self.gen_word())
        });

        let original_branch = self.get_current_branch()?;

        // Create initial binary file and commit
        let binary_data1: Vec<u8> = (0..50).map(|i| (i * 3) as u8).collect();
        let src_path = self.get_src_path()?;
        let full_path = src_path.join(&path);
        fs::write(&full_path, &binary_data1).wrap_err_with(|| format!("Failed to write binary file: {:?}", full_path))?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial binary file"])?;

        // Create new branch and modify binary file
        let conflict_branch = format!("binary-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let binary_data2: Vec<u8> = (0..50).map(|i| (i * 5) as u8).collect();
        fs::write(&full_path, &binary_data2).wrap_err_with(|| format!("Failed to write binary file: {:?}", full_path))?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Modified binary file on conflict branch"])?;

        // Switch back and modify binary file differently
        self.run_git(&["checkout", &original_branch])?;
        let binary_data3: Vec<u8> = (0..50).map(|i| (i * 7) as u8).collect();
        fs::write(&full_path, &binary_data3).wrap_err_with(|| format!("Failed to write binary file: {:?}", full_path))?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Modified binary file on original branch"])?;

        println!("Created binary conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("Binary file '{}' modified differently on both branches", path);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_mode_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            format!("script-{}.sh", self.gen_word())
        });
        let initial_content = content.unwrap_or_else(|| {
            format!("#!/bin/bash\necho \"Hello from {}\"\n", self.gen_word())
        });

        let original_branch = self.get_current_branch()?;

        // Create initial file and commit
        self.create_file(&path, &initial_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial script file"])?;

        // Create new branch and make file executable
        let conflict_branch = format!("mode-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let src_path = self.get_src_path()?;
        let full_path = src_path.join(&path);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&full_path)?.permissions();
            perms.set_mode(0o755); // Make executable
            fs::set_permissions(&full_path, perms)?;
        }

        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Made script executable on conflict branch"])?;

        // Switch back and modify content (but not permissions)
        self.run_git(&["checkout", &original_branch])?;
        let modified_content = format!("{}\necho \"Additional line added\"", initial_content);
        self.create_file(&path, &modified_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Modified script content on original branch"])?;

        println!("Created mode conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("File permissions changed on {} while content changed on {}", conflict_branch, original_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_whitespace_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            format!("whitespace-{}.txt", self.gen_word())
        });
        let base_content = content.unwrap_or_else(|| {
            "Line 1\nLine 2\nLine 3".to_string()
        });

        let original_branch = self.get_current_branch()?;

        // Create initial file and commit
        self.create_file(&path, &base_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial file with whitespace"])?;

        // Create new branch and add trailing spaces
        let conflict_branch = format!("whitespace-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let content_with_spaces = base_content.lines()
            .map(|line| format!("{}   ", line)) // Add trailing spaces
            .collect::<Vec<_>>()
            .join("\n");
        self.create_file(&path, &content_with_spaces)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Added trailing whitespace on conflict branch"])?;

        // Switch back and change indentation
        self.run_git(&["checkout", &original_branch])?;
        let content_with_tabs = base_content.lines()
            .map(|line| format!("\t{}", line)) // Add tabs at beginning
            .collect::<Vec<_>>()
            .join("\n");
        self.create_file(&path, &content_with_tabs)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Added tab indentation on original branch"])?;

        println!("Created whitespace conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("Trailing spaces added on {} while tab indentation added on {}", conflict_branch, original_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_case_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let base_name = filepath.unwrap_or_else(|| {
            format!("CaseFile-{}.txt", self.gen_word())
        });
        let initial_content = content.unwrap_or_else(|| self.gen_content(3, 1));

        let original_branch = self.get_current_branch()?;

        // Create initial file and commit
        self.create_file(&base_name, &initial_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial file with mixed case name"])?;

        // Create new branch and rename to lowercase
        let conflict_branch = format!("case-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let lowercase_name = base_name.to_lowercase();
        let src_path = self.get_src_path()?;
        let old_path = src_path.join(&base_name);
        let new_path = src_path.join(&lowercase_name);

        // On case-insensitive filesystems, we need to do this in two steps
        let temp_name = format!("temp-{}", base_name);
        let temp_path = src_path.join(&temp_name);
        fs::rename(&old_path, &temp_path)?;
        fs::rename(&temp_path, &new_path)?;

        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Renamed file to lowercase on conflict branch"])?;

        // Switch back and rename to uppercase
        self.run_git(&["checkout", &original_branch])?;
        let uppercase_name = base_name.to_uppercase();
        let old_path2 = src_path.join(&base_name);
        let new_path2 = src_path.join(&uppercase_name);

        let temp_name2 = format!("temp2-{}", base_name);
        let temp_path2 = src_path.join(&temp_name2);
        fs::rename(&old_path2, &temp_path2)?;
        fs::rename(&temp_path2, &new_path2)?;

        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Renamed file to uppercase on original branch"])?;

        println!("Created case conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("File renamed to '{}' on {} and '{}' on {}", lowercase_name, conflict_branch, uppercase_name, original_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    fn create_structural_conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let path = filepath.unwrap_or_else(|| {
            "shared/data.txt".to_string()
        });
        let initial_content = content.unwrap_or_else(|| self.gen_content(3, 1));

        let original_branch = self.get_current_branch()?;

        // Create initial file in a directory and commit
        self.create_file(&path, &initial_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Initial file in directory"])?;

        // Create new branch and move file to different directory structure
        let conflict_branch = format!("struct-{}", self.gen_word());
        self.run_git(&["checkout", "-b", &conflict_branch])?;

        let new_path = format!("moved/{}/{}", self.gen_word(), Path::new(&path).file_name().unwrap().to_string_lossy());
        let src_path = self.get_src_path()?;
        let old_full_path = src_path.join(&path);
        let new_full_path = src_path.join(&new_path);

        // Create new directory structure
        if let Some(parent) = new_full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(&old_full_path, &new_full_path)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Moved file to new directory structure"])?;

        // Switch back and modify original file
        self.run_git(&["checkout", &original_branch])?;
        let modified_content = format!("{}\n{}", initial_content, self.gen_content(2, 1));
        self.create_file(&path, &modified_content)?;
        self.git_add_src()?;
        self.run_git(&["commit", "-m", "Modified file in original location"])?;

        println!("Created structural conflict scenario between {} and {}", original_branch, conflict_branch);
        println!("File moved to '{}' on {} while modified in place on {}", new_path, conflict_branch, original_branch);
        println!("To see conflict: git merge {}", conflict_branch);

        Ok(())
    }

    pub fn create(&mut self, count: u32, filepath: Option<String>, content: Option<String>) -> Result<()> {
        let actual_count = if count == 0 { 1 } else { count };

        for i in 0..actual_count {
            let path = if let Some(ref fp) = filepath {
                PathBuf::from(fp)
            } else {
                self.gen_filepath(3, 1, None)
            };

            let file_content = if let Some(ref c) = content {
                c.clone()
            } else {
                self.gen_content(5, 1)
            };

            debug!("Creating file {}/{}: {:?}", i + 1, actual_count, path);
            self.create_file(path.to_str().unwrap(), &file_content)?;
        }

        Ok(())
    }

    fn create_file(&mut self, filepath: &str, content: &str) -> Result<()> {
        let path = Path::new(filepath);

        // Ensure it's in the src directory
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.ensure_src_dir()?.join(path)
        };

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("Failed to create parent directories for: {:?}", full_path))?;
        }

        fs::write(&full_path, content)
            .wrap_err_with(|| format!("Failed to write file: {:?}", full_path))?;

        info!("Created file: {:?}", full_path);
        Ok(())
    }

    pub fn modify(&mut self, filepath: Option<String>, lineno: Option<usize>, modify_type: ModifyType) -> Result<()> {
        let file_path = if let Some(fp) = filepath {
            PathBuf::from(fp)
        } else {
            self.get_random_file()?.ok_or_else(|| eyre::eyre!("No files found to modify"))?
        };

        let content = fs::read_to_string(&file_path)
            .wrap_err_with(|| format!("Failed to read file: {:?}", file_path))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if lines.is_empty() {
            lines.push(String::new());
        }

        let line_idx = if let Some(ln) = lineno {
            if ln == 0 || ln > lines.len() {
                return Err(eyre::eyre!("Line number {} is out of range (1-{})", ln, lines.len()));
            }
            ln - 1 // Convert to 0-based index
        } else {
            rand::rng().random_range(0..lines.len())
        };

        let modification = self.gen_content(1, 1);
        let actual_modify_type = match modify_type {
            ModifyType::Append => ModifyType::Append,
            ModifyType::Prepend => ModifyType::Prepend,
            ModifyType::Prefix => ModifyType::Prefix,
            ModifyType::Suffix => ModifyType::Suffix,
        };

        match actual_modify_type {
            ModifyType::Append => {
                lines.insert(line_idx + 1, modification);
            }
            ModifyType::Prepend => {
                lines.insert(line_idx, modification);
            }
            ModifyType::Prefix => {
                lines[line_idx] = format!("{} {}", modification, lines[line_idx]);
            }
            ModifyType::Suffix => {
                lines[line_idx] = format!("{} {}", lines[line_idx], modification);
            }
        }

        let new_content = lines.join("\n");
        fs::write(&file_path, new_content)
            .wrap_err_with(|| format!("Failed to write modified file: {:?}", file_path))?;

        info!("Modified file: {:?} (type: {:?})", file_path, actual_modify_type);
        Ok(())
    }

    pub fn merge(&mut self) -> Result<()> {
        println!("Merge operation not yet implemented");
        Ok(())
    }

    pub fn munge(&mut self) -> Result<()> {
        println!("Munge operation not yet implemented");
        Ok(())
    }

    pub fn rebase(&mut self) -> Result<()> {
        println!("Rebase operation not yet implemented");
        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        info!("Resetting to home branch and cleaning");

        if self.is_in_repo() {
            // Try to detect the actual default branch first
            let default_branch = if let Ok(output) = self.run_git(&["symbolic-ref", "refs/remotes/origin/HEAD"]) {
                let remote_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Some(branch) = remote_ref.strip_prefix("refs/remotes/origin/") {
                    branch.to_string()
                } else {
                    self.home_branch.clone()
                }
            } else {
                // Fallback: try common default branch names
                let common_branches = ["main", "master"];
                let mut found_branch = self.home_branch.clone();

                if let Ok(output) = self.run_git(&["branch", "-a"]) {
                    let branches = String::from_utf8_lossy(&output.stdout);
                    for branch in common_branches {
                        if branches.contains(branch) {
                            found_branch = branch.to_string();
                            break;
                        }
                    }
                }
                found_branch
            };

            info!("Switching to branch: {}", default_branch);
            let result = self.run_git(&["checkout", &default_branch]);
            if result.is_err() {
                warn!("Failed to checkout {}, trying to create it", default_branch);
                // If the branch doesn't exist, try to create it
                self.run_git(&["checkout", "-b", &default_branch])?;
            }

            self.run_git(&["clean", "-fd"])?;
            println!("Reset to {} branch and cleaned working directory", default_branch);
        } else {
            println!("Not in a git repository");
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let mut tool = RepoTool::new("main".to_string(), false)?;

    match cli.command {
        Commands::Init { name } => tool.init(name),
        Commands::Branch { name, force: _, delete: _ } => tool.branch(name, false, false), // Placeholder for home/commit logic
        Commands::Change { count } => tool.change(count),
        Commands::Commit { message, amend: _ } => tool.commit(message, false), // Placeholder for branch logic
        Commands::Conflict { filename, content, conflict_type } => tool.conflict(filename, content, conflict_type),
        Commands::Create { count, filename, content } => tool.create(count, filename, content),
        Commands::Modify { filepath, lineno, modify_type } => {
            let modify_type_enum = match modify_type.as_str() {
                "append" => ModifyType::Append,
                "prepend" => ModifyType::Prepend,
                "prefix" => ModifyType::Prefix,
                "suffix" => ModifyType::Suffix,
                _ => ModifyType::Append, // Default to append if invalid
            };
            tool.modify(filepath, lineno, modify_type_enum)
        },
        Commands::Merge => tool.merge(),
        Commands::Munge => tool.munge(),
        Commands::Rebase => tool.rebase(),
        Commands::Reset => tool.reset(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper functions that create properly isolated test environments
    fn setup_git_repo() -> (TempDir, RepoTool) {
        let temp_dir = TempDir::new().unwrap();
        let mut tool = RepoTool::new_in_directory("main".to_string(), false, temp_dir.path().to_path_buf()).unwrap();
        tool.run_git(&["init"]).unwrap();
        tool.run_git(&["config", "user.name", "Test User"]).unwrap();
        tool.run_git(&["config", "user.email", "test@example.com"]).unwrap();
        (temp_dir, tool)
    }

    fn setup_git_repo_with_commit() -> (TempDir, RepoTool) {
        let (temp_dir, mut tool) = setup_git_repo();
        // Create initial commit
        tool.create(1, Some("initial.txt".to_string()), Some("initial content".to_string())).unwrap();
        tool.git_add_src().unwrap();
        tool.run_git(&["commit", "-m", "initial commit"]).unwrap();
        (temp_dir, tool)
    }

    #[test]
    fn test_repo_tool_creation() {
        let tool = RepoTool::new("main".to_string(), false).unwrap();
        assert_eq!(tool.home_branch, "main");
        assert!(!tool.verbose);
        assert_eq!(tool.command_count, 0);
        assert!(!tool.words.is_empty());
        assert!(tool.working_directory.is_none());
    }

    #[test]
    fn test_word_generation() {
        let tool = RepoTool::new("main".to_string(), false).unwrap();

        let word = tool.gen_word();
        assert!(!word.is_empty());
        assert!(word.chars().all(|c| c.is_lowercase() || c.is_numeric()));

        let words = tool.gen_words(5);
        assert_eq!(words.len(), 5);
        assert!(words.iter().all(|w| !w.is_empty()));
    }

    #[test]
    fn test_filepath_generation() {
        let tool = RepoTool::new("main".to_string(), false).unwrap();

        let path = tool.gen_filepath(3, 1, None);
        assert!(!path.to_string_lossy().is_empty());
        assert!(path.extension().is_some());

        let path_with_prefix = tool.gen_filepath(2, 1, Some("custom"));
        assert!(path_with_prefix.starts_with("custom"));
    }

    #[test]
    fn test_content_generation() {
        let tool = RepoTool::new("main".to_string(), false).unwrap();

        let content = tool.gen_content(3, 1);
        assert!(!content.is_empty());

        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() >= 1 && lines.len() <= 3);
        assert!(lines.iter().all(|line| !line.is_empty()));
    }

    #[test]
    fn test_init_command() {
        let temp_dir = TempDir::new().unwrap();
        let mut tool = RepoTool::new_in_directory("main".to_string(), false, temp_dir.path().to_path_buf()).unwrap();

        let result = tool.init(Some("test-repo".to_string()));
        assert!(result.is_ok());

        // Check that the repo was created in the temp directory
        let repo_path = temp_dir.path().join("test-repo");
        assert!(repo_path.exists());
        assert!(repo_path.join(".git").exists());
    }

    #[test]
    fn test_create_command() {
        let (_temp_dir, mut tool) = setup_git_repo();

        let result = tool.create(1, Some("test-file.txt".to_string()), Some("test content".to_string()));
        assert!(result.is_ok());

        let src_path = tool.get_src_path().unwrap();
        let file_path = src_path.join("test-file.txt");
        assert!(file_path.exists());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_create_multiple_files() {
        let (_temp_dir, mut tool) = setup_git_repo();

        let result = tool.create(3, None, None);
        assert!(result.is_ok());

        let files = tool.find_files_in_src().unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_branch_creation() {
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();

        let result = tool.branch(Some("feature-branch".to_string()), false, false);
        assert!(result.is_ok());

        let current_branch = tool.get_current_branch().unwrap();
        assert_eq!(current_branch, "feature-branch");
    }

    #[test]
    fn test_branch_with_random_name() {
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();

        let result = tool.branch(None, false, false);
        assert!(result.is_ok());

        let current_branch = tool.get_current_branch().unwrap();
        assert!(current_branch.starts_with("dev/"));
    }

    #[test]
    fn test_commit_creation() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // Create a file first so we have something to commit
        tool.create(1, Some("commit-test.txt".to_string()), Some("test content".to_string())).unwrap();
        tool.git_add_src().unwrap();

        let result = tool.commit(Some("test-commit".to_string()), false);
        assert!(result.is_ok());

        // Verify commit was created
        let output = tool.run_git(&["log", "--oneline", "-n", "1"]).unwrap();
        let log_output = String::from_utf8_lossy(&output.stdout);
        assert!(log_output.contains("test-commit"));
    }

    #[test]
    fn test_modify_existing_file() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // Create a file first
        tool.create(1, Some("modify-test.txt".to_string()), Some("line 1\nline 2\nline 3".to_string())).unwrap();

        let src_path = tool.get_src_path().unwrap();
        let file_path = src_path.join("modify-test.txt");

        let result = tool.modify(Some(file_path.to_string_lossy().to_string()), Some(2), ModifyType::Append);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() > 3); // Should have more lines after append
    }

    #[test]
    fn test_change_command() {
        let (_temp_dir, mut tool) = setup_git_repo();

        let result = tool.change(2);
        assert!(result.is_ok());

        let files = tool.find_files_in_src().unwrap();
        assert!(!files.is_empty());
    }

    #[test]
    fn test_conflict_creation() {
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();  // Use setup with commit

        let result = tool.conflict(Some("conflict-file.txt".to_string()), Some("initial content".to_string()), ConflictType::Content);
        assert!(result.is_ok());

        // Verify branches exist
        let output = tool.run_git(&["branch", "-a"]).unwrap();
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(branches.contains("conflict-"));
    }

    #[test]
    fn test_command_counting() {
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();  // Use setup with commit

        let initial_count = tool.command_count;

        tool.run_git(&["status"]).unwrap();
        assert_eq!(tool.command_count, initial_count + 1);

        tool.run_git(&["log", "--oneline"]).unwrap();
        assert_eq!(tool.command_count, initial_count + 2);
    }

    #[test]
    fn test_reset_command() {
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();

        // Create and switch to a new branch
        tool.branch(Some("test-branch".to_string()), false, false).unwrap();

        // Reset should go back to main branch
        let result = tool.reset();
        assert!(result.is_ok());

        let current_branch = tool.get_current_branch().unwrap();
        assert_eq!(current_branch, "main");
    }

    #[test]
    fn test_git_status() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // Create a file but don't add it yet
        tool.create(1, None, None).unwrap();

        let status = tool.git_status().unwrap();
        // Git status -s shows untracked directories as "?? dirname/"
        assert!(status.len() == 1 && status[0].starts_with("?? src"));

        // Now add it
        tool.git_add_src().unwrap();

        let status = tool.git_status().unwrap();
        assert!(!status.is_empty());
        assert!(status[0].starts_with("A "));
    }

    #[test]
    fn test_file_finding() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // Initially no files
        let files = tool.find_files_in_src().unwrap();
        assert!(files.is_empty());

        // Create some files
        tool.create(3, None, None).unwrap();

        let files = tool.find_files_in_src().unwrap();
        assert_eq!(files.len(), 3);

        let random_file = tool.get_random_file().unwrap();
        assert!(random_file.is_some());
    }

    #[test]
    fn test_modify_types() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // Test each modify type
        let modify_types = [
            ModifyType::Append,
            ModifyType::Prepend,
            ModifyType::Prefix,
            ModifyType::Suffix,
        ];

        for (i, modify_type) in modify_types.iter().enumerate() {
            let filename = format!("modify-test-{}.txt", i);
            tool.create(1, Some(filename.clone()), Some("original line".to_string())).unwrap();

            let src_path = tool.get_src_path().unwrap();
            let file_path = src_path.join(&filename);

            let result = tool.modify(Some(file_path.to_string_lossy().to_string()), Some(1), modify_type.clone());
            assert!(result.is_ok());

            let content = std::fs::read_to_string(&file_path).unwrap();
            assert!(content.len() > "original line".len());
        }
    }

    #[test]
    fn test_error_handling() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // Test modifying non-existent file
        let result = tool.modify(Some("non-existent.txt".to_string()), None, ModifyType::Append);
        assert!(result.is_err());

        // Test invalid line number
        tool.create(1, Some("test.txt".to_string()), Some("line 1".to_string())).unwrap();
        let src_path = tool.get_src_path().unwrap();
        let file_path = src_path.join("test.txt");

        let result = tool.modify(Some(file_path.to_string_lossy().to_string()), Some(10), ModifyType::Append);
        assert!(result.is_err());
    }

    #[test]
    fn test_placeholder_commands() {
        let (_temp_dir, mut tool) = setup_git_repo();

        // These should not error but are not fully implemented
        assert!(tool.merge().is_ok());
        assert!(tool.munge().is_ok());
        assert!(tool.rebase().is_ok());
    }

    #[test]
    fn test_repo_detection() {
        let temp_dir = TempDir::new().unwrap();

        // In a fresh temp dir, should not detect a repo
        let tool = RepoTool::new_in_directory("main".to_string(), false, temp_dir.path().to_path_buf());
        assert!(tool.is_ok()); // Should work even without git repo
    }
}
