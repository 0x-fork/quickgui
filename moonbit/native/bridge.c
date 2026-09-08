/* MoonBit native FFI transport. Platform callbacks only copy into a bounded C
 * queue: MoonBit allocations and callbacks stay on its single UI worker thread.
 * The host owns AppKit/Winit on the process main thread. No polling or IPC. */
#include "moonbit.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#include <process.h>
static CRITICAL_SECTION queue_lock;
static CONDITION_VARIABLE queue_wake;
static void lock_queue(void) { EnterCriticalSection(&queue_lock); }
static void unlock_queue(void) { LeaveCriticalSection(&queue_lock); }
static void wake_queue(void) { WakeConditionVariable(&queue_wake); }
static void wait_queue(void) { SleepConditionVariableCS(&queue_wake, &queue_lock, INFINITE); }
#else
#include <dlfcn.h>
#include <pthread.h>
#include <unistd.h>
#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif
static pthread_mutex_t queue_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t queue_wake = PTHREAD_COND_INITIALIZER;
static void lock_queue(void) { pthread_mutex_lock(&queue_lock); }
static void unlock_queue(void) { pthread_mutex_unlock(&queue_lock); }
static void wake_queue(void) { pthread_cond_signal(&queue_wake); }
static void wait_queue(void) { pthread_cond_wait(&queue_wake, &queue_lock); }
#endif

typedef void (*event_callback)(const uint8_t *, size_t, uint32_t, uint32_t, uint32_t,
    const uint8_t *, size_t, const uint8_t *, size_t, const uint8_t *, size_t, void *);
typedef void (*reply_callback)(const uint8_t *, size_t, void *);

/* Signatures mirror crates/quickgui-host/src/capi.rs. Kept private to this SDK. */
#define HOST_FUNCTIONS(X) \
    X(uint32_t, protocol_version, (void)) \
    X(int, run_host, (void)) \
    X(void, set_event_callback, (event_callback, void *)) \
    X(void, clear_event_callback, (event_callback, void *)) \
    X(uint32_t, create_app, (const uint8_t *, size_t)) \
    X(int, prepare_app, (uint32_t, uint32_t)) \
    X(uint32_t, allocate_window, (void)) \
    X(int, create_window, (uint32_t, uint32_t, const uint8_t *, size_t, const uint8_t *, size_t)) \
    X(int, create_system_popover, (uint32_t, uint32_t, uint32_t, uint32_t, const uint8_t *, size_t, const uint8_t *, size_t)) \
    X(int, create_embedded_view, (uint32_t, uint32_t, uint32_t, uint8_t, uint8_t, const uint8_t *, size_t, const uint8_t *, size_t)) \
    X(int, apply_batch, (uint32_t, uint32_t, const uint8_t *, size_t)) \
    X(int, close_window, (uint32_t, uint32_t)) \
    X(int, focus_node, (uint32_t, uint32_t, uint32_t)) \
    X(int, show_dialog, (uint32_t, uint32_t, uint32_t, uint32_t, const uint8_t *, size_t)) \
    X(int, command, (uint32_t, uint32_t, const uint8_t *, size_t)) \
    X(int, invoke, (uint32_t, const uint8_t *, size_t, const uint8_t *, size_t)) \
    X(int, call, (const uint8_t *, size_t, const uint8_t *, size_t, reply_callback, void *)) \
    X(int, destroy_app, (uint32_t)) \
    X(int, register_extension_versioned, (const void *, const uint8_t *, size_t, const uint8_t *, size_t)) \
    X(void, abort, (const char *))

#define FIELD(result, name, args) result (*name) args;
static struct { HOST_FUNCTIONS(FIELD) } host;
#undef FIELD
static void *host_library;
static char bridge_error[1024];
static int has_run;

static moonbit_bytes_t copied_bytes(const void *data, size_t length) {
    moonbit_bytes_t bytes = moonbit_make_bytes_raw((int32_t)length);
    if (length) memcpy(bytes, data, length);
    return bytes;
}

