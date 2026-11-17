/* Explicit test-library operations. No registry, atexit hook or resource discovery. */
#include "opal_portability.h"
#include "opal_testing_workspace.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <unistd.h>
#endif
extern char *opal_string_alloc(uint64_t bytes);
extern OpalTestingWorkspaceResult delete_directory_recursive_sync(const char *path);

static OpalTestingWorkspaceResult failure(const char *message) {
    size_t size = strlen(message) + 1;
    char *error = malloc(size);
    if (!error) abort();
    memcpy(error, message, size);
    return (OpalTestingWorkspaceResult){NULL, error};
}

OpalTestingWorkspaceResult temporary_directory_create(void) {
    char *path;
#if defined(_WIN32)
    char root[MAX_PATH + 1], candidate[MAX_PATH + 1];
    DWORD length = GetTempPathA(MAX_PATH, root);
    if (!length || length > MAX_PATH || !GetTempFileNameA(root, "opl", 0, candidate))
        return failure("CreateFailureError: could not choose a temporary directory");
    if (!DeleteFileA(candidate) || !CreateDirectoryA(candidate, NULL))
        return failure("CreateFailureError: could not create a temporary directory");
    path = malloc(strlen(candidate) + 1);
    if (!path) {
        (void)RemoveDirectoryA(candidate);
        return failure("AllocationFailureError: could not allocate temporary directory handle");
    }
    memcpy(path, candidate, strlen(candidate) + 1);
#else
    const char *root = getenv("TMPDIR");
    if (!root || !*root) root = "/tmp";
    size_t length = strlen(root);
    const char suffix[] = "/opal-test-XXXXXX";
    if (length > SIZE_MAX - sizeof(suffix))
        return failure("AllocationFailureError: temporary directory pathname is too large");
    path = malloc(length + sizeof(suffix));
    if (!path) return failure("AllocationFailureError: could not allocate temporary directory handle");
    memcpy(path, root, length);
    memcpy(path + length, suffix, sizeof(suffix));
    if (!mkdtemp(path)) {
        free(path);
        return failure("CreateFailureError: could not create a temporary directory");
    }
#endif
    return (OpalTestingWorkspaceResult){path, NULL};
}

char *temporary_directory_file(const char *directory, const char *filename) {
    /* A nonfallible filename helper has a checked basename precondition. */
    if (!directory || !*directory || !filename || !*filename || !strcmp(filename, ".") ||
        !strcmp(filename, "..") || strchr(filename, '/') || strchr(filename, '\\')) abort();
    size_t length = strlen(directory), child = strlen(filename);
    if (child > SIZE_MAX - 2 || length > SIZE_MAX - child - 2) abort();
    size_t size = length + child + 2;
    char *result = opal_string_alloc((uint64_t)size);
    if (!result) abort();
    memcpy(result, directory, length);
    result[length] = '/';
    memcpy(result + length + 1, filename, child + 1);
    return result;
}

OpalTestingWorkspaceResult temporary_directory_remove(char *directory) {
    if (!directory || !*directory)
        return failure("InvalidPathError: temporary directory handle is unavailable");
    OpalTestingWorkspaceResult result = delete_directory_recursive_sync(directory);
    if (!result.error) free(directory);
    return result;
}
