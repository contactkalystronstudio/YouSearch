#pragma once

#ifdef _WIN32
  #define YS_EXPORT extern "C" __declspec(dllexport)
#else
  #define YS_EXPORT extern "C" __attribute__((visibility("default")))
#endif

YS_EXPORT int build_index_full(const char* roots_pipe, const char* out_path);
YS_EXPORT int build_index(void);