#include "opal_portability.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef OPAL_FS_VOID_RESULT_TYPE_DEFINED
typedef struct {
    void* value;
    const char* error;
} FsVoidResult;
#define OPAL_FS_VOID_RESULT_TYPE_DEFINED 1
#endif

typedef struct OpalStdoutWriter {
    FILE* stream;
} OpalStdoutWriter;

typedef struct OpalStdoutTerminal {
    FILE* stream;
} OpalStdoutTerminal;

typedef struct OpalFrameClock {
    int32_t frame_duration_ms;
    int64_t next_deadline_ms;
} OpalFrameClock;

#ifndef OPAL_FRAME_CLOCK_RESULT_TYPE_DEFINED
typedef struct {
    OpalFrameClock* value;
    const char* error;
} FsFrameClockResult;
#define OPAL_FRAME_CLOCK_RESULT_TYPE_DEFINED 1
#endif

typedef struct OpalFrameClockNode {
    OpalFrameClock* clock;
    struct OpalFrameClockNode* next;
} OpalFrameClockNode;

static OpalFrameClockNode* OPAL_FRAME_CLOCKS = NULL;
static int OPAL_FRAME_CLOCKS_CLEANUP_REGISTERED = 0;

static OpalStdoutWriter OPAL_STDOUT_WRITER = {NULL};
static OpalStdoutTerminal OPAL_STDOUT_TERMINAL = {NULL};
static const char* OPAL_STDOUT_WRITE_FAILURE_ERROR = "WriteFailureError";
static const char* OPAL_STDOUT_FLUSH_FAILURE_ERROR = "FlushFailureError";
static const char* OPAL_STDOUT_SINK_CLOSED_ERROR = "SinkClosedError";
static const char* OPAL_TERMINAL_WRITE_FAILURE_ERROR = "TerminalWriteFailureError";
static const char* OPAL_INVALID_CURSOR_POSITION_ERROR = "InvalidCursorPositionError";
static const char* OPAL_INVALID_DURATION_ERROR = "InvalidDurationError";
static const char* OPAL_INVALID_FRAME_RATE_ERROR = "InvalidFrameRateError";

static FsVoidResult stdout_void_success(void) {
    FsVoidResult result = {NULL, NULL};
    return result;
}

static FsVoidResult stdout_void_error(const char* error) {
    FsVoidResult result = {NULL, error};
    return result;
}

static FsFrameClockResult frame_clock_result_success(OpalFrameClock* clock) {
    FsFrameClockResult result = {clock, NULL};
    return result;
}

static FsFrameClockResult frame_clock_result_error(const char* error) {
    FsFrameClockResult result = {NULL, error};
    return result;
}

static void opal_frame_clock_cleanup_all(void) {
    OpalFrameClockNode* node = OPAL_FRAME_CLOCKS;
    while (node) {
        OpalFrameClockNode* next = node->next;
        free(node->clock);
        free(node);
        node = next;
    }
    OPAL_FRAME_CLOCKS = NULL;
}

static void opal_frame_clock_register_for_cleanup(OpalFrameClock* clock) {
    OpalFrameClockNode* node = (OpalFrameClockNode*)malloc(sizeof(OpalFrameClockNode));
    if (!node) {
        free(clock);
        fprintf(stderr, "Runtime error: out of memory\n");
        exit(1);
    }
    node->clock = clock;
    node->next = OPAL_FRAME_CLOCKS;
    OPAL_FRAME_CLOCKS = node;
    if (!OPAL_FRAME_CLOCKS_CLEANUP_REGISTERED) {
        if (atexit(opal_frame_clock_cleanup_all) != 0) {
            free(clock);
            fprintf(stderr, "Runtime error: failed to register frame clock cleanup\n");
            exit(1);
        }
        OPAL_FRAME_CLOCKS_CLEANUP_REGISTERED = 1;
    }
}

static const char* stdout_error_from_errno(const char* default_error) {
    if (errno == EPIPE) {
        return OPAL_STDOUT_SINK_CLOSED_ERROR;
    }
    return default_error;
}

static FILE* stdout_stream(void) {
    return stdout;
}

