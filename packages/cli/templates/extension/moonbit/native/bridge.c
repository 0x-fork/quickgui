#include "quickgui_extension.h"
#include "moonbit.h"
#include "target/build_info.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
static volatile LONG gate;
static unsigned gate_or(unsigned bits) { return (unsigned)InterlockedOr(&gate, (LONG)bits); }
static unsigned gate_and(unsigned bits) { return (unsigned)InterlockedAnd(&gate, (LONG)bits); }
static int gate_swap(unsigned from, unsigned to) {
    return (unsigned)InterlockedCompareExchange(&gate, (LONG)to, (LONG)from) == from;
}
#else
#include <stdatomic.h>
static atomic_uint gate;
static unsigned gate_or(unsigned bits) { return atomic_fetch_or(&gate, bits); }
static unsigned gate_and(unsigned bits) { return atomic_fetch_and(&gate, bits); }
static int gate_swap(unsigned from, unsigned to) {
    return atomic_compare_exchange_strong(&gate, &from, to);
}
#endif

#define QGE_ACTIVE 1u
#define QGE_SHUTDOWN_PENDING 2u

/* Registration is immutable. Invoke normally runs on the frontend UI worker,
 * but the host shuts down services on its main thread. The nonblocking gate
 * gives this private runtime one caller at a time, including initialization. */
extern void moonbit_init(void);
extern void moonbit_runtime_init(int argc, char **argv);
extern void qge_invoke(void *context);
extern void qge_shutdown(void);
static int initialized;

static void finish(void) {
    for (;;) {
        if ((gate_and(~QGE_SHUTDOWN_PENDING) & QGE_SHUTDOWN_PENDING) && initialized)
            qge_shutdown();
        /* If shutdown raced this release, consume its pending bit before leaving.
         * If it arrives afterwards, shutdown acquires the idle gate itself. */
        if (gate_swap(QGE_ACTIVE, 0)) return;
    }
}

typedef struct Request {
    uint32_t id;
    QuickGuiBytes operation;
    QuickGuiBytes params;
    QuickGuiServiceSink sink;
    int replied;
} Request;

uint32_t qge_request_id(void *context) {
    return ((Request *)context)->id;
}

moonbit_bytes_t qge_request_bytes(void *context, int32_t field) {
    const Request *request = context;
    QuickGuiBytes bytes = field == 0 ? request->operation : request->params;
    moonbit_bytes_t copy = moonbit_make_bytes_raw((int32_t)bytes.len);
    if (bytes.len) memcpy(copy, bytes.data, bytes.len);
    return copy;
}

static void error(QuickGuiServiceSink sink, const char *message) {
    sink.emit(sink.context, QUICKGUI_EXTENSION_ERROR,
              (QuickGuiBytes){(const uint8_t *)message, strlen(message)});
}

/* payload is borrowed: MoonBit releases it after this callback returns. The
 * host copies the reply before emit returns. Never pass managed objects to it. */
void qge_reply(void *context, int32_t kind, moonbit_bytes_t payload) {
    Request *request = context;
    if (request->replied) return;
    request->replied = 1;
    size_t len = (size_t)Moonbit_array_length(payload);
    if (len > QUICKGUI_EXTENSION_MAX_PAYLOAD) {
        error(request->sink, "oversized extension response");
    } else {
        request->sink.emit(request->sink.context, (uint32_t)kind,
                           (QuickGuiBytes){payload, len});
    }
}

static void invoke(uint32_t id, QuickGuiBytes operation, QuickGuiBytes params,
                   QuickGuiServiceSink sink) {
    if (!operation.data || !operation.len || operation.len > 64 ||
        (params.len && !params.data) || params.len > QUICKGUI_EXTENSION_MAX_PAYLOAD) {
        error(sink, "Invalid method or oversized payload");
    } else if (!gate_swap(0, QGE_ACTIVE)) {
        error(sink, "MoonBit service is busy");
    } else {
        if (!initialized) {
            moonbit_runtime_init(0, NULL);
            moonbit_init();
            initialized = 1;
        }
        Request request = {id, operation, params, sink, 0};
        qge_invoke(&request);
        if (!request.replied) error(sink, "MoonBit service did not reply");
        finish();
    }
    sink.release(sink.context);
}

static void shutdown(void) {
    gate_or(QGE_SHUTDOWN_PENDING);
    if (gate_swap(QGE_SHUTDOWN_PENDING, QGE_ACTIVE | QGE_SHUTDOWN_PENDING)) finish();
}

/* A MoonBit abort is a programming error and terminates the process. Return Err
 * from handle for recoverable service failures. No backtrace runtime is needed. */
_Noreturn void moonbit_panic(void) {
    fputs("QuickGUI MoonBit extension aborted\n", stderr);
    abort();
}

static const QuickGuiServiceApi api = {invoke, shutdown};
static const QuickGuiExtension descriptor = {
    QUICKGUI_EXTENSION_ABI_V1, sizeof(QuickGuiExtension),
    QUICKGUI_EXTENSION_SERVICE, sizeof(QuickGuiServiceApi),
    {(const uint8_t *)QGE_NAME, sizeof(QGE_NAME) - 1},
    {(const uint8_t *)QGE_VERSION, sizeof(QGE_VERSION) - 1},
    &api,
};

QUICKGUI_EXTENSION_EXPORT const QuickGuiExtension *quickgui_extension_v1(void) {
    return &descriptor;
}
