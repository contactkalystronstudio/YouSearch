@echo off
setlocal EnableDelayedExpansion

set SRC=indexer.cpp
set OBJ=indexer.o
set OUT=indexer.dll
set FLAGS=-std=c++17 -O2 -Wall -Wextra

where g++ >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] g++ not found. Install MinGW-w64 and add it to PATH.
    exit /b 1
)

for /f "tokens=*" %%v in ('g++ --version 2^>^&1 ^| findstr /r "[0-9][0-9]*\.[0-9]"') do set GVER=%%v
echo [INFO] Compiler: !GVER!

echo [INFO] Compiling %SRC%...
g++ %FLAGS% -c -o %OBJ% %SRC%
if %errorlevel% neq 0 (
    echo [ERROR] Compilation failed.
    exit /b 1
)

echo [INFO] Linking %OUT%...
g++ -shared -o %OUT% %OBJ% -lstdc++fs
if %errorlevel% neq 0 (
    echo [INFO] Retrying without -lstdc++fs (GCC 9+)...
    g++ -shared -o %OUT% %OBJ%
    if %errorlevel% neq 0 (
        echo [ERROR] Link failed.
        del /f %OBJ% 2>nul
        exit /b 1
    )
)

del /f %OBJ% 2>nul

echo [OK] Built %OUT%
echo [OK] Place %OUT% next to yousearch.exe

endlocal