static FsVoidResult stdout_write_stream(FILE* stream, const char* value) {
    const char* safe_value = value ? value : "";
    size_t value_length = strlen(safe_value);

    if (stream == NULL) {
        return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
    }

    clearerr(stream);
    errno = 0;
    if (value_length > 0) {
        size_t written = fwrite(safe_value, 1, value_length, stream);
        if (written != value_length || ferror(stream)) {
            return stdout_void_error(stdout_error_from_errno(OPAL_STDOUT_WRITE_FAILURE_ERROR));
        }
    }

    return stdout_void_success();
}

static FsVoidResult stdout_flush_stream(FILE* stream) {
    if (stream == NULL) {
        return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
    }

    clearerr(stream);
    errno = 0;
    if (fflush(stream) != 0) {
        return stdout_void_error(stdout_error_from_errno(OPAL_STDOUT_FLUSH_FAILURE_ERROR));
    }

    return stdout_void_success();
}

static FsVoidResult terminal_write_stream(FILE* stream, const char* value) {
    const char* safe_value = value ? value : "";
    size_t value_length = strlen(safe_value);

    if (stream == NULL) {
        return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
    }

    clearerr(stream);
    errno = 0;
    if (value_length > 0) {
        size_t written = fwrite(safe_value, 1, value_length, stream);
        if (written != value_length || ferror(stream)) {
            return stdout_void_error(stdout_error_from_errno(OPAL_TERMINAL_WRITE_FAILURE_ERROR));
        }
    }

    return stdout_void_success();
}

static FILE* terminal_stream(OpalStdoutTerminal* terminal) {
    return terminal ? terminal->stream : NULL;
}

static int8_t terminal_supports_ansi_stream(FILE* stream) {
    if (stream == NULL) {
        return 0;
    }
#if OPAL_WINDOWS
    return 0;
#else
    return isatty(fileno(stream)) ? 1 : 0;
#endif
}

static FsVoidResult terminal_invalid_cursor_position_error(void) {
    return stdout_void_error(OPAL_INVALID_CURSOR_POSITION_ERROR);
}

static FsVoidResult terminal_write_cursor_move(FILE* stream, int32_t row, int32_t column) {
    char escape_sequence[64];
    int written = 0;

    if (row < 0 || column < 0) {
        return terminal_invalid_cursor_position_error();
    }

    written = snprintf(
        escape_sequence,
        sizeof(escape_sequence),
        "\x1b[%" PRId32 ";%" PRId32 "H",
        row + 1,
        column + 1
    );
    if (written < 0 || (size_t)written >= sizeof(escape_sequence)) {
        return stdout_void_error(OPAL_TERMINAL_WRITE_FAILURE_ERROR);
    }

    return terminal_write_stream(stream, escape_sequence);
}

static FsVoidResult terminal_write_rows(FILE* stream, const char** rows, int64_t count) {
    int64_t index = 0;

    if (stream == NULL) {
        return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
    }
    if (count <= 0 || rows == NULL) {
        return stdout_void_success();
    }

    for (index = 0; index < count; index++) {
        FsVoidResult write_result = terminal_write_stream(stream, rows[index]);
        if (write_result.error != NULL) {
            return write_result;
        }
        if (index + 1 < count) {
            write_result = terminal_write_stream(stream, "\n");
            if (write_result.error != NULL) {
                return write_result;
            }
        }
    }

    return stdout_void_success();
}

static char* duplicate_without_trailing_newline(const char* source) {
    char* raw = opal_strdup(source);
    if (!raw) {
        fprintf(stderr, "Runtime error: out of memory\n");
        exit(1);
    }
    size_t len = strlen(raw);
    if (len > 0 && raw[len - 1] == '\n') {
        raw[len - 1] = '\0';
    }

    size_t trimmed_len = strlen(raw);
    char* out = (char*)malloc(trimmed_len + 1);
    if (!out) {
        fprintf(stderr, "Runtime error: out of memory\n");
        exit(1);
    }
    memcpy(out, raw, trimmed_len + 1);
    free(raw);
    return out;
}

char* take_input(void) {
    char* line = NULL;
    size_t len = 0;
    ssize_t read = opal_getline(&line, &len, stdin);

    if (read == -1) {
        if (line != NULL) {
            free(line);
        }
        return duplicate_without_trailing_newline("");
    }

    char* result = duplicate_without_trailing_newline(line);
    free(line);
    return result;
}

void print_string(const char* s) {
    puts(s);
}

FsVoidResult print_text_sync(const char* value) {
    return stdout_write_stream(stdout_stream(), value);
}

FsVoidResult flush_standard_output_sync(void) {
    return stdout_flush_stream(stdout_stream());
}

