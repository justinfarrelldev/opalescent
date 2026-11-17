/* Explicit synchronous PTY driver. Children are stopped only by declared cleanup. */
#include "opal_portability.h"
#include "opal_testing_terminal.h"
#include "opal_testing_screen.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#if !defined(_WIN32)
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <util.h>
#else
#include <pty.h>
#endif
#endif
extern char *opal_string_alloc(uint64_t bytes);

static OpalTestingWorkspaceResult error(const char *message) {
    char *copy = malloc(strlen(message) + 1);
    if (!copy) abort();
    memcpy(copy, message, strlen(message) + 1);
    return (OpalTestingWorkspaceResult){NULL, copy};
}
static char *owned(const char *text) {
    char *copy = opal_string_alloc((uint64_t)strlen(text) + 1);
    if (!copy) abort();
    memcpy(copy, text, strlen(text) + 1);
    return copy;
}
const char *application_executable(void) {
    const char *path = getenv("OPAL_TEST_APPLICATION_EXECUTABLE");
    if (!path || !*path) abort();
    return path;
}

#if !defined(_WIN32)
typedef struct {
    pid_t child;
    int master, status;
    bool exited;
    OpalTestingScreen screen;
    char *transcript;
    size_t length;
} TerminalProcess;

static void pump(TerminalProcess *p) {
    char buffer[4096];
    for (;;) {
        ssize_t count = read(p->master, buffer, sizeof(buffer));
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) break;
        size_t size = (size_t)count;
        if (p->length > SIZE_MAX - size - 1) abort();
        char *text = realloc(p->transcript, p->length + size + 1);
        if (!text) abort();
        p->transcript = text;
        memcpy(text + p->length, buffer, size);
        p->length += size; text[p->length] = 0;
        opal_testing_screen_feed(&p->screen, buffer, size);
    }
}

OpalTestingWorkspaceResult terminal_process_launch(const char *executable, const char *argument,
    const char *directory, int64_t columns, int64_t rows) {
    if (!executable || !argument || !directory || columns < 1 || rows < 1 || columns > 512 || rows > 512)
        return error("CreateFailureError: invalid terminal launch configuration");
    TerminalProcess *p = calloc(1, sizeof(*p));
    if (!p) return error("AllocationFailureError: could not allocate terminal driver");
    if (!opal_testing_screen_init(&p->screen, (size_t)columns, (size_t)rows)) {
        free(p); return error("AllocationFailureError: could not allocate terminal screen");
    }
    struct winsize window = {(unsigned short)rows, (unsigned short)columns, 0, 0};
    p->child = forkpty(&p->master, NULL, NULL, &window);
    if (p->child == -1) {free(p->screen.cells); free(p); return error("CreateFailureError: could not launch pseudoterminal");}
    if (!p->child) {
        (void)unsetenv("OPAL_TERMINAL_FAKE_BACKEND");
        (void)unsetenv("OPAL_TERMINAL_FAKE_EVENTS");
        (void)unsetenv("OPAL_TERMINAL_FAKE_PAUSE_EVENTS");
        (void)setenv("TERM", "xterm-256color", 1);
        if (chdir(directory)) _exit(126);
        execl(executable, executable, argument, (char *)NULL);
        _exit(127);
    }
    int flags = fcntl(p->master, F_GETFL);
    if (flags < 0 || fcntl(p->master, F_SETFL, flags | O_NONBLOCK) < 0) {
        (void)kill(p->child, SIGKILL); while (waitpid(p->child, NULL, 0) < 0 && errno == EINTR) {}
        (void)close(p->master); free(p->screen.cells); free(p);
        return error("CreateFailureError: could not configure terminal transport");
    }
    return (OpalTestingWorkspaceResult){p, NULL};
}

OpalTestingWorkspaceResult terminal_process_send_text(void *process, const char *text) {
    TerminalProcess *p = process;
    if (!p || !text || p->exited) return error("WriteFailureError: terminal process is not writable");
    size_t length = strlen(text), offset = 0;
    while (offset < length) {
        ssize_t count = write(p->master, text + offset, length - offset);
        if (count > 0) {offset += (size_t)count; continue;}
        if (count < 0 && errno == EINTR) continue;
        if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            struct pollfd writable = {p->master, POLLOUT, 0};
            if (poll(&writable, 1, 1000) > 0) continue;
        }
        return error("WriteFailureError: could not send terminal input");
    }
    return (OpalTestingWorkspaceResult){NULL, NULL};
}

