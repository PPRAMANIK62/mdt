# mdt

A terminal-based markdown viewer and editor built with Rust.

## About

mdt is a TUI application for browsing, previewing, and editing markdown files. Point it at a directory and you get a file tree, a rendered markdown preview with syntax-highlighted code blocks, and a built-in editor. Built on [Ratatui](https://github.com/ratatui/ratatui).

## Features

- File tree browser with directory navigation
- Rendered markdown preview (headings, bold/italic, code blocks with syntax highlighting, lists, links, blockquotes)
- Built-in editor with vim-like keybindings
- In-document and file search
- Toggleable file tree panel
- Help overlay

## Keybindings

| Key | Action |
|---|---|
| `j` / `k` | Navigate / Scroll |
| `Enter` | Open file / Toggle directory |
| `Tab` | Switch focus (file tree / preview) |
| `Space+e` | Toggle file tree |
| `i` / `e` | Enter edit mode |
| `/` | Search |
| `n` / `N` | Next / Previous match |
| `gg` / `G` | Jump to top / bottom |
| `Ctrl+d` / `Ctrl+u` | Half page down / up |
| `:w` | Save |
| `:wq` | Save and exit editor |
| `:q` | Quit editor (or app) |
| `?` | Toggle help overlay |
| `Esc` | Close editor / Clear search |

## Installation

```
cargo install --path .
```

Or build from source:

```
cargo build --release
```

## Usage

```
mdt [path]
```

Defaults to the current directory if no path is given.

## License

MIT. See [LICENSE](LICENSE).
