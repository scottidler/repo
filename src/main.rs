use clap::{Parser, Subcommand};
use eyre::{Result, WrapErr};
use log::{debug, info, warn};
use rand::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "repo")]
#[command(about = "A Git workflow demonstration tool")]
#[command(long_about = "Test repo for git flows - creates repositories, branches, commits, and conflicts for demonstration purposes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Home branch name
    #[arg(long, global = true, default_value = "master")]
    home_branch: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new repository
    Init {
        /// Repository name
        #[arg(short, long)]
        repo_name: Option<String>,
    },
    /// Create a new branch
    Branch {
        /// Branch name (defaults to dev/<random-word>)
        #[arg(short, long)]
        branch_name: Option<String>,
        /// Reset to home branch
        #[arg(short = 'H', long)]
        home: bool,
        /// Create a commit after branching
        #[arg(short, long)]
        commit: bool,
    },
    /// Create random changes
    Change {
        /// Number of changes to create
        #[arg(short, long, default_value = "0")]
        count: u32,
    },
    /// Create a commit
    Commit {
        /// Commit name (defaults to random word)
        #[arg(short, long)]
        commit_name: Option<String>,
        /// Create a branch before committing
        #[arg(short, long)]
        branch: bool,
    },
    /// Create a merge conflict scenario
    Conflict {
        /// File path for the conflict
        #[arg(short, long)]
        filepath: Option<String>,
        /// Content for the file
        #[arg(long)]
        content: Option<String>,
    },
    /// Create a new file with random content
    Create {
        /// Number of files to create
        #[arg(short, long, default_value = "1")]
        count: u32,
        /// File path
        #[arg(short, long)]
        filepath: Option<String>,
        /// File content
        #[arg(long)]
        content: Option<String>,
    },
    /// Modify an existing file
    Modify {
        /// File to modify
        #[arg(short, long)]
        filepath: Option<String>,
        /// Line number to modify
        #[arg(short, long)]
        lineno: Option<usize>,
        /// Type of modification
        #[arg(short, long, value_enum, default_value = "random")]
        modify_type: ModifyType,
    },
    /// Perform a merge (placeholder)
    Merge,
    /// Perform a munge operation (placeholder)
    Munge,
    /// Perform a rebase (placeholder)  
    Rebase,
    /// Reset repository state
    Reset,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ModifyType {
    Append,
    Prepend,
    Prefix,
    Suffix,
    Random,
}

struct RepoTool {
    home_branch: String,
    verbose: bool,
    command_count: u32,
    words: Vec<String>,
}

