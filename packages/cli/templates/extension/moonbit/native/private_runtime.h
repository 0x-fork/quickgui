/* Force-include before the toolchain sources: use a private runtime even on
 * Windows, where MOONBIT_BUILD_RUNTIME normally dllexports its implementation.
 * This avoids sharing the application's allocation/layout tables with a plugin. */
#include "moonbit.h"
#undef MOONBIT_EXPORT
#define MOONBIT_EXPORT
#undef MOONBIT_FFI_EXPORT
#define MOONBIT_FFI_EXPORT

#ifdef _MSC_VER
#define _Noreturn __declspec(noreturn)
#endif