FsVoidResult sleep_ms_sync(int32_t milliseconds) {
    if (opal_sleep_ms(milliseconds) != 0) {
        return stdout_void_error(OPAL_INVALID_DURATION_ERROR);
    }

    return stdout_void_success();
}

FsFrameClockResult frame_clock_new(int32_t frames_per_second) {
    int64_t now_ms = 0;
    OpalFrameClock* clock = NULL;

    if (frames_per_second <= 0) {
        return frame_clock_result_error(OPAL_INVALID_FRAME_RATE_ERROR);
    }
    if (opal_monotonic_time_ms(&now_ms) != 0) {
        return frame_clock_result_error(OPAL_INVALID_FRAME_RATE_ERROR);
    }

    clock = (OpalFrameClock*)malloc(sizeof(*clock));
    if (!clock) {
        return frame_clock_result_error(OPAL_INVALID_FRAME_RATE_ERROR);
    }

    clock->frame_duration_ms = 1000 / frames_per_second;
    if (clock->frame_duration_ms <= 0) {
        clock->frame_duration_ms = 1;
    }
    clock->next_deadline_ms = now_ms + clock->frame_duration_ms;
    opal_frame_clock_register_for_cleanup(clock);
    return frame_clock_result_success(clock);
}

FsVoidResult frame_clock_wait_next_sync(OpalFrameClock* clock) {
    int64_t now_ms = 0;
    int64_t sleep_for_ms = 0;

    if (clock == NULL || clock->frame_duration_ms <= 0) {
        return stdout_void_error(OPAL_INVALID_FRAME_RATE_ERROR);
    }
    if (opal_monotonic_time_ms(&now_ms) != 0) {
        return stdout_void_error(OPAL_INVALID_FRAME_RATE_ERROR);
    }

    sleep_for_ms = clock->next_deadline_ms - now_ms;
    if (sleep_for_ms > 0) {
        if (sleep_for_ms > INT32_MAX) {
            sleep_for_ms = INT32_MAX;
        }
        if (opal_sleep_ms((int32_t)sleep_for_ms) != 0) {
            return stdout_void_error(OPAL_INVALID_FRAME_RATE_ERROR);
        }
        if (opal_monotonic_time_ms(&now_ms) != 0) {
            return stdout_void_error(OPAL_INVALID_FRAME_RATE_ERROR);
        }
    }

    if (now_ms >= clock->next_deadline_ms) {
        clock->next_deadline_ms = now_ms + clock->frame_duration_ms;
    } else {
        clock->next_deadline_ms += clock->frame_duration_ms;
    }

    return stdout_void_success();
}

OpalStdoutWriter* stdout_writer(void) {
    OPAL_STDOUT_WRITER.stream = stdout_stream();
    return &OPAL_STDOUT_WRITER;
}

FsVoidResult writer_write_sync(OpalStdoutWriter* writer, const char* value) {
    return stdout_write_stream(writer ? writer->stream : NULL, value);
}

FsVoidResult writer_flush_sync(OpalStdoutWriter* writer) {
    return stdout_flush_stream(writer ? writer->stream : NULL);
}

OpalStdoutTerminal* stdout_terminal(void) {
    OPAL_STDOUT_TERMINAL.stream = stdout_stream();
    return &OPAL_STDOUT_TERMINAL;
}

int8_t terminal_supports_ansi(OpalStdoutTerminal* terminal) {
    return terminal_supports_ansi_stream(terminal_stream(terminal));
}

FsVoidResult terminal_clear_screen_on_sync(OpalStdoutTerminal* terminal) {
    return terminal_write_stream(terminal_stream(terminal), "\x1b[2J\x1b[H");
}

FsVoidResult terminal_move_cursor_on_sync(OpalStdoutTerminal* terminal, int32_t row, int32_t column) {
    return terminal_write_cursor_move(terminal_stream(terminal), row, column);
}

FsVoidResult terminal_draw_rows_sync(OpalStdoutTerminal* terminal, const char** rows, int64_t count) {
    return terminal_write_rows(terminal_stream(terminal), rows, count);
}

FsVoidResult terminal_clear_screen_sync(void) {
    return terminal_clear_screen_on_sync(stdout_terminal());
}

FsVoidResult terminal_move_cursor_sync(int32_t row, int32_t column) {
    return terminal_move_cursor_on_sync(stdout_terminal(), row, column);
}
