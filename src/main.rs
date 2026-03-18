use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, UNIX_EPOCH};

use colored::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_roots")]
    pub roots: Vec<String>,
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default = "default_excludes")]
    pub excludes: Vec<String>,
    #[serde(default = "default_limit")]
    pub default_limit: usize,
    #[serde(default = "default_fuzzy")]
    pub fuzzy_threshold: usize,
    #[serde(default = "default_content_kb")]
    pub max_content_kb: usize,
    #[serde(default = "default_port")]
    pub web_port: u16,
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub history: Vec<String>,
}

fn default_roots()      -> Vec<String> { vec![".".into()] }
fn default_limit()      -> usize       { 20 }
fn default_fuzzy()      -> usize       { 2 }
fn default_content_kb() -> usize       { 512 }
fn default_port()       -> u16         { 7878 }

fn default_extensions() -> Vec<String> {
    [".rs",".cpp",".c",".h",".hpp",".txt",".md",
     ".json",".toml",".yaml",".yml",".py",".js",
     ".ts",".html",".css",".xml",".sh",".bat",".cmake"]
    .iter().map(|s| s.to_string()).collect()
}

fn default_excludes() -> Vec<String> {
    [".git","node_modules","target",".vs","__pycache__",
     "dist","build",".cache","vendor",".idea","out"]
    .iter().map(|s| s.to_string()).collect()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            roots:         default_roots(),
            extensions:    default_extensions(),
            excludes:      default_excludes(),
            default_limit: default_limit(),
            fuzzy_threshold: default_fuzzy(),
            max_content_kb: default_content_kb(),
            web_port:      default_port(),
            tags:          HashMap::new(),
            history:       Vec::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let local = Path::new(".yousearchrc");
    if local.exists() { return local.to_path_buf(); }
    if let Some(home) = home_dir() { return home.join(".yousearchrc"); }
    local.to_path_buf()
}

pub fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str(&text) { return cfg; }
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) {
    if let Ok(text) = toml::to_string_pretty(cfg) {
        let _ = fs::write(config_path(), text);
    }
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub path:    String,
    pub mtime:   u64,
    pub content: Option<String>,
    pub tags:    Vec<String>,
}

pub fn index_path() -> PathBuf {
    let local = Path::new("index.yswe");
    if local.exists() { return local.to_path_buf(); }
    if let Some(home) = home_dir() {
        let h = home.join(".yousearch").join("index.yswe");
        if h.exists() { return h; }
    }
    local.to_path_buf()
}

fn rdu32(r: &mut BufReader<File>) -> std::io::Result<u32> {
    let mut b = [0u8; 4]; r.read_exact(&mut b)?; Ok(u32::from_le_bytes(b))
}

fn rdu64(r: &mut BufReader<File>) -> std::io::Result<u64> {
    let mut b = [0u8; 8]; r.read_exact(&mut b)?; Ok(u64::from_le_bytes(b))
}

fn rdstr(r: &mut BufReader<File>, n: usize) -> std::io::Result<String> {
    let mut buf = vec![0u8; n]; r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

pub fn load_index(cfg: &Config) -> Vec<IndexEntry> {
    let file = match File::open(index_path()) { Ok(f) => f, Err(_) => return vec![] };
    let mut r = BufReader::new(file);

    let mut magic = [0u8; 4];
    if r.read_exact(&mut magic).is_err() { return vec![]; }

    let mut entries = Vec::new();

    if &magic == b"YSWE" {
        let mut _ver = [0u8; 1]; let _ = r.read_exact(&mut _ver);
        let count = match rdu32(&mut r) { Ok(c) => c, Err(_) => return vec![] };

        for _ in 0..count {
            let plen  = match rdu32(&mut r) { Ok(l) => l as usize, Err(_) => break };
            let path  = match rdstr(&mut r, plen)  { Ok(s) => s, Err(_) => break };
            let mtime = rdu64(&mut r).unwrap_or(0);

            let mut hc = [0u8; 1]; r.read_exact(&mut hc).ok();
            let content = if hc[0] == 1 {
                let clen = rdu32(&mut r).unwrap_or(0) as usize;
                rdstr(&mut r, clen).ok()
            } else {
                None
            };

            let tags = cfg.tags.get(&path).cloned().unwrap_or_default();
            entries.push(IndexEntry { path, mtime, content, tags });
        }
    } else {
        let count = u32::from_le_bytes(magic);
        for _ in 0..count {
            let len  = match rdu32(&mut r) { Ok(l) => l as usize, Err(_) => break };
            let path = match rdstr(&mut r, len)  { Ok(s) => s, Err(_) => break };
            entries.push(IndexEntry { path, mtime: 0, content: None, tags: vec![] });
        }
    }
    entries
}

pub fn save_index(entries: &[IndexEntry]) {
    let path = index_path();
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }

    let mut out = match File::create(&path) {
        Ok(f) => f,
        Err(e) => { eprintln!("{}", format!("  Index write error: {e}").red()); return; }
    };
    let _ = out.write_all(b"YSWE");
    let _ = out.write_all(&[2u8]);
    let _ = out.write_all(&(entries.len() as u32).to_le_bytes());

    for e in entries {
        let pb = e.path.as_bytes();
        let _ = out.write_all(&(pb.len() as u32).to_le_bytes());
        let _ = out.write_all(pb);
        let _ = out.write_all(&e.mtime.to_le_bytes());

        match &e.content {
            Some(c) => {
                let _ = out.write_all(&[1u8]);
                let cb = c.as_bytes();
                let _ = out.write_all(&(cb.len() as u32).to_le_bytes());
                let _ = out.write_all(cb);
            }
            None => { let _ = out.write_all(&[0u8]); }
        }
    }
}

