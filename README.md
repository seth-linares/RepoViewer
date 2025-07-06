# RepoViewer

A TUI file explorer for quickly gathering repo metadata and file content.

<p align="center">
    <img src="media/RepoViewer.gif" />
</p>

## What's New in v2.0.0 🐈

RepoViewer now updates itself! No more manually downloading new versions -- just run `RepoViewer --update` and you're good to go. I also fixed that annoying clipboard bug on Linux where your copied content would vanish into the void. Check out the [CHANGELOG](CHANGELOG.md) for all the details.

## Installation

### The Easy Way

```bash
cargo install --git https://github.com/seth-linares/RepoViewer
```

### Keeping Up to Date

Once you have RepoViewer 2.0.0 or later, updating is super simple:

```bash
# Check if there's a new version
RepoViewer --check-update

# Update to the latest version
RepoViewer --update
```

If you had 1.0.0 you need to force cargo to re-install so you can have a self-updating version:

```bash
cargo install --git https://github.com/seth-linares/RepoViewer --force
```

The updater is smart enough to grab the right binary for your platform, and it'll show you a nice progress bar while downloading. If something goes wrong (like permission issues), it'll tell you exactly what to do.

### Building from Source

If you want to clone and build it yourself:

```bash
git clone https://github.com/seth-linares/RepoViewer
cd RepoViewer
cargo build --release
```

### Building without clipboard support

If you're in an environment without clipboard access (like certain servers or containers), or just want a smaller binary, you can build without the clipboard feature:

```bash
cargo build --release --no-default-features
```

The only difference is that `c` and `C` commands will show an error message explaining that clipboard support wasn't compiled in -- up to you!



## How to use it

### Basic Navigation

Start RepoViewer in any directory:

```bash
RepoViewer
```

Or specify a path:

```bash
RepoViewer ./my-project
```

Once you're in, navigation is pretty simple (you may need to use shift to access some keys):

- Arrow keys move around (up/down to select, left to go back, right/enter to open directories)
- `h` toggles hidden files
- `g` toggles gitignored files (when you're in a git repo)
- `?` shows help
- `q` or ESC exits



### Collecting Files

This is where it gets interesting. As you browse, you can build a collection of files to export:

- `a` adds the current file to your collection
- `A` adds all files in the current directory 
- `d` removes the current file from collection
- `D` clears the entire collection
- `r` refreshes your collection (updates modified files, removes deleted ones)

Files in your collection show up with a `[+]` marker, and the header keeps track of the total size. I added warnings when your collection gets large (yellow at 25MB, red at 50MB) because nobody wants to accidentally paste something thats so huge it crashes/freezes their computer.

### Exporting

Once you've collected what you need:

- `S` saves everything to a markdown file in your current directory
- `C` copies the collection to your clipboard (requires the clipboard feature)
- `t` saves just the directory tree to a file
- `c` copies just the tree to clipboard

The markdown output includes the file paths as headers and properly formatted code blocks with syntax highlighting.

### Quick Shortcuts

I added some navigation shortcuts that I find myself using constantly:

- `~` jumps back to where you started RepoViewer
- `G` jumps to the git repository root (if you're in one)
- PageUp/PageDown jump to the first/last item in the current directory

### Command Line Options

#### Tree Generation

If you just need a quick tree without the TUI:

```bash
RepoViewer --tree
RepoViewer --tree --depth 3 --hidden
```

#### Version and Updates

```bash
# Check your current version
RepoViewer --version

# See if there's a newer version available
RepoViewer --check-update

# Update to the latest version
RepoViewer --update

# Update without confirmation prompt
RepoViewer --update --yes
```

## Why I made this

I'm not aware of any good cross-platform tools that let you quickly browse repos, see their structure at a glance, and selectively copy file contents for export. Maybe that means **RepoViewer** is pretty niche and I'm the only one who needs it, but I genuinely love what I've created. It was a fun creative challenge figuring out the design and functionality, and it really does improve how I work. I hope others will see the value in **RepoViewer** too, but I'm fine being its biggest fan for now 🐈‍⬛.


If you're curious about other tools I've tried and why they weren't good enough for me here's a list:

1. **`tree` command** - Decent for structure (I still prefer mine), but doesn't help with file contents
2. **Manual copying** - You lose track of what you've shared and I sometimes accidentally skip files and usually waste time formatting
3. **IDE features** - IDE's have some useful features and it's what I relied on, but they don't have a way to get the tree I still have to manually copy file contents


Aside from the tree/file structure feature, the file collection feature is something I think is absolutely key and makes **RepoViewer** worth it, because you rarely need to share everything. You navigate around, cherry-pick the relevant files, see them marked with `[+]` so you know what you've grabbed, and export them all at once with proper relative paths. The refresh feature means you can build your collection, make changes, then refresh to update the collection with your modifications before sharing again.

## Technical Choices

I built this in Rust because I wanted it to be fast and work everywhere without dependencies. The TUI is built with Ratatui, which provides a nice cross-platform terminal interface. I was going to use C++ but cross-compilation is a joke to be honest. Then I was going to use Python and I realized it is a pain to work with executables and dependencies. Rust was unironically the best choice for my app and while yes you can physically use another language, it's not even close to as convenient or useful as Rust is here.

For file type detection, I maintain a whitelist of known text file extensions rather than trying to detect binary files heuristically. This prevents you from accidentally including binary files in your collection.

The tool respects `.gitignore` files when you're in a git repository, but you can toggle that behavior if needed. This keeps all the `node_modules`, `target`, and other build directories from cluttering up what you're trying to share.

### Self-Updating

The self-update feature (new in v2.0.0) uses GitHub releases to check for and download updates. It's built to be reliable -- it detects your platform automatically, verifies the download, and even keeps a backup of your current version just in case. If you're curious about the implementation, check out `update.rs`. The whole thing uses rustls instead of OpenSSL, so it works everywhere without extra dependencies.

## Contributing

Feel free to open issues or PRs! The codebase is organized into modules:
- `app/` - Core application state and logic
- `ui.rs` - Terminal UI rendering  
- `utils.rs` - File type detection and helpers
- `main.rs` - CLI parsing and event loop
- `update.rs` - Self-update functionality (new in v2.0.0)
- `clipboard.rs` - Platform-specific clipboard handling

The code has pretty extensive comments explaining the design decisions, especially in the collection module where most of the interesting logic lives.

For people who want to modify the color scheme, you need to go into `utils.rs` and `ui.rs`. It's very easy to modify, you can look up "ratatui colors" on google and find some documentation if you need help, but it should be self-explanatory.
