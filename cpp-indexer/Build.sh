#!/usr/bin/env bash
set -euo pipefail

SRC="indexer.cpp"
OUT="libindexer.so"
FLAGS="-std=c++17 -O2 -Wall -Wextra -fPIC"

if ! command -v g++ &>/dev/null; then
    echo "[ERROR] g++ not found."
    echo "  Ubuntu/Debian : sudo apt install g++"
    echo "  Fedora/RHEL   : sudo dnf install gcc-c++"
    echo "  Arch          : sudo pacman -S gcc"
    exit 1
fi

GVER=$(g++ --version | head -1)
echo "[INFO] Compiler: $GVER"

GCC_MAJOR=$(g++ -dumpversion | cut -d. -f1)
if [ "$GCC_MAJOR" -lt 8 ]; then
    echo "[ERROR] GCC 8+ required for <filesystem>. Found GCC $GCC_MAJOR."
    exit 1
fi

echo "[INFO] Compiling $SRC..."

if [ "$GCC_MAJOR" -ge 9 ]; then
    g++ $FLAGS -shared -o "$OUT" "$SRC"
else
    g++ $FLAGS -shared -o "$OUT" "$SRC" -lstdc++fs
fi

echo "[OK] Built $OUT"
echo "[INFO] To use: place $OUT next to the yousearch binary,"
echo "       or set LD_LIBRARY_PATH to this directory."