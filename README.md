# repo

A Git workflow demonstration tool written in Rust

## Overview

This is a comprehensive rewrite of the original Python git workflow tool, now implemented in Rust using modern best practices. The tool creates repositories, branches, commits, and conflicts for demonstration and testing purposes.

## Features

- **Initialize repositories** - Create new git repositories with random names
- **Create files** - Generate random files with realistic content
- **Branch management** - Create branches with random or specified names
- **Commit creation** - Make commits with generated or custom messages
- **Conflict scenarios** - Create merge conflict situations for testing
- **File modification** - Modify existing files in various ways (append, prepend, prefix, suffix)
- **Repository operations** - Reset, clean, and manage repository state

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Initialize a new repository
./target/release/repo init --repo-name my-demo

# Create random files
./target/release/repo create --count 5

# Make a commit
./target/release/repo commit --commit-name "initial-setup"

# Create a feature branch
./target/release/repo branch --branch-name "feature-demo"

# Modify existing files
./target/release/repo modify --modify-type append

# Create a merge conflict scenario
./target/release/repo conflict --filepath "test.txt" --content "base content"

# Reset to main branch
./target/release/repo reset
```

## Commands

- `init` - Initialize a new repository
- `branch` - Create a new branch
- `change` - Create random changes (files and modifications)
- `commit` - Create a commit
- `conflict` - Create a merge conflict scenario
- `create` - Create new files with random content
- `modify` - Modify existing files
- `reset` - Reset repository state to main branch

## Options

- `-v, --verbose` - Enable verbose output
- `--home-branch <BRANCH>` - Set the home branch name (default: master)

## Architecture

The Rust implementation provides several improvements over the original Python version:

### Security Improvements
- **No shell injection vulnerabilities** - Uses proper subprocess argument arrays
- **Safe file operations** - Proper error handling and path validation
- **Input validation** - All user inputs are validated before use

### Performance Improvements
- **Faster execution** - Compiled binary vs interpreted Python
- **Better resource management** - Rust's ownership model prevents memory leaks
- **Concurrent operations** - Built-in support for parallel operations

### Code Quality
- **Type safety** - Rust's type system prevents many runtime errors
- **Error handling** - Comprehensive error handling with `eyre` crate
- **Modern CLI** - Uses `clap` derive for robust argument parsing
- **Comprehensive tests** - Full test suite proving correctness

### Git Integration
- **No pager issues** - All git commands use `--no-pager` flag
- **Proper branch detection** - Smart detection of default branches (main/master)
- **Clean command execution** - Proper stdout/stderr handling

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

The tests cover:
- Repository initialization
- File creation and modification
- Branch operations
- Commit creation
- Conflict scenarios
- Error handling
- Command counting
- Git integration

## Example Session

```bash
# Create and enter a new repository
./target/release/repo init --repo-name demo
cd demo

# Create some files
./target/release/repo create --count 3

# Make initial commit
./target/release/repo commit --commit-name "initial"

# Create feature branch
./target/release/repo branch --branch-name "feature"

# Modify files
./target/release/repo modify --modify-type append

# Create conflict scenario
./target/release/repo conflict

# View branches
git branch -a

# View commit history
git log --oneline
```

## Dependencies

- `clap` - Command line argument parsing
- `eyre` - Error handling
- `log` + `env_logger` - Logging
- `rand` - Random generation
- `uuid` - Unique identifiers
- `tempfile` - Temporary directories (tests)

## License

Mozilla Public License 2.0
