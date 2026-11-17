#ifndef OPAL_TESTING_WORKSPACE_H
#define OPAL_TESTING_WORKSPACE_H
#include <stddef.h>
/* Ordinary application result ABI: error != NULL means value is unavailable. */
typedef struct { void *value; const char *error; } OpalTestingWorkspaceResult;
OpalTestingWorkspaceResult temporary_directory_create(void);
/* Valid child basenames only. The result is an ordinary owned managed string. */
char *temporary_directory_file(const char *directory, const char *filename);
/* Explicit removal only; successful removal consumes the mutable handle storage. */
OpalTestingWorkspaceResult temporary_directory_remove(char *directory);
#endif
