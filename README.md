# YouSearch
Blazing-fast local search engine (Rust + C++) with content search, fuzzy matching, and live indexing.

# ⚡ YouSearch

**Fast, fuzzy, content-aware file search. Single binary. Terminal + Web UI.**

[![CI](https://img.shields.io/github/actions/workflow/status/contactkalystronstudio/yousearch/ci.yml?label=build&style=flat-square)](https://github.com/YOUR_USERNAME/yousearch/actions)
[![Release](https://img.shields.io/github/v/release/contactkalystronstudio/yousearch?style=flat-square&color=7f6cf7)](https://github.com/contactkalystronstudio/yousearch/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-4df0a0?style=flat-square)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024%20edition-f4c842?style=flat-square)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue?style=flat-square)](#install)

---

```
yousearch auth !test --limit=5
yousearch config #backend
yousearch serve
yousearch index --watch
```

---

## Why YouSearch?

| | YouSearch | find / dir | ripgrep | Everything |
|---|:---:|:---:|:---:|:---:|
| Fuzzy filename search | ✓ | — | — | — |
| Content search | ✓ | — | ✓ | — |
| Synonym expansion | ✓ | — | — | — |
| Live index (watch mode) | ✓ | — | — | ✓ |
| Web UI | ✓ | — | — | Windows only |
| Multi-folder index | ✓ | — | — | ✓ |
| Tagging system | ✓ | — | — | — |
| Single binary, no install | ✓ | built-in | ✓ | — |
| Linux + Windows | ✓ | built-in | ✓ | Windows only |

---

## Features

**Search**

- Filename and content search in one query
- Fuzzy matching with Levenshtein distance — `conifg` → `config`, `mn` → `main`
- Subsequence matching — `mdr` → `middleware`
- Synonym expansion — `auth` finds `authentication`, `login`, `token`, `jwt`
- Exclude operator — `!test` removes test files
- Tag filter — `#backend` narrows to tagged files only
- Result limit — `--limit=N`
- Inline content preview with line numbers in both CLI and Web UI

**Indexing**

- Full rebuild — `yousearch index`
- Incremental, changed files only — `yousearch index -i`
- Live watch, auto-updates every 2s — `yousearch index --watch`
- Multi-folder — `yousearch add <path>`
- Skips `.git`, `node_modules`, `target`, and more automatically
- Skips binary files and files over 512 KB automatically

**UI**

- Colored terminal output with highlighted matches
- Web UI at `http://localhost:7878` — search as you type, click to copy path
- Config file `.yousearchrc` (TOML) — local file overrides `~/.yousearchrc`

---

## Install

### Windows — pre-built binary

1. Download `yousearch.exe` from [Releases](https://github.com/YOUR_USERNAME/yousearch/releases)
2. Run it once to set up PATH:

```
yousearch setup
```

### Linux — pre-built binary

```bash
curl -fsSL https://github.com/YOUR_USERNAME/yousearch/releases/latest/download/yousearch-linux-x86_64.tar.gz | tar xz
sudo mv yousearch /usr/local/bin/
yousearch setup
```

### Build from source

Requires Rust 1.80+

```bash
git clone https://github.com/YOUR_USERNAME/yousearch
cd yousearch/rust-engine
cargo build --release
```

Binary lands at `target/release/yousearch` (or `.exe` on Windows).

---

## Quick Start

```bash
yousearch index              # index the current directory
yousearch main               # search by filename and content
yousearch auth !test         # auth files, exclude test files
yousearch config #backend    # search "config", tagged #backend only
yousearch index --watch      # live indexing
yousearch serve              # open web UI in browser
```

---

## Commands

| Command | Description |
|---|---|
| `index` | Full index rebuild |
| `index -i` | Incremental update — changed files only |
| `index --watch` | Live indexing — watches for file changes |
| `search <query>` | Search (aliases: `find` `f` `s`) |
| `add <path>` | Add a directory to indexed roots |
| `tag <file> <label>` | Tag a file |
| `untag <file> <label>` | Remove a tag |
| `tags` | List all tags |
| `serve` | Start web UI |
| `status` | Index statistics |
| `history` | Recent queries |
| `config` | Show active config |
| `setup` | Interactive first-run setup |

---

## Query Syntax

| Syntax | Meaning |
|---|---|
| `main` | Fuzzy search by name and content |
| `auth login` | Both terms must match |
| `!test` | Exclude files containing "test" |
| `#backend` | Only files tagged with #backend |
| `--limit=5` | Return at most 5 results |

---

## Config — `.yousearchrc`

Place in your project root or `~/.yousearchrc`:

```toml
roots            = [".", "C:/projects/myapp"]
excludes         = [".git", "node_modules", "target", "dist"]
extensions       = [".rs", ".cpp", ".py", ".ts", ".md", ".json"]
default_limit    = 20
fuzzy_threshold  = 2
max_content_kb   = 512
web_port         = 7878
```

---

## Index Format

Binary file `index.yswe`:

```
YSWE            magic (4 bytes)
version         u8 = 2
count           u32 little-endian
entries:
  path_len      u32
  path          UTF-8, forward slashes
  mtime         u64 Unix epoch seconds
  has_content   u8  (0 or 1)
  [content_len  u32]
  [content      UTF-8, max 512 KB]
```

---

## Project Layout

```
yousearch/
├── rust-engine/
│   ├── src/main.rs
│   ├── Cargo.toml
│   └── build.rs
└── cpp-indexer/
    ├── indexer.cpp
    ├── indexer.h
    ├── build.bat         Windows
    └── build.sh          Linux
```

The Rust binary has its own built-in indexer. The C++ shared library is only needed for external FFI use.

**Windows:**
```bat
cd cpp-indexer
build.bat
```

**Linux:**
```bash
cd cpp-indexer
chmod +x build.sh && ./build.sh
```

---

## Contributing

1. Fork and clone
2. `git checkout -b feature/my-thing`
3. `cargo clippy && cargo test`
4. Open a pull request

---

## License

MIT — see [LICENSE](LICENSE).

---

## Roadmap

- [ ] Windows installer (.msi)
- [ ] Background service / system tray
- [ ] VSCode extension
- [ ] Git-aware ranking
- [ ] `--json` output flag
- [ ] Plugin API