static char *text_bytes(moonbit_bytes_t bytes) {
    size_t length = Moonbit_array_length(bytes);
    if (length > 64 * 1024 || memchr(bytes, 0, length)) return NULL;
    char *text = malloc(length + 1);
    if (text) { memcpy(text, bytes, length); text[length] = 0; }
    return text;
}

static void *open_library(const char *path) {
#ifdef _WIN32
    wchar_t wide[32768];
    if (!MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, path, -1, wide, 32768)) return NULL;
    return (void *)LoadLibraryW(wide);
#else
    return dlopen(path, RTLD_NOW | RTLD_LOCAL);
#endif
}

static void *symbol(void *library, const char *name) {
#ifdef _WIN32
    return (void *)GetProcAddress((HMODULE)library, name);
#else
    return dlsym(library, name);
#endif
}

static int executable_directory(char *path, size_t capacity) {
#ifdef _WIN32
    wchar_t wide[32768];
    DWORD length = GetModuleFileNameW(NULL, wide, 32768);
    if (!length || length >= 32768 || !WideCharToMultiByte(CP_UTF8, 0, wide, -1, path, (int)capacity, NULL, NULL)) return 0;
#elif defined(__APPLE__)
    uint32_t size = (uint32_t)capacity;
    if (_NSGetExecutablePath(path, &size)) return 0;
#else
    ssize_t length = readlink("/proc/self/exe", path, capacity - 1);
    if (length <= 0 || (size_t)length >= capacity - 1) return 0;
    path[length] = 0;
#endif
    char *slash = strrchr(path, '/');
#ifdef _WIN32
    char *backslash = strrchr(path, '\\');
    if (!slash || (backslash && backslash > slash)) slash = backslash;
#endif
    if (!slash) return 0;
    *slash = 0;
    return 1;
}

static void *load_beside_executable(const char *filename, const char *directory_override) {
    char directory[32768], path[32768];
    if (directory_override && directory_override[0]) {
        int n = snprintf(path, sizeof(path), "%s/%s", directory_override, filename);
        if (n > 0 && (size_t)n < sizeof(path)) return open_library(path);
        return NULL;
    }
    if (!executable_directory(directory, sizeof(directory))) return NULL;
#ifdef __APPLE__
    int n = snprintf(path, sizeof(path), "%s/../Frameworks/%s", directory, filename);
    void *library = n > 0 && (size_t)n < sizeof(path) ? open_library(path) : NULL;
    if (library) return library;
#endif
    int written = snprintf(path, sizeof(path), "%s/%s", directory, filename);
    return written > 0 && (size_t)written < sizeof(path) ? open_library(path) : NULL;
}