fn is_text_ext(ext: &str, cfg: &Config) -> bool {
    cfg.extensions.iter().any(|e| e == ext)
}

fn is_excluded(path: &Path, cfg: &Config) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        cfg.excludes.iter().any(|e| e == s.as_ref())
    })
}

fn is_binary(path: &Path) -> bool {
    if let Ok(mut f) = File::open(path) {
        let mut buf = [0u8; 512];
        if let Ok(n) = f.read(&mut buf) {
            return buf[..n].contains(&0u8);
        }
    }
    false
}

fn read_content(path: &Path, max_kb: usize) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if meta.len() > max_kb as u64 * 1024 { return None; }
    if is_binary(path) { return None; }
    let mut s = String::new();
    let mut f = File::open(path).ok()?;
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

fn mtime(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0)
}

fn walk(dir: &Path, cfg: &Config, out: &mut Vec<IndexEntry>) {
    let rd = match fs::read_dir(dir) { Ok(r) => r, Err(_) => return };
    for entry in rd.flatten() {
        let p = entry.path();
        if is_excluded(&p, cfg) { continue; }
        if p.is_dir()  { walk(&p, cfg, out); }
        if !p.is_file() { continue; }
        let ext = p.extension().and_then(|e| e.to_str())
            .map(|e| format!(".{e}")).unwrap_or_default();
        let content = if is_text_ext(&ext, cfg) { read_content(&p, cfg.max_content_kb) } else { None };
        out.push(IndexEntry {
            path:    p.to_string_lossy().into_owned(),
            mtime:   mtime(&p),
            content,
            tags:    vec![],
        });
    }
}

fn walk_light(dir: &Path, cfg: &Config, out: &mut HashMap<String, u64>) {
    let rd = match fs::read_dir(dir) { Ok(r) => r, Err(_) => return };
    for entry in rd.flatten() {
        let p = entry.path();
        if is_excluded(&p, cfg) { continue; }
        if p.is_dir()  { walk_light(&p, cfg, out); }
        if p.is_file() { out.insert(p.to_string_lossy().into_owned(), mtime(&p)); }
    }
}

pub fn build_full(cfg: &Config) -> Vec<IndexEntry> {
    let mut entries = Vec::new();
    for root in &cfg.roots {
        let rp = Path::new(root);
        if rp.exists() { walk(rp, cfg, &mut entries); }
    }
    entries
}

pub fn build_incremental(cfg: &Config) -> (Vec<IndexEntry>, usize, usize, usize) {
    let old: HashMap<String, IndexEntry> = load_index(cfg)
        .into_iter().map(|e| (e.path.clone(), e)).collect();

    let mut fresh_mtimes: HashMap<String, u64> = HashMap::new();
    for root in &cfg.roots {
        let rp = Path::new(root);
        if rp.exists() { walk_light(rp, cfg, &mut fresh_mtimes); }
    }

    let mut added = 0usize;
    let mut updated = 0usize;
    let mut removed = 0usize;

    let result: Vec<IndexEntry> = fresh_mtimes.into_iter().map(|(path, mt)| {
        let tags = cfg.tags.get(&path).cloned().unwrap_or_default();
        if let Some(prev) = old.get(&path) {
            if prev.mtime == mt && mt != 0 {
                IndexEntry { path, mtime: mt, content: prev.content.clone(), tags }
            } else {
                updated += 1;
                let ext = Path::new(&path).extension().and_then(|e| e.to_str())
                    .map(|e| format!(".{e}")).unwrap_or_default();
                let content = if is_text_ext(&ext, cfg) {
                    read_content(Path::new(&path), cfg.max_content_kb)
                } else {
                    None
                };
                IndexEntry { path, mtime: mt, content, tags }
            }
        } else {
            added += 1;
            let ext = Path::new(&path).extension().and_then(|e| e.to_str())
                .map(|e| format!(".{e}")).unwrap_or_default();
            let content = if is_text_ext(&ext, cfg) {
                read_content(Path::new(&path), cfg.max_content_kb)
            } else {
                None
            };
            IndexEntry { path, mtime: mt, content, tags }
        }
    }).collect();

    let cur: std::collections::HashSet<&str> = result.iter().map(|e| e.path.as_str()).collect();
    for k in old.keys() { if !cur.contains(k.as_str()) { removed += 1; } }

    (result, added, updated, removed)
}