impl RepoTool {
    fn new(home_branch: String, verbose: bool) -> Result<Self> {
        let words = Self::load_words()?;
        Ok(Self {
            home_branch,
            verbose,
            command_count: 0,
            words,
        })
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

    fn run_command(&mut self, cmd: &str, args: &[&str]) -> Result<Output> {
        self.command_count += 1;
        
        if self.verbose {
            println!("#{}) {} {}", self.command_count, cmd, args.join(" "));
        }
        
        let output = Command::new(cmd)
            .args(args)
            .output()
            .wrap_err_with(|| format!("Failed to execute: {} {}", cmd, args.join(" ")))?;
        
        if self.verbose {
            if !output.stdout.is_empty() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        Ok(output)
    }

    fn run_git(&mut self, args: &[&str]) -> Result<Output> {
        let mut git_args = vec!["--no-pager"];
        git_args.extend_from_slice(args);
        self.run_command("git", &git_args)
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
        } else {
            path.push("src");
        }
        
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

    fn get_repo_root(&mut self) -> Result<PathBuf> {
        let output = self.run_git(&["rev-parse", "--show-toplevel"])?;
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(root))
    }

    fn get_current_branch(&mut self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_src_path(&mut self) -> Result<PathBuf> {
        let repo_root = self.get_repo_root()?;
        Ok(repo_root.join("src"))
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
        let name = repo_name.unwrap_or_else(|| {
            format!("repo-{}", Uuid::new_v4().to_string()[..8].to_string())
        });

        info!("Initializing repository: {}", name);

        if Path::new(&name).exists() {
            fs::remove_dir_all(&name)
                .wrap_err_with(|| format!("Failed to remove existing directory: {}", name))?;
        }

        fs::create_dir_all(&name)
            .wrap_err_with(|| format!("Failed to create directory: {}", name))?;

        std::env::set_current_dir(&name)
            .wrap_err_with(|| format!("Failed to change to directory: {}", name))?;

        self.run_git(&["init"])?;

        println!("Initialized repository: {}", name);
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
                self.modify(None, None, ModifyType::Random)?;
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

    pub fn conflict(&mut self, filepath: Option<String>, content: Option<String>) -> Result<()> {
        info!("Creating conflict scenario");

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

        println!("Created conflict scenario between {} and {}", original_branch, conflict_branch);
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
            ModifyType::Random => {
                let types = [ModifyType::Append, ModifyType::Prepend, ModifyType::Prefix, ModifyType::Suffix];
                types.choose(&mut rand::rng()).unwrap().clone()
            }
            other => other,
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
            ModifyType::Random => unreachable!(),
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
    let mut tool = RepoTool::new(cli.home_branch, cli.verbose)?;

    match cli.command {
        Commands::Init { repo_name } => tool.init(repo_name),
        Commands::Branch { branch_name, home, commit } => tool.branch(branch_name, home, commit),
        Commands::Change { count } => tool.change(count),
        Commands::Commit { commit_name, branch } => tool.commit(commit_name, branch),
        Commands::Conflict { filepath, content } => tool.conflict(filepath, content),
        Commands::Create { count, filepath, content } => tool.create(count, filepath, content),
        Commands::Modify { filepath, lineno, modify_type } => tool.modify(filepath, lineno, modify_type),
        Commands::Merge => tool.merge(),
        Commands::Munge => tool.munge(),
        Commands::Rebase => tool.rebase(),
        Commands::Reset => tool.reset(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    // NOTE: These tests must run with --test-threads=1 because they change
    // the current working directory, which is process-global state.
    // Run with: cargo test -- --test-threads=1

    fn setup_git_repo() -> (TempDir, RepoTool) {
        let temp_dir = TempDir::new().unwrap();
        let _original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();
        
        let mut tool = RepoTool::new("main".to_string(), false).unwrap();
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
        assert!(path.starts_with("src"));
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
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();
        
        let mut tool = RepoTool::new("main".to_string(), false).unwrap();
        let result = tool.init(Some("test-repo".to_string()));
        
        assert!(result.is_ok());
        
        // The init command changes directory, so check in the current directory
        let current_dir = env::current_dir().unwrap();
        assert_eq!(current_dir.file_name().unwrap(), "test-repo");
        assert!(Path::new(".git").exists());
        
        // Restore directory
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_command() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo();
        
        let result = tool.create(1, Some("test-file.txt".to_string()), Some("test content".to_string()));
        assert!(result.is_ok());
        
        let src_path = tool.get_src_path().unwrap();
        let file_path = src_path.join("test-file.txt");
        assert!(file_path.exists());
        
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_multiple_files() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo();
        
        let result = tool.create(3, None, None);
        assert!(result.is_ok());
        
        let files = tool.find_files_in_src().unwrap();
        assert_eq!(files.len(), 3);
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_branch_creation() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();
        
        let result = tool.branch(Some("feature-branch".to_string()), false, false);
        assert!(result.is_ok());
        
        let current_branch = tool.get_current_branch().unwrap();
        assert_eq!(current_branch, "feature-branch");
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_branch_with_random_name() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();
        
        let result = tool.branch(None, false, false);
        assert!(result.is_ok());
        
        let current_branch = tool.get_current_branch().unwrap();
        assert!(current_branch.starts_with("dev/"));
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_commit_creation() {
        let original_dir = env::current_dir().unwrap();
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
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_modify_existing_file() {
        let original_dir = env::current_dir().unwrap();
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
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_change_command() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo();
        
        let result = tool.change(2);
        assert!(result.is_ok());
        
        let files = tool.find_files_in_src().unwrap();
        assert!(!files.is_empty());
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_conflict_creation() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo();
        
        let result = tool.conflict(Some("conflict-file.txt".to_string()), Some("initial content".to_string()));
        assert!(result.is_ok());
        
        // Verify branches exist
        let output = tool.run_git(&["branch", "-a"]).unwrap();
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(branches.contains("conflict-"));
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_reset_command() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo_with_commit();
        
        // Create and switch to a new branch
        tool.branch(Some("test-branch".to_string()), false, false).unwrap();
        
        // Reset should go back to main branch
        let result = tool.reset();
        assert!(result.is_ok());
        
        let current_branch = tool.get_current_branch().unwrap();
        assert_eq!(current_branch, "main");
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_git_status() {
        let original_dir = env::current_dir().unwrap();
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
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_file_finding() {
        let original_dir = env::current_dir().unwrap();
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
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_modify_types() {
        let original_dir = env::current_dir().unwrap();
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
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_error_handling() {
        let original_dir = env::current_dir().unwrap();
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
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_command_counting() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo();
        
        let initial_count = tool.command_count;
        
        tool.run_git(&["status"]).unwrap();
        assert_eq!(tool.command_count, initial_count + 1);
        
        tool.run_git(&["log", "--oneline"]).unwrap();
        assert_eq!(tool.command_count, initial_count + 2);
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_placeholder_commands() {
        let original_dir = env::current_dir().unwrap();
        let (_temp_dir, mut tool) = setup_git_repo();
        
        // These should not error but are not fully implemented
        assert!(tool.merge().is_ok());
        assert!(tool.munge().is_ok());
        assert!(tool.rebase().is_ok());
        
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_repo_detection() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();
        
        // In a fresh temp dir, should not detect a repo
        let tool = RepoTool::new("main".to_string(), false);
        assert!(tool.is_ok()); // Should work even without git repo
        
        env::set_current_dir(original_dir).unwrap();
    }
}
