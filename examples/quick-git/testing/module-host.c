// Test-only host for the real Zig module. Background results are copied into a mutex-protected
// queue and drained on Bun's thread; no JavaScript callback is invoked from a native worker.
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef void (*Reply)(const unsigned char *, size_t, void *);
typedef void (*Completion)(uint32_t, const unsigned char *, size_t);
extern void quickgui_module_git_call(uint32_t, const unsigned char *, size_t, Reply, void *);
extern void quickgui_module_git_call_async(uint32_t, uint32_t, const unsigned char *, size_t);

typedef struct Result { struct Result *next; uint32_t id; size_t length; unsigned char data[]; } Result;
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static Result *head, *tail;

void quickgui_module_complete(uint32_t id, const unsigned char *data, size_t length) {
    Result *result = malloc(sizeof(Result) + length);
    if (!result) abort();
    result->next = NULL; result->id = id; result->length = length;
    memcpy(result->data, data, length);
    pthread_mutex_lock(&lock);
    if (tail) tail->next = result; else head = result;
    tail = result;
    pthread_mutex_unlock(&lock);
}

void test_call(uint32_t index, const unsigned char *args, size_t length, Reply reply) {
    quickgui_module_git_call(index, args, length, reply, NULL);
}
void test_call_async(uint32_t id, uint32_t index, const unsigned char *args, size_t length) {
    quickgui_module_git_call_async(id, index, args, length);
}
void test_drain(Completion complete) {
    pthread_mutex_lock(&lock);
    Result *result = head; head = tail = NULL;
    pthread_mutex_unlock(&lock);
    while (result) {
        Result *next = result->next;
        complete(result->id, result->data, result->length);
        free(result);
        result = next;
    }
}