fn snapshot_mtimes(cfg: &Config) -> HashMap<String, u64> {
    let mut m = HashMap::new();
    for root in &cfg.roots {
        let rp = Path::new(root);
        if rp.exists() { walk_light(rp, cfg, &mut m); }
    }
    m
}

fn edit_dist(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 { return n; }
    if n == 0 { return m; }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            let c = if a[i-1] == b[j-1] { 0 } else { 1 };
            dp[i][j] = (dp[i-1][j] + 1).min(dp[i][j-1] + 1).min(dp[i-1][j-1] + c);
        }
    }
    dp[m][n]
}

fn is_subseq(needle: &str, haystack: &str) -> bool {
    let mut ni = needle.chars().peekable();
    for c in haystack.chars() {
        if let Some(&n) = ni.peek() { if c == n { ni.next(); } }
        if ni.peek().is_none() { return true; }
    }
    needle.is_empty()
}

fn synonyms(term: &str) -> Vec<String> {
    const MAP: &[(&str, &[&str])] = &[
        ("auth",   &["authentication","login","signin","token","jwt"]),
        ("config", &["configuration","settings","conf","cfg"]),
        ("db",     &["database","sql","sqlite","postgres","mysql"]),
        ("api",    &["endpoint","route","handler","controller"]),
        ("test",   &["spec","tests","testing","unittest"]),
        ("util",   &["utility","helper","helpers","utils","tools"]),
        ("err",    &["error","errors","exception","fault"]),
        ("fn",     &["func","function","def","method"]),
        ("str",    &["string","text","content"]),
        ("img",    &["image","picture","photo","icon"]),
        ("btn",    &["button","click","action"]),
    ];
    let mut out = vec![term.to_string()];
    for (k, vals) in MAP {
        if *k == term { out.extend(vals.iter().map(|v| v.to_string())); }
        for v in *vals { if *v == term { out.push(k.to_string()); } }
    }
    out.dedup();
    out
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub path:          String,
    pub score:         i32,
    pub matched_lines: Vec<(usize, String)>,
}

fn score_entry(entry: &IndexEntry, terms: &[String], threshold: usize) -> Option<MatchResult> {
    let fname = Path::new(&entry.path)
        .file_name().and_then(|n| n.to_str()).unwrap_or(&entry.path);
    let fl = fname.to_lowercase();
    let pl = entry.path.to_lowercase();

    let mut total:     i32  = 0;
    let mut any_match: bool = false;
    let mut matched_lines: Vec<(usize, String)> = Vec::new();

    for term in terms {
        let mut s: i32 = 0;

        if fl.contains(term.as_str()) {
            s += 120;
            if fl == *term            { s += 80; }
            if fl.starts_with(term.as_str()) { s += 30; }
            any_match = true;
        }
        if pl.contains(term.as_str()) { s += 40; any_match = true; }

        if s == 0 {
            for token in fl.split(|c: char| !c.is_alphanumeric()) {
                if token.is_empty() { continue; }
                let d = edit_dist(term, token);
                if d <= threshold {
                    s += 35 - (d as i32 * 8);
                    any_match = true;
                    break;
                }
            }
        }

        if s == 0 && term.len() >= 2 && is_subseq(term, &fl) {
            s += 18;
            any_match = true;
        }

        if let Some(ref content) = entry.content {
            let mut hits = 0i32;
            for (ln, line) in content.lines().enumerate() {
                let ll = line.to_lowercase();
                if ll.contains(term.as_str()) {
                    hits += 1;
                    if matched_lines.len() < 3 {
                        matched_lines.push((ln + 1, line.trim().to_string()));
                    }
                    any_match = true;
                }
            }
            if hits > 0 {
                s += 8 + hits.min(25);
                if matched_lines.first().map_or(false, |(ln, _)| *ln <= 10) { s += 5; }
            }
        }

        total += s;
    }

    if any_match {
        Some(MatchResult { path: entry.path.clone(), score: total, matched_lines })
    } else {
        None
    }
}

