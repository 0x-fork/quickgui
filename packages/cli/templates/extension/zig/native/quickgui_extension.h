/* QuickGUI service extension ABI v1. No renderer, host, or C++ runtime dependency.
 * This header may be vendored into an independently built extension package.
 * Keep layouts in sync with src/extension_api.rs. */
#ifndef QUICKGUI_EXTENSION_H
#define QUICKGUI_EXTENSION_H

#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
#define QUICKGUI_EXTENSION_EXPORT __declspec(dllexport)
#else
#define QUICKGUI_EXTENSION_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define QUICKGUI_EXTENSION_ABI_V1 1
#define QUICKGUI_EXTENSION_SERVICE 2
#define QUICKGUI_EXTENSION_MAX_PAYLOAD (64 * 1024)
#define QUICKGUI_EXTENSION_REPLY 0
#define QUICKGUI_EXTENSION_ERROR 1
#define QUICKGUI_EXTENSION_EVENT 2

typedef struct QuickGuiBytes {
    const uint8_t *data;
    size_t len;
} QuickGuiBytes;

/* Ownership transfers to invoke, including error paths. emit borrows its bytes
 * only for the call. release must be called exactly once after the last callback.
 * Replies/events contain JSON; errors contain UTF-8 text. */
typedef struct QuickGuiServiceSink {
    void *context;
    void (*emit)(void *context, uint32_t kind, QuickGuiBytes payload);
    void (*release)(void *context);
} QuickGuiServiceSink;

/* Copy any borrowed inputs retained after returning. Enqueue slow work; never block on UI,
 * network, or worker completion. shutdown cancels sessions without waiting.
 * QuickGUI calls from its UI goroutine; workers may emit/release from any thread.
 * A start request's ID identifies its session and its retained event sink. */
typedef struct QuickGuiServiceApi {
    void (*invoke)(uint32_t request, QuickGuiBytes method, QuickGuiBytes params,
                   QuickGuiServiceSink sink);
    void (*shutdown)(void);
} QuickGuiServiceApi;

/* All fields and the pointed-to table remain valid until process exit.
 * version is the provider's own exact release, not the core's release. */
typedef struct QuickGuiExtension {
    uint32_t abi_version;
    uint32_t descriptor_size;
    uint32_t kind;
    uint32_t api_size;
    QuickGuiBytes name;
    QuickGuiBytes version;
    const void *api;
} QuickGuiExtension;

QUICKGUI_EXTENSION_EXPORT const QuickGuiExtension *quickgui_extension_v1(void);

#ifdef __cplusplus
}
#endif
#endif
