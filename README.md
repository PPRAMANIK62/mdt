# mdt

A fast, terminal-based markdown viewer and editor built with Rust.

Point `mdt` at a directory and you get a file tree, a fully rendered markdown preview, and a built-in editor with vim-style keybindings. It renders headings, code blocks with syntax highlighting, tables, task lists, blockquotes, and more, all inside your terminal.

## Features

**File browser**

- Collapsible file tree with directory navigation
- File search/filter that narrows the tree as you type
- Toggle the file tree on or off with a leader key

**Markdown preview**

- Headings (H1 through H6) with distinct color and weight
- Bold, italic, and strikethrough text
- Inline code with background highlighting
- Fenced code blocks with syntax highlighting (powered by syntect)
- Code blocks rendered with box-drawing borders
- Ordered and unordered lists with proper nesting
- Hanging indents on wrapped list continuation lines
- Task lists with checkbox rendering (unchecked/checked)
- Blockquotes with vertical bar indicators, including nested blockquotes
- Tables with box-drawing borders and bold headers
- Horizontal rules
- Links with a searchable link picker overlay
- Autolinks

**Editor**

- Built-in text editor with vim-style keybindings
- Insert and Normal modes
- Dirty-file tracking with unsaved-changes warnings
- Save, save-and-quit, and force-quit commands

**Search**

- In-document search with match count and navigation
- File tree search that filters entries in real time
- Wrapping match navigation (n/N)

**Terminal integration**

- Width-aware text wrapping for paragraphs, headings, blockquotes, and lists
- Code block and table truncation for narrow terminals
- Terminal background color detection (prevents transparency bleed)
- `NO_COLOR` environment variable support
- Dirty-flag rendering (only redraws when something changes)
- Pre-warmed syntax highlighting on a background thread
- Welcome screen with ASCII art logo
- Help overlay and link picker modal
- Panic-safe terminal teardown

## Installation

Install from source with Cargo:

```
cargo install --path .
```

Or build manually:

```
cargo build --release
```

The binary will be at `target/release/mdt`.

## Usage

```
mdt [path]
```

Opens the given directory (or file). Defaults to the current directory if no path is provided.

When `mdt` starts, you'll see the welcome screen. Press `Space+e` to open the file tree, navigate to a markdown file, and press `Enter` to preview it.

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down / up (preview), navigate items (file tree) |
| `gg` | Jump to top |
| `G` | Jump to bottom |
| `Ctrl+d` | Half page down |
| `Ctrl+u` | Half page up |
| `Tab` | Switch focus between file tree and preview |

### File Tree

| Key | Action |
|-----|--------|
| `Enter` | Open file / toggle directory |
| `h` / `Left` | Collapse directory |
| `l` / `Right` | Expand directory |
| `Space+e` | Toggle file tree visibility |

### Editor

| Key | Action |
|-----|--------|
| `i` / `e` | Enter edit mode (from preview) |
| `Esc` | Exit insert mode / exit editor |
| `:w` | Save |
| `:wq` / `:x` | Save and exit editor |
| `:q` | Quit editor (warns on unsaved changes) |
| `:q!` | Force quit editor (discards changes) |

### Search

| Key | Action |
|-----|--------|
| `/` | Start search |
| `Enter` | Confirm search |
| `n` | Next match |
| `N` | Previous match |
| `Esc` | Cancel search |

### Other

| Key | Action |
|-----|--------|
| `o` | Open link picker (in preview) |
| `?` | Toggle help overlay |
| `q` | Quit |
| `Ctrl+c` | Force quit from any mode |

## Built With

- [ratatui](https://github.com/ratatui/ratatui) -- terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) -- cross-platform terminal manipulation
- [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) -- CommonMark parser (with SIMD)
- [syntect](https://github.com/trishume/syntect) -- syntax highlighting
- [tui-tree-widget](https://github.com/EdJoPaTo/tui-rs-tree-widget) -- tree view widget for ratatui
- [ratatui-textarea](https://github.com/rhysd/tui-textarea) -- text editor widget with vim keybindings
- [terminal-colorsaurus](https://github.com/bash/terminal-colorsaurus) -- terminal background color detection

## License

MIT. See [LICENSE](LICENSE).