pub fn search(entries: &[IndexEntry], raw: &str, cfg: &Config) -> Vec<MatchResult> {
    let mut include:    Vec<String>    = Vec::new();
    let mut exclude:    Vec<String>    = Vec::new();
    let mut limit                      = cfg.default_limit;
    let mut tag_filter: Option<String> = None;

    for part in raw.split_whitespace() {
        if let Some(r) = part.strip_prefix('!') {
            exclude.push(r.to_lowercase());
        } else if let Some(r) = part.strip_prefix("--limit=") {
            limit = r.parse().unwrap_or(limit);
        } else if let Some(r) = part.strip_prefix('#') {
            tag_filter = Some(r.to_lowercase());
        } else {
            include.extend(synonyms(&part.to_lowercase()));
        }
    }
    include.dedup();

    let mut results: Vec<MatchResult> = entries.iter()
        .filter(|e| !exclude.iter().any(|t| e.path.to_lowercase().contains(t.as_str())))
        .filter(|e| {
            tag_filter.as_ref().map_or(true, |tag| {
                e.tags.iter().any(|t| &t.to_lowercase() == tag)
            })
        })
        .filter_map(|e| {
            if include.is_empty() {
                Some(MatchResult { path: e.path.clone(), score: 0, matched_lines: vec![] })
            } else {
                score_entry(e, &include, cfg.fuzzy_threshold)
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);
    results
}

fn safe_trunc(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None           => s,
    }
}

fn highlight(text: &str, terms: &[String]) -> String {
    if terms.is_empty() { return text.to_string(); }
    let mut out   = String::with_capacity(text.len() + 32);
    let lower     = text.to_lowercase();
    let mut pos   = 0usize;

    'main: while pos < text.len() {
        if !text.is_char_boundary(pos) { pos += 1; continue; }

        for term in terms {
            if term.is_empty() { continue; }
            let end = pos + term.len();
            if end > lower.len() { continue; }
            if !lower.is_char_boundary(end) || !text.is_char_boundary(end) { continue; }
            if &lower[pos..end] == term.as_str() {
                out.push_str(&text[pos..end].yellow().bold().to_string());
                pos = end;
                continue 'main;
            }
        }

        let ch = text[pos..].chars().next().map_or(1, |c| c.len_utf8());
        out.push_str(&text[pos..pos + ch]);
        pos += ch;
    }
    out
}

pub fn print_results(results: &[MatchResult], terms: &[String]) {
    if results.is_empty() {
        println!("\n{}", "  ✗  No matches found.".red().bold());
        println!("{}", "     Try shorter terms or check for typos.".dimmed());
        return;
    }

    println!();
    println!("{}{}{}",
        "╔══ ".cyan(),
        format!("  {} result(s) ", results.len()).white().bold().on_bright_black(),
        " ══════════════════════════════════════╗".cyan()
    );
    println!();

    for (i, r) in results.iter().enumerate() {
        let p    = Path::new(&r.path);
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or(&r.path);
        let dir  = p.parent().and_then(|p| p.to_str()).unwrap_or(".");

        println!("  {}  {}  {}",
            format!("{:>3}.", i + 1).dimmed(),
            highlight(name, terms).white().bold(),
            format!("↑{}", r.score).dimmed()
        );
        println!("       {} {}", "▸".cyan().dimmed(), dir.dimmed());

        for (ln, line) in &r.matched_lines {
            let snip = safe_trunc(line.trim(), 90);
            println!("       {} {}  {}",
                format!("L{ln}").bright_cyan(),
                "│".dimmed(),
                highlight(snip, terms).dimmed()
            );
        }
        println!();
    }

    println!("{}", format!("  ✓  {} file(s) matched.", results.len()).green().bold());
}

pub fn cmd_index(args: &[String], cfg: &Config) {
    let watch       = args.iter().any(|a| a == "--watch" || a == "-w");
    let incremental = args.iter().any(|a| a == "--incremental" || a == "-i");

    banner();
    println!("{}", format!("  Indexing {} root(s)…", cfg.roots.len()).dimmed());
    for r in &cfg.roots { println!("  {} {}", "▸".cyan(), r); }
    println!();

    let (entries, added, updated, removed) = if incremental {
        println!("{}", "  Mode: Incremental".yellow());
        build_incremental(cfg)
    } else {
        println!("{}", "  Mode: Full rebuild".yellow());
        let e = build_full(cfg);
        let n = e.len();
        (e, n, 0, 0)
    };

    save_index(&entries);

    let with_content = entries.iter().filter(|e| e.content.is_some()).count();

    println!();
    if incremental {
        println!("{}", format!("  +{added} added  ~{updated} updated  -{removed} removed").dimmed());
    }
    println!("{}", format!("  ✓  {} files indexed  ({} with content)", entries.len(), with_content).green().bold());

    if watch {
        println!();
        println!("{}", "  👁  Watching for changes — Ctrl+C to stop".yellow().bold());
        watch_loop(cfg);
    }
}