OpalTestingWorkspaceResult terminal_process_send_key(void *process, const char *key) {
    static const char *names[] = {"Enter", "Escape", "Backspace", "Tab", "Up", "Down", "Left", "Right", "Home", "End", "Delete"};
    static const char *values[] = {"\r", "\033", "\177", "\t", "\033[A", "\033[B", "\033[D", "\033[C", "\033[H", "\033[F", "\033[3~"};
    for (size_t i = 0; key && i < sizeof(names)/sizeof(names[0]); ++i)
        if (!strcmp(key, names[i])) return terminal_process_send_text(process, values[i]);
    return error("WriteFailureError: unknown terminal named key");
}
char *terminal_process_row(void *process, int64_t row) {
    TerminalProcess *p = process; if (!p || row < 0) abort();
    pump(p); return opal_testing_screen_row(&p->screen, (size_t)row);
}
int64_t terminal_process_cursor_line(void *process) {TerminalProcess *p = process; if (!p) abort(); pump(p); return (int64_t)p->screen.row;}
int64_t terminal_process_cursor_column(void *process) {TerminalProcess *p = process; if (!p) abort(); pump(p); return (int64_t)(p->screen.column < p->screen.columns ? p->screen.column : p->screen.columns - 1);}
bool terminal_process_exited(void *process) {
    TerminalProcess *p = process; if (!p) abort(); pump(p);
    if (!p->exited) {
        pid_t result; do {result = waitpid(p->child, &p->status, WNOHANG);} while (result < 0 && errno == EINTR);
        if (result == p->child) p->exited = true;
        else if (result < 0) abort();
    }
    return p->exited;
}
int64_t terminal_process_exit_code(void *process) {
    TerminalProcess *p = process;
    if (!terminal_process_exited(p)) return -1;
    return WIFEXITED(p->status) ? (int64_t)WEXITSTATUS(p->status) : (int64_t)(128 + WTERMSIG(p->status));
}
char *terminal_process_transcript(void *process) {
    TerminalProcess *p = process; if (!p) abort(); pump(p); return owned(p->transcript ? p->transcript : "");
}
OpalTestingWorkspaceResult terminal_process_stop(void *process) {
    TerminalProcess *p = process;
    if (!p) return error("DeleteFailureError: terminal process handle is unavailable");
    if (!terminal_process_exited(p)) {
        (void)kill(p->child, SIGTERM);
        for (unsigned i = 0; i < 100 && !terminal_process_exited(p); ++i) {
            struct pollfd pending = {p->master, POLLIN, 0}; (void)poll(&pending, 1, 10);
        }
        if (!p->exited) {
            (void)kill(p->child, SIGKILL);
            pid_t result; do {result = waitpid(p->child, &p->status, 0);} while (result < 0 && errno == EINTR);
            if (result != p->child) return error("DeleteFailureError: could not wait for terminal process");
            p->exited = true;
        }
    }
    (void)close(p->master); free(p->screen.cells); free(p->transcript); free(p);
    return (OpalTestingWorkspaceResult){NULL, NULL};
}
#else
OpalTestingWorkspaceResult terminal_process_launch(const char *executable, const char *argument, const char *directory, int64_t columns, int64_t rows) {
    (void)executable; (void)argument; (void)directory; (void)columns; (void)rows;
    return error("CreateFailureError: native PTY driver is unavailable on this platform");
}
OpalTestingWorkspaceResult terminal_process_send_text(void *p, const char *text) {(void)p; (void)text; return error("WriteFailureError: PTY unavailable");}
OpalTestingWorkspaceResult terminal_process_send_key(void *p, const char *key) {return terminal_process_send_text(p, key);}
char *terminal_process_row(void *p, int64_t row) {(void)p; (void)row; abort();}
int64_t terminal_process_cursor_line(void *p) {(void)p; abort();}
int64_t terminal_process_cursor_column(void *p) {(void)p; abort();}
bool terminal_process_exited(void *p) {(void)p; abort();}
int64_t terminal_process_exit_code(void *p) {(void)p; abort();}
char *terminal_process_transcript(void *p) {(void)p; return owned("");}
OpalTestingWorkspaceResult terminal_process_stop(void *p) {(void)p; return error("DeleteFailureError: PTY unavailable");}
#endif
