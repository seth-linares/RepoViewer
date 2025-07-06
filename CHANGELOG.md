# Changelog

All notable changes to RepoViewer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Performance optimizations (coming soon)

## [2.0.0] - 2025-07-06

### Added
- **Self-updating functionality** - Update RepoViewer directly from the command line
  - `--check-update` flag to check for new versions
  - `--update` flag to download and install updates
  - `--yes` flag to skip update confirmation
  - Automatic platform detection for correct binary selection
- **GitHub Actions CI/CD pipeline** for automated releases
  - Cross-compilation for multiple platforms (Windows, macOS Intel/ARM, Linux x64/ARM64)
  - Automated binary uploads to GitHub releases
  - SHA256 checksum generation for security verification
- **Proper clipboard lifetime management** 
  - Fixed clipboard contents being lost on Linux/X11/Wayland
  - Intelligent platform detection for clipboard behavior
  - Background thread handling for X11 clipboard persistence
- **Version command** - `--version/-V` flag shows detailed version information
- **Enhanced error handling** with user-friendly messages
  - Network error detection and helpful suggestions
  - Permission error guidance
  - Platform-specific error messages

### Changed
- **Major version bump** to 2.0.0 to reflect the significant new features and improvements
- Improved error messages throughout the application
- Better platform detection for clipboard operations
- Cargo.toml optimizations for release builds
  - Link Time Optimization (LTO) enabled
  - Single codegen unit for better optimization
  - Symbol stripping for smaller binaries

### Fixed
- **Critical clipboard bug** where contents would be lost immediately after copying on Linux
  - Clipboard now properly persists after RepoViewer exits
  - Works correctly on X11, Wayland, and WSLg environments
- Clap version flag conflict (disabled auto-generated version flag)

### Technical Details
- Migrated from OpenSSL to rustls for better portability
- Added comprehensive error types for update operations
- Implemented proper mutex handling for thread safety

## [1.0.0] - 2025-06-11

### Added
- Initial release of RepoViewer
- **TUI file explorer** with intuitive navigation
  - Arrow keys for navigation
  - Tree structure visualization
  - Directory traversal with breadcrumbs
- **File collection system**
  - Add individual files or entire directories
  - Track collection size with warnings at 25MB and 50MB
  - Refresh collected files to sync with filesystem changes
- **Export functionality**
  - Save collections to markdown files with syntax highlighting
  - Copy collections or directory trees to clipboard
  - Generate clean, shareable code contexts for LLMs
- **Git integration**
  - Automatic detection of git repositories
  - Toggle visibility of gitignored files
  - Quick navigation to git root
- **Smart filtering**
  - Toggle hidden files visibility
  - Automatic binary file detection
  - Text file type recognition for safe collection
- **Cross-platform support**
  - Works on Windows, macOS, and Linux
  - Platform-specific hidden file detection
- **Command-line tree generation**
  - `--tree` flag for quick directory structure output
  - `--depth` option to limit tree depth
  - `--hidden` and `--all` flags for comprehensive views

### Known Issues in 1.0.0
- Clipboard contents lost immediately on Linux (fixed in 2.0.0)
- No automated build pipeline (fixed in 2.0.0)
- No self-update capability (fixed in 2.0.0)

## Upgrade Guide

### From 1.0.0 to 2.0.0

1. **If you installed via cargo:**
```bash
cargo install --git https://github.com/seth-linares/RepoViewer --force
```

2. **If you have a binary installation:**
   - Download the latest binary from the [releases page](https://github.com/seth-linares/RepoViewer/releases)
   - Or use the new self-update feature once you have 2.0.0:
```bash
RepoViewer --update
```

3. **New features to try:**
   - Check your version: `RepoViewer --version`
   - Check for updates: `RepoViewer --check-update`
   - Update to latest: `RepoViewer --update`

## Future Plans

- Performance optimizations for large directories
- Configuration file support
- Custom themes
- Plugin system for extending functionality
- Integration with more version control systems

---

For more information, visit the [RepoViewer GitHub repository](https://github.com/seth-linares/RepoViewer).