pub fn cmd_search(query: &str, cfg: &Config) {
    if query.is_empty() {
        eprintln!("{}", "  Usage: yousearch search <query>".red());
        return;
    }

    if !index_path().exists() {
        println!("{}", "  ⚙  No index found. Building now…".yellow());
        let e = build_full(cfg);
        save_index(&e);
        println!("{}", format!("  ✓  {} files indexed.", e.len()).green());
        println!();
    }

    let entries = load_index(cfg);
    let terms: Vec<String> = query.split_whitespace()
        .filter(|p| !p.starts_with('!') && !p.starts_with("--") && !p.starts_with('#'))
        .map(|p| p.to_lowercase())
        .collect();

    let results = search(&entries, query, cfg);
    print_results(&results, &terms);

    let mut c = cfg.clone();
    c.history.push(query.to_string());
    if c.history.len() > 100 { c.history.remove(0); }
    save_config(&c);
}

pub fn cmd_add(path: &str, cfg: &mut Config) {
    let abs = fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string());

    if cfg.roots.contains(&abs) {
        println!("{}", format!("  ℹ  '{}' already indexed.", abs).yellow());
    } else {
        cfg.roots.push(abs.clone());
        save_config(cfg);
        println!("{}", format!("  ✓  Added '{abs}' — run 'yousearch index' to update.").green());
    }
}

pub fn cmd_tag(file: &str, tag: &str, cfg: &mut Config) {
    cfg.tags.entry(file.to_string()).or_default().push(tag.to_string());
    save_config(cfg);
    println!("{}", format!("  ✓  Tagged '{file}' with #{tag}").green());
}

pub fn cmd_untag(file: &str, tag: &str, cfg: &mut Config) {
    if let Some(tags) = cfg.tags.get_mut(file) { tags.retain(|t| t != tag); }
    save_config(cfg);
    println!("{}", format!("  ✓  Removed #{tag} from '{file}'").green());
}

pub fn cmd_tags(cfg: &Config) {
    if cfg.tags.is_empty() { println!("{}", "  No tags set yet.".dimmed()); return; }
    println!("{}", "  🏷  Tags:".cyan().bold());
    for (file, tags) in &cfg.tags {
        println!("  {} → {}",
            file.dimmed(),
            tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ").yellow()
        );
    }
}

pub fn cmd_history(cfg: &Config) {
    if cfg.history.is_empty() { println!("{}", "  No history yet.".dimmed()); return; }
    println!("{}", "  🕐 Search History:".cyan().bold());
    for (i, q) in cfg.history.iter().rev().take(20).enumerate() {
        println!("  {}  {}", format!("{:>2}.", i + 1).dimmed(), q);
    }
}

pub fn cmd_status(cfg: &Config) {
    let ip = index_path();
    println!("{}", "  📊 YouSearch Status".cyan().bold());
    println!("  {} {}", "Config :".yellow(), config_path().display());
    println!("  {} {}", "Index  :".yellow(), ip.display());
    if ip.exists() {
        let entries      = load_index(cfg);
        let with_content = entries.iter().filter(|e| e.content.is_some()).count();
        let size         = fs::metadata(&ip).map(|m| m.len()).unwrap_or(0);
        println!("  {} {} files",   "Entries:".yellow(), entries.len());
        println!("  {} {} files",   "Content:".yellow(), with_content);
        println!("  {} {:.2} MB",   "Size   :".yellow(), size as f64 / 1_048_576.0);
    } else {
        println!("  {}", "No index found — run 'yousearch index'.".red());
    }
    println!("  {} {:?}", "Roots  :".yellow(), cfg.roots);
}

pub fn cmd_config_show(cfg: &Config) {
    println!("{}", "  ⚙  Config".cyan().bold());
    println!("  {} {:?}",  "roots      :".yellow(), cfg.roots);
    println!("  {} {:?}",  "excludes   :".yellow(), cfg.excludes);
    println!("  {} {}",    "limit      :".yellow(), cfg.default_limit);
    println!("  {} {}",    "fuzzy      :".yellow(), cfg.fuzzy_threshold);
    println!("  {} {}KB",  "max_content:".yellow(), cfg.max_content_kb);
    println!("  {} {}",    "web_port   :".yellow(), cfg.web_port);
}

fn watch_loop(cfg: &Config) {
    let mut known = snapshot_mtimes(cfg);
    loop {
        thread::sleep(Duration::from_secs(2));
        let current = snapshot_mtimes(cfg);
        let changed = current.iter().any(|(p, m)| known.get(p).map_or(true, |km| km != m));
        let removed = known.keys().any(|p| !current.contains_key(p));
        if changed || removed {
            print!("  🔄 Change detected — updating… ");
            let _ = std::io::stdout().flush();
            let (entries, a, u, r) = build_incremental(cfg);
            save_index(&entries);
            println!("{}", format!("+{a} ~{u} -{r} ({} total)", entries.len()).green());
            known = current;
        }
    }
}

