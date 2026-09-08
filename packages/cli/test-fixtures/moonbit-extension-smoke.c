/* Exercise independently compiled providers without a window or a core build. */
#include "quickgui_extension.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#include <pthread.h>
#endif

#define CHECK(condition) do { \
    if (!(condition)) { fprintf(stderr, "Extension check failed at line %d: %s\n", __LINE__, #condition); exit(1); } \
} while (0)

typedef struct Reply {
    uint32_t kind;
    size_t length;
    int emitted;
    int released;
    char bytes[QUICKGUI_EXTENSION_MAX_PAYLOAD + 1];
} Reply;

static void emit(void *context, uint32_t kind, QuickGuiBytes bytes) {
    Reply *reply = context;
    CHECK(!reply->emitted && !reply->released);
    CHECK(bytes.len <= QUICKGUI_EXTENSION_MAX_PAYLOAD);
    reply->emitted++;
    reply->kind = kind;
    reply->length = bytes.len;
    if (bytes.len) memcpy(reply->bytes, bytes.data, bytes.len);
    reply->bytes[bytes.len] = 0;
}

static void release(void *context) {
    Reply *reply = context;
    CHECK(reply->emitted == 1 && !reply->released);
    reply->released++;
}

static const QuickGuiServiceApi *load(const char *path, const char *name) {
#ifdef _WIN32
    HMODULE library = LoadLibraryA(path);
    CHECK(library);
    const QuickGuiExtension *(*entry)(void) = (void *)GetProcAddress(library, "quickgui_extension_v1");
    CHECK(GetProcAddress(library, "moonbit_init") == NULL);
    CHECK(GetProcAddress(library, "moonbit_layout_table") == NULL);
#else
    void *library = dlopen(path, RTLD_NOW | RTLD_GLOBAL);
    if (!library) fprintf(stderr, "%s\n", dlerror());
    CHECK(library);
    const QuickGuiExtension *(*entry)(void) = dlsym(library, "quickgui_extension_v1");
    CHECK(dlsym(library, "moonbit_init") == NULL);
    CHECK(dlsym(library, "moonbit_layout_table") == NULL);
#endif
    CHECK(entry);
    const QuickGuiExtension *descriptor = entry();
    CHECK(descriptor->abi_version == QUICKGUI_EXTENSION_ABI_V1);
    CHECK(descriptor->descriptor_size == sizeof(QuickGuiExtension));
    CHECK(descriptor->kind == QUICKGUI_EXTENSION_SERVICE);
    CHECK(descriptor->api_size == sizeof(QuickGuiServiceApi));
    CHECK(descriptor->name.len == strlen(name));
    CHECK(memcmp(descriptor->name.data, name, strlen(name)) == 0);
    const QuickGuiServiceApi *api = descriptor->api;
    CHECK(api && api->invoke && api->shutdown);
    return api;
}

static void call(const QuickGuiServiceApi *api, const char *operation,
                 QuickGuiBytes params, uint32_t kind, const char *expected) {
    Reply reply = {0};
    api->invoke(42, (QuickGuiBytes){(const uint8_t *)operation, strlen(operation)}, params,
                (QuickGuiServiceSink){&reply, emit, release});
    CHECK(reply.emitted == 1 && reply.released == 1);
    CHECK(reply.kind == kind && reply.length == strlen(expected));
    CHECK(strcmp(reply.bytes, expected) == 0);
}

/* Hold emit while the MoonBit handler is on the stack, then issue shutdown from
 * the other thread. A blocking shutdown would deadlock this deterministic test. */
static int entered, resume;
#ifdef _WIN32
static CRITICAL_SECTION mutex;
static CONDITION_VARIABLE changed;
static void lock(void) { EnterCriticalSection(&mutex); }
static void unlock(void) { LeaveCriticalSection(&mutex); }
static void wait_changed(void) { SleepConditionVariableCS(&changed, &mutex, INFINITE); }
static void signal_changed(void) { WakeAllConditionVariable(&changed); }
#else
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t changed = PTHREAD_COND_INITIALIZER;
static void lock(void) { pthread_mutex_lock(&mutex); }
static void unlock(void) { pthread_mutex_unlock(&mutex); }
static void wait_changed(void) { pthread_cond_wait(&changed, &mutex); }
static void signal_changed(void) { pthread_cond_broadcast(&changed); }
#endif

static void blocking_emit(void *context, uint32_t kind, QuickGuiBytes bytes) {
    emit(context, kind, bytes);
    lock();
    entered = 1;
    signal_changed();
    while (!resume) wait_changed();
    unlock();
}

#ifdef _WIN32
static DWORD WINAPI held_call(void *context) {
#else
static void *held_call(void *context) {
#endif
    const QuickGuiServiceApi *api = context;
    Reply reply = {0};
    api->invoke(42, (QuickGuiBytes){(const uint8_t *)"counter", 7},
                (QuickGuiBytes){(const uint8_t *)"null", 4},
                (QuickGuiServiceSink){&reply, blocking_emit, release});
    CHECK(reply.emitted == 1 && reply.released == 1);
    CHECK(reply.kind == QUICKGUI_EXTENSION_REPLY);
    return 0;
}

static void overlapping_shutdown(const QuickGuiServiceApi *api) {
#ifdef _WIN32
    InitializeCriticalSection(&mutex);
    InitializeConditionVariable(&changed);
    HANDLE thread = CreateThread(NULL, 0, held_call, (void *)api, 0, NULL);
    CHECK(thread);
#else
    pthread_t thread;
    CHECK(pthread_create(&thread, NULL, held_call, (void *)api) == 0);
#endif
    lock();
    while (!entered) wait_changed();
    unlock();
    api->shutdown();
    call(api, "counter", (QuickGuiBytes){(const uint8_t *)"null", 4},
         QUICKGUI_EXTENSION_ERROR, "MoonBit service is busy");
    lock();
    resume = 1;
    signal_changed();
    unlock();
#ifdef _WIN32
    CHECK(WaitForSingleObject(thread, INFINITE) == WAIT_OBJECT_0);
    CloseHandle(thread);
    DeleteCriticalSection(&mutex);
#else
    CHECK(pthread_join(thread, NULL) == 0);
#endif
    // The pending shutdown must run after the handler exits, resetting its state.
    call(api, "counter", (QuickGuiBytes){(const uint8_t *)"null", 4}, QUICKGUI_EXTENSION_REPLY, "1");
}

int main(int argc, char **argv) {
    CHECK(argc == 3);
    const QuickGuiServiceApi *a = load(argv[1], "moon-smoke-a");
    const QuickGuiServiceApi *b = load(argv[2], "moon-smoke-b");
    const char *json = "{ \"keep\": null, \"nested\": [true, \"MoonBit\"], \"n\": 900719925474099312345 }";
    QuickGuiBytes params = {(const uint8_t *)json, strlen(json)};
    // A shutdown before the first call must not initialize either runtime.
    a->shutdown();
    b->shutdown();
    for (int i = 0; i < 200; i++) {
        call(a, "echo", params, QUICKGUI_EXTENSION_REPLY, json);
        call(b, "echo", params, QUICKGUI_EXTENSION_REPLY, json);
    }
    call(a, "counter", params, QUICKGUI_EXTENSION_REPLY, "1");
    call(a, "counter", params, QUICKGUI_EXTENSION_REPLY, "2");
    call(b, "counter", params, QUICKGUI_EXTENSION_REPLY, "1");
    call(a, "request-id", params, QUICKGUI_EXTENSION_REPLY, "42");
    call(a, "missing", params, QUICKGUI_EXTENSION_ERROR, "Unknown method");
    call(a, "large-reply", params, QUICKGUI_EXTENSION_ERROR, "oversized extension response");
    call(a, "echo", (QuickGuiBytes){(const uint8_t *)json, QUICKGUI_EXTENSION_MAX_PAYLOAD + 1},
         QUICKGUI_EXTENSION_ERROR, "Invalid method or oversized payload");
    call(a, "echo", (QuickGuiBytes){NULL, 1},
         QUICKGUI_EXTENSION_ERROR, "Invalid method or oversized payload");
    a->shutdown();
    call(a, "counter", params, QUICKGUI_EXTENSION_REPLY, "1");
    call(b, "counter", params, QUICKGUI_EXTENSION_REPLY, "2");
    overlapping_shutdown(a);
    a->shutdown();
    b->shutdown();
    puts("MoonBit provider ABI, ownership, bounds, shutdown, and runtime isolation passed");
    return 0;
}