int32_t qgm_load(int32_t protocol) {
    if (host_library) return host.protocol_version() == (uint32_t)protocol ? 0 : -1;
    const char *override = getenv("QUICKGUI_LIBRARY");
    if (!override || !override[0]) override = getenv("QUICKGUI_HOST_LIB");
#ifdef _WIN32
    const char *filename = "quickgui_host.dll";
#elif defined(__APPLE__)
    const char *filename = "libquickgui_host.dylib";
#else
    const char *filename = "libquickgui_host.so";
#endif
    void *library = override && override[0] ? open_library(override) : load_beside_executable(filename, NULL);
    if (!library) {
        snprintf(bridge_error, sizeof(bridge_error), "Could not load %s; use quickgui dev/build or set QUICKGUI_LIBRARY", filename);
        return -1;
    }
#define LOAD(result, name, args) do { \
    void *address = symbol(library, "quickgui_" #name); \
    if (!address) { snprintf(bridge_error, sizeof(bridge_error), "Missing native symbol quickgui_%s", #name); return -1; } \
    memcpy(&host.name, &address, sizeof(host.name)); \
} while (0);
    HOST_FUNCTIONS(LOAD)
#undef LOAD
    if (host.protocol_version() != (uint32_t)protocol) {
        snprintf(bridge_error, sizeof(bridge_error), "QuickGUI protocol mismatch: MoonBit=%d native=%u", protocol, host.protocol_version());
        return -1;
    }
    host_library = library; /* Never unload code retained by platform/service callbacks. */
    return 0;
}

moonbit_bytes_t qgm_error(void) { return copied_bytes(bridge_error, strlen(bridge_error)); }

/* One allocation per event, bounded both by count and total retained bytes. */
typedef struct queued_event {
    struct queued_event *next;
    uint32_t window, target, flags;
    size_t lengths[4], allocation;
    uint8_t bytes[];
} queued_event;
static queued_event *head, *tail, *current;
static size_t queued_count, queued_bytes;
static int queue_closed, queue_failed;
#define MAX_QUEUE_COUNT 8192
#define MAX_QUEUE_BYTES (64 * 1024 * 1024)

static void receive_event(const uint8_t *kind, size_t kind_len, uint32_t window, uint32_t target, uint32_t flags,
    const uint8_t *value, size_t value_len, const uint8_t *extra, size_t extra_len,
    const uint8_t *data, size_t data_len, void *context) {
    (void)context;
    const uint8_t *spans[4] = {kind, value, extra, data};
    size_t lengths[4] = {kind_len, value_len, extra_len, data_len};
    size_t allocation = sizeof(queued_event);
    int valid = kind_len <= 128;
    for (int i = 0; i < 4; i++) {
        if (lengths[i] > MAX_QUEUE_BYTES - allocation || (lengths[i] && !spans[i])) { valid = 0; break; }
        allocation += lengths[i];
    }
    lock_queue();
    if (queue_closed || queue_failed) { unlock_queue(); return; }
    if (!valid || queued_count >= MAX_QUEUE_COUNT || allocation > MAX_QUEUE_BYTES - queued_bytes) {
        queue_failed = 1;
        wake_queue();
        unlock_queue();
        return;
    }
    queued_event *event = malloc(allocation);
    if (!event) { queue_failed = 1; wake_queue(); unlock_queue(); return; }
    event->next = NULL; event->window = window; event->target = target; event->flags = flags;
    event->allocation = allocation;
    memcpy(event->lengths, lengths, sizeof(lengths));
    size_t offset = 0;
    for (int i = 0; i < 4; i++) {
        if (lengths[i]) memcpy(event->bytes + offset, spans[i], lengths[i]);
        offset += lengths[i];
    }
    if (tail) tail->next = event; else head = event;
    tail = event;
    queued_count++; queued_bytes += allocation;
    wake_queue();
    unlock_queue();
}

int32_t qgm_next_event(int32_t blocking) {
    free(current); current = NULL;
    lock_queue();
    while (blocking && !head && !queue_closed && !queue_failed) wait_queue();
    if (queue_failed) {
        unlock_queue();
        snprintf(bridge_error, sizeof(bridge_error), "QuickGUI MoonBit event queue exceeded its capacity");
        host.abort(bridge_error);
        return -1;
    }
    if (head) {
        current = head; head = head->next;
        if (!head) tail = NULL;
        queued_count--; queued_bytes -= current->allocation;
    }
    int result = current ? 1 : (queue_closed ? -2 : 0);
    unlock_queue();
    return result;
}

int32_t qgm_event_number(int32_t field) {
    if (!current) return 0;
    return (int32_t)(field == 0 ? current->window : field == 1 ? current->target : current->flags);
}

moonbit_bytes_t qgm_event_bytes(int32_t field) {
    if (!current || field < 0 || field > 3) return copied_bytes(NULL, 0);
    size_t offset = 0;
    for (int i = 0; i < field; i++) offset += current->lengths[i];
    return copied_bytes(current->bytes + offset, current->lengths[field]);
}

static void (*ui_entry)(void);
#ifdef _WIN32
static unsigned __stdcall ui_thread(void *context) {
#else
static void *ui_thread(void *context) {
#endif
    (void)context;
    ui_entry();
    free(current); current = NULL;
#ifdef _WIN32
    return 0;
#else
    return NULL;
#endif
}

int32_t qgm_run(void (*entry)(void)) {
    if (!host_library || has_run) {
        snprintf(bridge_error, sizeof(bridge_error), "Load QuickGUI first; native.run may only be called once");
        return -1;
    }
    has_run = 1;
    ui_entry = entry;
#ifdef _WIN32
    InitializeCriticalSection(&queue_lock);
    InitializeConditionVariable(&queue_wake);
#endif
    host.set_event_callback(receive_event, NULL);
#ifdef _WIN32
    HANDLE thread = (HANDLE)_beginthreadex(NULL, 0, ui_thread, NULL, 0, NULL);
    if (!thread) {
#else
    pthread_t thread;
    if (pthread_create(&thread, NULL, ui_thread, NULL)) {
#endif
        host.clear_event_callback(receive_event, NULL);
        snprintf(bridge_error, sizeof(bridge_error), "Could not start MoonBit UI thread");
        return -1;
    }
    int code = host.run_host();
    host.clear_event_callback(receive_event, NULL);
    lock_queue(); queue_closed = 1; wake_queue(); unlock_queue();
#ifdef _WIN32
    WaitForSingleObject(thread, INFINITE); CloseHandle(thread);
#else
    pthread_join(thread, NULL);
#endif
    lock_queue();
    while (head) { queued_event *next = head->next; free(head); head = next; }
    tail = NULL; queued_bytes = 0; queued_count = 0;
    unlock_queue();
    return code;
}

int32_t qgm_create_app(moonbit_bytes_t json) { return (int32_t)host.create_app(json, Moonbit_array_length(json)); }
int32_t qgm_prepare_app(int32_t app) { return host.prepare_app((uint32_t)app, 1); }
int32_t qgm_allocate_window(void) { return (int32_t)host.allocate_window(); }
int32_t qgm_create_window(int32_t app, int32_t window, moonbit_bytes_t json, moonbit_bytes_t batch) {
    return host.create_window(app, window, json, Moonbit_array_length(json), batch, Moonbit_array_length(batch));
}

int32_t qgm_create_child(int32_t app, int32_t window, int32_t parent, int32_t anchor,
    int32_t mode, moonbit_bytes_t json, moonbit_bytes_t batch) {
    if (mode == 0)
        return host.create_system_popover(app, window, parent, anchor, json, Moonbit_array_length(json), batch, Moonbit_array_length(batch));
    return host.create_embedded_view(app, window, parent, (mode & 2) != 0, (mode & 4) != 0,
        json, Moonbit_array_length(json), batch, Moonbit_array_length(batch));
}
int32_t qgm_apply_batch(int32_t app, int32_t window, moonbit_bytes_t batch) {
    return host.apply_batch(app, window, batch, Moonbit_array_length(batch));
}
int32_t qgm_close_window(int32_t app, int32_t window) { return host.close_window(app, window); }
int32_t qgm_focus_node(int32_t app, int32_t window, int32_t node) { return host.focus_node(app, window, node); }
int32_t qgm_command(int32_t app, int32_t request, moonbit_bytes_t json) {
    return host.command(app, request, json, Moonbit_array_length(json));
}
int32_t qgm_invoke(int32_t request, moonbit_bytes_t method, moonbit_bytes_t params) {
    return host.invoke(request, method, Moonbit_array_length(method), params, Moonbit_array_length(params));
}
int32_t qgm_dialog(int32_t app, int32_t window, int32_t request, int32_t kind, moonbit_bytes_t json) {
    return host.show_dialog(app, window, request, kind, json, Moonbit_array_length(json));
}
void qgm_abort(moonbit_bytes_t message) {
    char *text = text_bytes(message);
    host.abort(text ? text : "QuickGUI MoonBit application failed");
    free(text);
}

typedef struct { uint8_t *data; size_t length; int called; } sync_reply;
static void copy_reply(const uint8_t *data, size_t length, void *context) {
    sync_reply *reply = context;
    if (reply->called || length > MAX_QUEUE_BYTES) return;
    reply->called = 1;
    reply->data = malloc(length ? length : 1);
    if (reply->data) { memcpy(reply->data, data, length); reply->length = length; }
}
moonbit_bytes_t qgm_call(moonbit_bytes_t method, moonbit_bytes_t params) {
    sync_reply reply = {0};
    int code = host.call(method, Moonbit_array_length(method), params, Moonbit_array_length(params), copy_reply, &reply);
    moonbit_bytes_t bytes = code == 0 && reply.called && reply.data
        ? copied_bytes(reply.data, reply.length) : copied_bytes(NULL, 0);
    free(reply.data);
    return bytes;
}

int32_t qgm_load_extension(moonbit_bytes_t name_bytes, moonbit_bytes_t version_bytes) {
    char *name = text_bytes(name_bytes), *version = text_bytes(version_bytes);
    if (!name || !version || !name[0] || strlen(name) > 64 || strlen(version) > 64) {
        snprintf(bridge_error, sizeof(bridge_error), "Invalid extension name or version");
        free(name); free(version); return -1;
    }
    char stem[80] = "quickgui_";
    size_t n = strlen(name);
    for (size_t i = 0; i < n; i++) {
        char c = name[i];
        if (!((c >= 'a' && c <= 'z') || (i && c >= '0' && c <= '9') || (i && c == '-'))) {
            snprintf(bridge_error, sizeof(bridge_error), "Invalid extension name");
            free(name); free(version); return -1;
        }
        stem[9 + i] = c == '-' ? '_' : c;
    }
    stem[9 + n] = 0;
    char filename[100];
#ifdef _WIN32
    snprintf(filename, sizeof(filename), "%s.dll", stem);
#elif defined(__APPLE__)
    snprintf(filename, sizeof(filename), "lib%s.dylib", stem);
#else
    snprintf(filename, sizeof(filename), "lib%s.so", stem);
#endif
    void *library = load_beside_executable(filename, getenv("QUICKGUI_EXTENSION_DIR"));
    void *address = library ? symbol(library, "quickgui_extension_v1") : NULL;
    const void *(*descriptor)(void) = NULL;
    if (address) memcpy(&descriptor, &address, sizeof(descriptor));
    int code = descriptor ? host.register_extension_versioned(descriptor(), (uint8_t *)name, n,
        (uint8_t *)version, strlen(version)) : -1;
    if (code) snprintf(bridge_error, sizeof(bridge_error), "Could not load matching extension %s@%s", name, version);
    free(name); free(version);
    return code;
}

/* Metadata is a packaged resource, not a development-runtime dependency. */
moonbit_bytes_t qgm_metadata(void) {
    char directory[32768], path[32768];
    if (!executable_directory(directory, sizeof(directory))) return copied_bytes(NULL, 0);
#ifdef __APPLE__
    int n = snprintf(path, sizeof(path), "%s/../Resources/quickgui-moonbit.json", directory);
#else
    int n = snprintf(path, sizeof(path), "%s/quickgui-moonbit.json", directory);
#endif
    if (n <= 0 || (size_t)n >= sizeof(path)) return copied_bytes(NULL, 0);
    FILE *file = fopen(path, "rb");
    if (!file) return copied_bytes(NULL, 0);
    if (fseek(file, 0, SEEK_END) || ftell(file) < 0 || ftell(file) > 1024 * 1024) { fclose(file); return copied_bytes(NULL, 0); }
    size_t length = (size_t)ftell(file);
    rewind(file);
    moonbit_bytes_t data = moonbit_make_bytes_raw((int32_t)length);
    if (fread(data, 1, length, file) != length) { fclose(file); moonbit_decref(data); return copied_bytes(NULL, 0); }
    fclose(file);
    return data;
}

moonbit_bytes_t qgm_resource_directory(void) {
    char directory[32768], path[32768];
    if (!executable_directory(directory, sizeof(directory))) return copied_bytes(NULL, 0);
#ifdef __APPLE__
    size_t length = strlen(directory);
    if (length >= 7 && !strcmp(directory + length - 7, "/MacOS")) {
        int n = snprintf(path, sizeof(path), "%s/../Resources", directory);
        if (n > 0 && (size_t)n < sizeof(path)) return copied_bytes(path, (size_t)n);
    }
#else
    (void)path;
#endif
    return copied_bytes(directory, strlen(directory));
}