pub fn cmd_serve(cfg: &Config) {
    let port = cfg.web_port;
    let addr = format!("127.0.0.1:{port}");

    let listener = match TcpListener::bind(&addr) {
        Ok(l)  => l,
        Err(e) => { eprintln!("{}", format!("  ✗  Cannot bind {addr}: {e}").red()); return; }
    };

    let url = format!("http://localhost:{port}");
    println!("{}", format!("  🌐 Web UI: {url}").cyan().bold());
    println!("{}", "     Ctrl+C to stop.".dimmed());

    if cfg!(target_os = "windows") {
        let _ = std::process::Command::new("cmd").args(["/C", "start", &url]).spawn();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    } else {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }

    let cfg_arc = Arc::new(cfg.clone());
    for stream in listener.incoming() {
        let cfg_ref = Arc::clone(&cfg_arc);
        if let Ok(s) = stream {
            thread::spawn(move || handle_http(s, &cfg_ref));
        }
    }
}

fn handle_http(mut stream: std::net::TcpStream, cfg: &Config) {
    let clone = match stream.try_clone() { Ok(c) => c, Err(_) => return };
    let mut reader = BufReader::new(clone);

    let mut req = String::new();
    let _ = reader.read_line(&mut req);
    loop {
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        if line.trim().is_empty() { break; }
    }

    let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();

    let response = if path == "/" || path == "/index.html" {
        let html = web_html(cfg.web_port);
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", html.len(), html)
    } else if path.starts_with("/search?q=") {
        let q   = url_decode(&path[10..]);
        let idx = load_index(cfg);
        let res = search(&idx, &q, cfg);
        let json = to_json(&res, &q);
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", json.len(), json)
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
    };

    let _ = stream.write_all(response.as_bytes());
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(b) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                out.push(b as char);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ").replace('\r', "")
}

fn to_json(results: &[MatchResult], query: &str) -> String {
    let items: Vec<String> = results.iter().map(|r| {
        let p    = Path::new(&r.path);
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or(&r.path);
        let dir  = p.parent().and_then(|p| p.to_str()).unwrap_or(".");
        let lines: Vec<String> = r.matched_lines.iter().map(|(ln, text)| {
            let t = safe_trunc(text, 120);
            format!("{{\"line\":{ln},\"text\":\"{}\"}}", esc(t))
        }).collect();
        format!(
            "{{\"path\":\"{}\",\"name\":\"{}\",\"dir\":\"{}\",\"score\":{},\"lines\":[{}]}}",
            esc(&r.path), esc(name), esc(dir), r.score, lines.join(",")
        )
    }).collect();
    format!(
        "{{\"query\":\"{}\",\"count\":{},\"results\":[{}]}}",
        esc(query), results.len(), items.join(",")
    )
}

