# mdt

A fast, terminal-based markdown viewer and editor built with Rust.

**[Documentation](https://mdt.purbayan.me/)**

Point `mdt` at a directory and you get a file tree, a fully rendered markdown preview, a built-in editor with vim-style keybindings, and a live split-pane preview that updates as you type. It renders headings, code blocks with syntax highlighting, tables, task lists, blockquotes, and more, all inside your terminal.

![Demo](public/demo.gif)

## Features

**File browser**

- Collapsible file tree with directory navigation
- File search/filter that narrows the tree as you type
- Toggle the file tree on or off with a leader key
- File management: create, delete, rename, and move files and directories
- Nested path creation (e.g. `abc/def/notes.md`)
- Fuzzy file finder to quickly jump to any file
- File watching with automatic reload when files change on disk

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
- Scrollbar that appears when content exceeds the viewport
- Heading jump navigation (`[` / `]`)

**Editor**

- Built-in text editor with vim-style keybindings
- Insert and Normal modes
- Dirty-file tracking with unsaved-changes warnings
- Save, save-and-quit, and force-quit commands
- Reload file from disk with `:e` / `:edit` command

**Live preview**

- Real-time split-pane preview that updates as you type
- Horizontal (editor left, preview right) and vertical (editor top, preview bottom) split modes
- Debounced rendering to keep editing responsive
- Parallel scrolling between editor and preview panes
- Toggle with `Space+p` or the `:preview` command
- Swap split orientation with `Space+s`

**Search**

- In-document search with match count and navigation
- File tree search that filters entries in real time
- Wrapping match navigation (n/N)

**Terminal integration**

- Mouse support: scroll wheel to scroll, click to switch panes
- Width-aware text wrapping for paragraphs, headings, blockquotes, and lists
- Code block and table truncation for narrow terminals
- Terminal background color detection (prevents transparency bleed)
- `NO_COLOR` environment variable support
- Dirty-flag rendering (only redraws when something changes)
- Pre-warmed syntax highlighting on a background thread
- Advisory file locking to prevent concurrent instance conflicts
- Welcome screen with ASCII art logo
- Help overlay and link picker modal
- Panic-safe terminal teardown

## Installation

Install from [crates.io](https://crates.io/crates/mdtui):

```
cargo install mdtui
```

Or build from source:

```
cargo build --release
```

The binary will be at `target/release/mdt`.

## Usage

```
mdt [path] [--max-file-size <bytes>]
```

Opens the given directory (or file). Defaults to the current directory if no path is provided.

| Flag | Description |
|------|-------------|
| `--max-file-size <bytes>` | Maximum file size to open (default: 5 MB) |

When `mdt` starts, you'll see the welcome screen. Press `Space+e` to open the file tree, navigate to a markdown file, and press `Enter` to preview it.

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` / `Down` / `Up` | Scroll down / up (preview), navigate items (file tree) |
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
| `a` | Create new file |
| `A` | Create new directory |
| `d` | Delete file or directory |
| `r` | Rename file or directory |
| `m` | Move file or directory |
| `Backspace` | Collapse directory |
| `ff` | Fuzzy file finder |

### Preview

| Key | Action |
|-----|--------|
| `[` | Jump to previous heading |
| `]` | Jump to next heading |

### Editor

| Key | Action |
|-----|--------|
| `i` / `e` | Enter edit mode (from preview) |
| `Esc` | Exit insert mode / exit editor |
| `:w` | Save |
| `:wq` / `:x` | Save and exit editor |
| `:q` | Quit editor (warns on unsaved changes) |
| `:q!` | Force quit editor (discards changes) |
| `:e` / `:edit` | Reload file from disk |

### Live Preview

| Key | Action |
|-----|--------|
| `Space+p` | Toggle live preview split |
| `Space+s` | Swap split orientation (horizontal / vertical) |
| `:preview` | Toggle live preview (command mode) |

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
- [clap](https://github.com/clap-rs/clap) -- command-line argument parsing
- [fs2](https://github.com/danburkert/fs2-rs) -- advisory file locking
- [fuzzy-matcher](https://github.com/lotabout/fuzzy-matcher) -- fuzzy string matching for file finder
- [notify](https://github.com/notify-rs/notify) -- cross-platform file system notifications

## License

MIT. See [LICENSE](LICENSE).
