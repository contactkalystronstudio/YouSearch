# ⚡ YouSearch

**Fast, fuzzy, content-aware file search. Single binary. Terminal + Web UI.**

[![CI](https://img.shields.io/github/actions/workflow/status/contactkalystronstudio/yousearch/ci.yml?label=build&style=flat-square)](https://github.com/contactkalystronstudio/yousearch/actions)
[![Release](https://img.shields.io/github/v/release/contactkalystronstudio/yousearch?style=flat-square&color=7f6cf7)](https://github.com/contactkalystronstudio/yousearch/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-4df0a0?style=flat-square)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024%20edition-f4c842?style=flat-square)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue?style=flat-square)](#install)

---

```bash
yousearch auth !test --limit=5
yousearch config #backend
yousearch search query --json
yousearch serve
yousearch index --watch
```

---

## Why YouSearch?

| Feature | YouSearch | find / dir | ripgrep | Everything |
| :--- | :---: | :---: | :---: | :---: |
| Fuzzy filename search | ✓ | — | — | — |
| Content search | ✓ | — | ✓ | — |
| Synonym expansion | ✓ | — | — | — |
| Live index (watch mode) | ✓ | — | — | ✓ |
| Web UI | ✓ | — | — | Windows Only |
| Tagging system | ✓ | — | — | — |
| Single binary, no install | ✓ | built-in | ✓ | — |
| Linux + Windows | ✓ | built-in | ✓ | Windows Only |

---

## Features

### **Search Engine**
* **Hybrid Matching:** Search filenames and file content simultaneously in a single query.
* **Fuzzy Logic:** Intelligent typo handling using Levenshtein distance (`conifg` → `config`, `mn` → `main`).
* **Subsequence Matching:** Powerful matching like `mdr` finding `middleware`.
* **Synonym Expansion:** Searching `auth` automatically finds `authentication`, `login`, `token`, and `jwt`.
* **JSON Support:** Use the `--json` flag for machine-readable output, perfect for piping into other tools or scripts.
* **Advanced Filters:** Use `!` for exclusion (e.g., `!test`) and `#` for tags (e.g., `#backend`).

### **Indexing**
* **Live Watch:** `yousearch index --watch` monitors your file system and auto-updates every 2 seconds.
* **Incremental Mode:** Use `yousearch index -i` to scan only modified files for near-instant updates.
* **Safety & Performance:** Automatically skips `.git`, `node_modules`, binary files, and large files (>512KB). **Now with Symlink Loop protection.**

### **User Interface**
* **Modern CLI:** Beautifully colored terminal output with highlighted matches and line previews.
* **Web Dashboard:** A sleek search-as-you-type interface at `http://localhost:7878`.

---

## Install

### Windows — pre-built binary
1. Download `yousearch.exe` and `indexer.dll` from [Releases](https://github.com/contactkalystronstudio/yousearch/releases).
2. Run once to set up PATH automatically:
```bash
yousearch setup
```

### Linux — pre-built binary
```bash
curl -fsSL [https://github.com/contactkalystronstudio/yousearch/releases/latest/download/yousearch-linux-x86_64.tar.gz](https://github.com/contactkalystronstudio/yousearch/releases/latest/download/yousearch-linux-x86_64.tar.gz) | tar xz
sudo mv yousearch /usr/local/bin/
yousearch setup
```

### Build from source
Requires Rust 1.80+
```bash
git clone [https://github.com/contactkalystronstudio/yousearch](https://github.com/contactkalystronstudio/yousearch)
cd yousearch/rust-engine
cargo build --release
```

---

## Commands

| Command | Description |
| :--- | :--- |
| `index` | Full index rebuild |
| `index -i` | Incremental update (changed files only) |
| `index --watch` | Live indexing mode |
| `search ` | Main search (Aliases: `find`, `f`, `s`) |
| `search --json` | Return search results as JSON |
| `add ` | Add directory to indexed roots |
| `tag  ` | Label a file with a tag |
| `serve` | Start Web UI on port 7878 |
| `status` | View index statistics and counts |
| `setup` | Interactive PATH and environment setup |

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

## Project Layout

```text
yousearch/
├── rust-engine/     # Core logic and CLI
│   ├── src/main.rs
│   └── Cargo.toml
└── cpp-indexer/     # Native FFI indexer
    ├── indexer.cpp
    ├── build.bat    # Windows build script
    └── build.sh     # Linux build script
```

---

## Roadmap
* [x] `--json` output flag
* [x] Symlink loop protection
* [ ] Git-aware ranking (Prioritize files in `.git` history)
* [ ] VSCode Extension
* [ ] Windows installer (.msi)
* [ ] Background service / System Tray

---

## License
MIT — see [LICENSE](LICENSE).