fn web_html(port: u16) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>YouSearch</title>
<style>
:root{{--bg:#0b0c10;--surf:#13141a;--card:#1a1c25;--border:#252636;--accent:#7f6cf7;--gold:#f4c842;--text:#dde1f5;--muted:#5c5e7a;--green:#4df0a0;--red:#f06a6a}}
*{{box-sizing:border-box;margin:0;padding:0}}
body{{background:var(--bg);color:var(--text);font-family:'Segoe UI',system-ui,sans-serif;min-height:100vh}}
.hero{{background:linear-gradient(135deg,#0f0f1e,#13142a);padding:2.5rem 2rem 2rem;border-bottom:1px solid var(--border)}}
.logo{{font-size:2.2rem;font-weight:900;background:linear-gradient(120deg,var(--accent),var(--gold));-webkit-background-clip:text;-webkit-text-fill-color:transparent}}
.sub{{color:var(--muted);margin:.2rem 0 1.5rem;font-size:.875rem}}
.row{{display:flex;gap:.5rem}}
input{{flex:1;background:var(--surf);border:1.5px solid var(--border);color:var(--text);padding:.7rem 1rem;border-radius:8px;font-size:1rem;outline:none;transition:border .15s}}
input:focus{{border-color:var(--accent);box-shadow:0 0 0 3px #7f6cf718}}
button{{background:var(--accent);color:#fff;border:none;padding:.7rem 1.4rem;border-radius:8px;cursor:pointer;font-weight:700;font-size:.9rem;transition:opacity .15s}}
button:hover{{opacity:.85}}
.chips{{margin-top:.7rem;display:flex;gap:.4rem;flex-wrap:wrap}}
.chip{{background:var(--card);border:1px solid var(--border);color:var(--muted);font-size:.75rem;padding:2px 9px;border-radius:20px;font-family:monospace}}
.wrap{{max-width:860px;margin:2rem auto;padding:0 1.5rem}}
.meta{{display:flex;align-items:center;gap:.5rem;margin-bottom:1.2rem}}
.meta-text{{color:var(--muted);font-size:.85rem}}
.badge{{background:var(--accent);color:#fff;font-size:.72rem;padding:2px 10px;border-radius:20px;font-weight:700}}
.card{{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:1rem 1.2rem;margin-bottom:.7rem;cursor:pointer;transition:border-color .15s,transform .1s,box-shadow .15s}}
.card:hover{{border-color:var(--accent);transform:translateY(-2px);box-shadow:0 4px 20px #7f6cf722}}
.card-top{{display:flex;align-items:center;gap:.6rem}}
.fname{{font-weight:700;font-size:1rem;color:#fff}}
.score{{margin-left:auto;font-size:.72rem;color:var(--muted);background:var(--bg);padding:2px 8px;border-radius:20px}}
.fdir{{font-size:.78rem;color:var(--muted);margin-top:.2rem}}
.lines{{margin-top:.6rem;border-top:1px solid var(--border);padding-top:.6rem;display:flex;flex-direction:column;gap:2px}}
.ln{{font-family:'Consolas',monospace;font-size:.78rem;color:#8a8cb0;display:flex;gap:.5rem}}
.lnum{{color:var(--accent);flex-shrink:0}}
mark{{background:transparent;color:var(--gold);font-weight:700}}
.empty{{text-align:center;color:var(--muted);padding:4rem 2rem}}
.empty-icon{{font-size:3rem;margin-bottom:1rem}}
.copied{{position:fixed;bottom:1.5rem;right:1.5rem;background:var(--green);color:#000;padding:.5rem 1rem;border-radius:8px;font-weight:700;opacity:0;transition:opacity .3s;pointer-events:none}}
.spin{{display:inline-block;width:14px;height:14px;border:2px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:s .6s linear infinite;vertical-align:middle;margin-right:.4rem}}
@keyframes s{{to{{transform:rotate(360deg)}}}}
</style>
</head>
<body>
<div class="hero">
  <div class="logo">⚡ YouSearch</div>
  <div class="sub">Intelligent file &amp; content search</div>
  <div class="row">
    <input id="q" type="text" placeholder="Search files, code, content…" autofocus />
    <button onclick="go()">Search</button>
  </div>
  <div class="chips">
    <span class="chip">!exclude</span>
    <span class="chip">#tag</span>
    <span class="chip">--limit=N</span>
    <span class="chip">fuzzy</span>
    <span class="chip">content search</span>
  </div>
</div>

<div class="wrap" id="out">
  <div class="empty"><div class="empty-icon">🔍</div><div>Type above to search your files</div></div>
</div>

<div class="copied" id="copied">✓ Path copied!</div>

<script>
const PORT={port};
let timer;
document.getElementById('q').addEventListener('input',()=>{{clearTimeout(timer);timer=setTimeout(go,220)}});
document.getElementById('q').addEventListener('keydown',e=>{{if(e.key==='Enter')go()}});

function hl(txt,terms){{
  if(!terms.length)return txt;
  let r=txt;
  terms.forEach(t=>{{
    const re=new RegExp('('+t.replace(/[.*+?^${{}}()|[\]\\]/g,'\\$&')+')','gi');
    r=r.replace(re,'<mark>$1</mark>');
  }});
  return r;
}}

async function go(){{
  const q=document.getElementById('q').value.trim();
  const el=document.getElementById('out');
  if(!q){{el.innerHTML='<div class="empty"><div class="empty-icon">🔍</div><div>Type above to search</div></div>';return}}
  el.innerHTML='<div class="empty"><span class="spin"></span>Searching…</div>';
  try{{
    const res=await fetch(`http://localhost:${{PORT}}/search?q=${{encodeURIComponent(q)}}`);
    const data=await res.json();
    const terms=q.split(/\s+/).filter(p=>!p.startsWith('!')&&!p.startsWith('--')&&!p.startsWith('#')).map(p=>p.toLowerCase());
    if(!data.count){{el.innerHTML='<div class="empty"><div class="empty-icon">✗</div><div>No matches for "'+q+'"</div></div>';return}}
    el.innerHTML=
      `<div class="meta"><span class="meta-text">Results</span><span class="badge">${{data.count}}</span></div>`+
      data.results.map(r=>`
        <div class="card" onclick="copy('${{r.path.replace(/\\/g,'\\\\').replace(/'/g,"\\'")}}')" title="Click to copy path">
          <div class="card-top">
            <span class="fname">${{hl(r.name,terms)}}</span>
            <span class="score">↑${{r.score}}</span>
          </div>
          <div class="fdir">📁 ${{r.dir}}</div>
          ${{r.lines.length?`<div class="lines">${{r.lines.map(l=>`<div class="ln"><span class="lnum">L${{l.line}}</span><span>${{hl(l.text,terms)}}</span></div>`).join('')}}</div>`:''}}</div>`).join('');
  }}catch(e){{el.innerHTML='<div class="empty">❌ '+e.message+'</div>'}}
}}

function copy(p){{
  navigator.clipboard&&navigator.clipboard.writeText(p);
  const el=document.getElementById('copied');
  el.style.opacity='1';
  setTimeout(()=>el.style.opacity='0',1800);
}}
</script>
</body>
</html>"##, port = port)
}

fn cmd_setup() {
    banner();
    println!("{}", "  Interactive Setup".white().bold());
    println!();

    let path = config_path();
    if !path.exists() {
        save_config(&Config::default());
        println!("{}", format!("  ✓  Config created at {}", path.display()).green());
    } else {
        println!("{}", format!("  ℹ  Config already at {}", path.display()).yellow());
    }

    print!("\n  Add to PATH? (y/n): ");
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    if input.trim().eq_ignore_ascii_case("y") {
        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                let ds  = dir.to_string_lossy();
                let cur = env::var("PATH").unwrap_or_default();
                if !cur.contains(ds.as_ref()) {
                    let _ = std::process::Command::new("setx")
                        .args(["PATH", &format!("{ds};{cur}")])
                        .status();
                    println!("{}", "  ✓  Added to PATH. Restart terminal.".green());
                } else {
                    println!("{}", "  ℹ  Already in PATH.".yellow());
                }
            }
        }
    }

    println!();
    print_help();
}

fn home_dir() -> Option<PathBuf> {
    env::var("USERPROFILE").or_else(|_| env::var("HOME"))
        .ok().map(PathBuf::from)
}

fn banner() {
    println!("{}", "  ╔══════════════════════════════════════╗".cyan());
    println!("{}", "  ║  ⚡  YouSearch v2.0                  ║".cyan().bold());
    println!("{}", "  ╚══════════════════════════════════════╝".cyan());
    println!();
}

fn print_help() {
    banner();
    println!("{}", "  Commands:".yellow().bold());
    let cmds: &[(&str, &str)] = &[
        ("index",            "Build / rebuild the index"),
        ("index --watch",    "Live indexing — auto-update on changes"),
        ("index -i",         "Incremental update (fast, changed files only)"),
        ("search <query>",   "Search files by name & content"),
        ("add <path>",       "Add a directory to the index"),
        ("tag <file> <tag>", "Tag a file with a label"),
        ("untag <f> <t>",    "Remove a tag"),
        ("tags",             "List all tags"),
        ("serve",            "Launch web UI on localhost"),
        ("status",           "Show index statistics"),
        ("history",          "Recent search history"),
        ("config",           "Show current config"),
        ("setup",            "Interactive first-run setup"),
    ];
    for (c, d) in cmds {
        println!("  {}  {}", format!("{c:<24}").white().bold(), d.dimmed());
    }
    println!();
    println!("{}", "  Query syntax:".yellow().bold());
    println!("  {}  exclude term",   "!term     ".white());
    println!("  {}  filter by tag",  "#tag      ".white());
    println!("  {}  max results",    "--limit=N ".white());
    println!();
    println!("{}", "  Examples:".yellow().bold());
    println!("  yousearch main");
    println!("  yousearch auth !test --limit=5");
    println!("  yousearch config #backend");
    println!("  yousearch index --watch");
    println!("  yousearch serve");
}

fn main() {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let args: Vec<String> = env::args().collect();
    let mut cfg = load_config();

    if args.len() == 1 {
        cmd_setup();
        return;
    }

    let cmd  = args[1].to_lowercase();
    let rest = &args[2..];

    match cmd.as_str() {
        "index" | "i"                  => cmd_index(rest, &cfg),
        "search" | "s" | "find" | "f"  => cmd_search(&rest.join(" "), &cfg),
        "add"                           => {
            if rest.is_empty() { eprintln!("{}", "  Usage: yousearch add <path>".red()); }
            else { cmd_add(&rest[0], &mut cfg); }
        }
        "tag"                           => {
            if rest.len() < 2 { eprintln!("{}", "  Usage: yousearch tag <file> <tag>".red()); }
            else { cmd_tag(&rest[0], &rest[1], &mut cfg); }
        }
        "untag"                         => {
            if rest.len() < 2 { eprintln!("{}", "  Usage: yousearch untag <file> <tag>".red()); }
            else { cmd_untag(&rest[0], &rest[1], &mut cfg); }
        }
        "tags"                          => cmd_tags(&cfg),
        "serve" | "web" | "ui"         => cmd_serve(&cfg),
        "status" | "stat"              => cmd_status(&cfg),
        "history" | "hist"             => cmd_history(&cfg),
        "config" | "cfg"               => cmd_config_show(&cfg),
        "setup" | "install" | "init"   => cmd_setup(),
        "help" | "--help" | "-h"       => print_help(),
        _                               => cmd_search(&args[1..].join(" "), &cfg),
    }
}