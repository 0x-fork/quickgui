#include "quickgui_extension.h"
#include <string.h>

static void invoke(uint32_t request, QuickGuiBytes method, QuickGuiBytes params,
                   QuickGuiServiceSink sink) {
    (void)request;
    if (method.len == 4 && memcmp(method.data, "echo", 4) == 0 &&
        params.len <= QUICKGUI_EXTENSION_MAX_PAYLOAD) {
        /* This CPU-only operation is immediate; asynchronous providers copy the
         * inputs, queue work, and retain the sink until that work finishes. */
        sink.emit(sink.context, QUICKGUI_EXTENSION_REPLY, params);
    } else {
        static const uint8_t error[] = "Unknown method or oversized payload";
        sink.emit(sink.context, QUICKGUI_EXTENSION_ERROR,
                  (QuickGuiBytes){error, sizeof(error) - 1});
    }
    sink.release(sink.context);
}

static void shutdown_service(void) { /* No sessions or workers in this provider. */ }
static const QuickGuiServiceApi api = {invoke, shutdown_service};
static const QuickGuiExtension descriptor = {
    QUICKGUI_EXTENSION_ABI_V1,
    sizeof(QuickGuiExtension),
    QUICKGUI_EXTENSION_SERVICE,
    sizeof(QuickGuiServiceApi),
    {(const uint8_t *)"acme-echo", sizeof("acme-echo") - 1},
    {(const uint8_t *)"1.0.0", sizeof("1.0.0") - 1},
    &api,
};

QUICKGUI_EXTENSION_EXPORT const QuickGuiExtension *quickgui_extension_v1(void) {
    return &descriptor;
}
