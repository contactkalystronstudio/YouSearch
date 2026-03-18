#include <chrono>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <set>
#include <sstream>
#include <string>
#include <vector>

namespace fs  = std::filesystem;
namespace chr = std::chrono;

#ifdef _WIN32
  #define YS_EXPORT extern "C" __declspec(dllexport)
#else
  #define YS_EXPORT extern "C" __attribute__((visibility("default")))
#endif

static const uint64_t MAX_BYTES = 512ULL * 1024ULL;

static const std::set<std::string> TEXT_EXTS = {
    ".rs",".cpp",".c",".h",".hpp",
    ".txt",".md",".json",".toml",".yaml",".yml",
    ".py",".js",".ts",".html",".css",".xml",
    ".sh",".bat",".cmake"
};

static const std::set<std::string> SKIP_DIRS = {
    ".git","node_modules","target",".vs","__pycache__",
    "dist","build",".cache","vendor",".idea","out"
};

static bool should_skip_dir(const fs::path& p) {
    const auto name = p.filename().string();
    return SKIP_DIRS.count(name) > 0;
}

static bool is_text_file(const fs::path& p) {
    return TEXT_EXTS.count(p.extension().string()) > 0;
}

static std::string to_fwd(std::string s) {
    for (auto& c : s) if (c == '\\') c = '/';
    return s;
}

static uint64_t file_mtime(const fs::path& p) {
    std::error_code ec;
    auto ftime   = fs::last_write_time(p, ec);
    if (ec) return 0;

    auto fc_now  = fs::file_time_type::clock::now();
    auto sys_now = chr::system_clock::now();
    auto diff    = ftime - fc_now;
    auto as_sys  = sys_now + chr::duration_cast<chr::system_clock::duration>(diff);

    auto secs = chr::duration_cast<chr::seconds>(as_sys.time_since_epoch()).count();
    return static_cast<uint64_t>(secs > 0 ? secs : 0);
}

static std::string read_text(const fs::path& p) {
    std::error_code ec;
    auto sz = fs::file_size(p, ec);
    if (ec || sz == 0 || sz > MAX_BYTES) return {};

    std::ifstream f(p, std::ios::binary);
    if (!f) return {};

    std::string buf(static_cast<size_t>(sz), '\0');
    f.read(buf.data(), static_cast<std::streamsize>(sz));
    buf.resize(static_cast<size_t>(f.gcount()));

    const size_t probe = buf.size() < 512 ? buf.size() : 512;
    for (size_t i = 0; i < probe; ++i)
        if (buf[i] == '\0') return {};

    return buf;
}

static void wu8 (std::ofstream& o, uint8_t  v) { o.write(reinterpret_cast<const char*>(&v), 1); }
static void wu32(std::ofstream& o, uint32_t v) { o.write(reinterpret_cast<const char*>(&v), 4); }
static void wu64(std::ofstream& o, uint64_t v) { o.write(reinterpret_cast<const char*>(&v), 8); }
static void wstr(std::ofstream& o, const std::string& s) {
    wu32(o, static_cast<uint32_t>(s.size()));
    o.write(s.c_str(), static_cast<std::streamsize>(s.size()));
}

struct Entry {
    std::string path;
    uint64_t    mtime   = 0;
    std::string content;
};

static void collect(const fs::path& root, std::vector<Entry>& out) {
    std::error_code ec;
    auto it = fs::recursive_directory_iterator(
        root, fs::directory_options::skip_permission_denied, ec);
    if (ec) return;

    for (auto& ent : it) {
        if (ent.is_directory()) {
            if (should_skip_dir(ent.path()))
                it.disable_recursion_pending();
            continue;
        }
        if (!ent.is_regular_file()) continue;

        const auto& p = ent.path();
        std::string content;
        if (is_text_file(p)) content = read_text(p);

        out.push_back({ to_fwd(p.string()), file_mtime(p), std::move(content) });
    }
}

static bool flush_index(const std::vector<Entry>& entries, const std::string& dest) {
    std::ofstream out(dest, std::ios::binary);
    if (!out) return false;

    out.write("YSWE", 4);
    wu8 (out, 2);
    wu32(out, static_cast<uint32_t>(entries.size()));

    for (const auto& e : entries) {
        wstr(out, e.path);
        wu64(out, e.mtime);
        if (!e.content.empty()) {
            wu8(out, 1);
            wstr(out, e.content);
        } else {
            wu8(out, 0);
        }
    }
    return out.good();
}

YS_EXPORT
int build_index_full(const char* roots_pipe, const char* out_path) {
    std::vector<Entry> entries;
    std::istringstream ss(roots_pipe ? roots_pipe : ".");
    std::string root;
    while (std::getline(ss, root, '|')) {
        if (!root.empty()) {
            std::error_code ec;
            if (fs::exists(root, ec) && !ec)
                collect(fs::path(root), entries);
        }
    }
    const std::string dest = (out_path && out_path[0]) ? out_path : "index.yswe";
    return flush_index(entries, dest) ? 0 : 1;
}

YS_EXPORT
int build_index(void) {
    return build_index_full(".", nullptr);
}