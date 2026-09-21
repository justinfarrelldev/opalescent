#include "opal_portability.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef OPAL_FS_VOID_RESULT_TYPE_DEFINED
typedef struct {
  void *value;
  const char *error;
} FsVoidResult;
#define OPAL_FS_VOID_RESULT_TYPE_DEFINED 1
#endif

#ifndef OPAL_FS_STRING_RESULT_DEFINED
typedef struct {
  char *value;
  const char *error;
} FsStringResult;
#define OPAL_FS_STRING_RESULT_DEFINED 1
#endif

#ifndef OPAL_FS_HANDLE_RESULT_DEFINED
typedef struct {
  void *value;
  const char *error;
} FsHandleResult;
#define OPAL_FS_HANDLE_RESULT_DEFINED 1
#endif

#ifndef OPAL_FS_BOOLEAN_RESULT_DEFINED
typedef struct {
  int8_t value;
  const char *error;
} FsBooleanResult;
#define OPAL_FS_BOOLEAN_RESULT_DEFINED 1
#endif

typedef struct OpalStdoutWriter {
  FILE *stream;
  uint64_t lease_epoch;
} OpalStdoutWriter;

typedef struct OpalStdoutTerminal {
  FILE *stream;
  uint64_t lease_epoch;
} OpalStdoutTerminal;

typedef struct OpalStdoutWriterNode {
  OpalStdoutWriter *writer;
  struct OpalStdoutWriterNode *next;
} OpalStdoutWriterNode;

typedef struct OpalStdoutTerminalNode {
  OpalStdoutTerminal *terminal;
  struct OpalStdoutTerminalNode *next;
} OpalStdoutTerminalNode;

typedef struct OpalFrameClock {
  int32_t frame_duration_ms;
  int64_t next_deadline_ms;
} OpalFrameClock;

#ifndef OPAL_FRAME_CLOCK_RESULT_TYPE_DEFINED
typedef struct {
  OpalFrameClock *value;
  const char *error;
} FsFrameClockResult;
#define OPAL_FRAME_CLOCK_RESULT_TYPE_DEFINED 1
#endif

typedef struct OpalFrameClockNode {
  OpalFrameClock *clock;
  struct OpalFrameClockNode *next;
} OpalFrameClockNode;

static OpalFrameClockNode *OPAL_FRAME_CLOCKS = NULL;
static int OPAL_FRAME_CLOCKS_CLEANUP_REGISTERED = 0;

enum {
  OPAL_TERMINAL_FREE = 0,
  OPAL_TERMINAL_OPENING = 1,
  OPAL_TERMINAL_ACTIVE = 2,
  OPAL_TERMINAL_PAUSED = 3,
  OPAL_TERMINAL_RESTORE_PENDING = 4,
  OPAL_TERMINAL_FAILED_OPEN_RECOVERY = 5,
  OPAL_TERMINAL_FAILED_CLOSE_RECOVERY = 6
};

static int OPAL_TERMINAL_STATE = OPAL_TERMINAL_FREE;
static uint64_t OPAL_TERMINAL_LEASE_EPOCH = 0;
static OpalStdoutWriterNode *OPAL_STDOUT_WRITERS = NULL;
static OpalStdoutTerminalNode *OPAL_STDOUT_TERMINALS = NULL;
static int OPAL_STDOUT_HANDLES_CLEANUP_REGISTERED = 0;
static const char *OPAL_STDOUT_WRITE_FAILURE_ERROR = "WriteFailureError";
static const char *OPAL_STDOUT_FLUSH_FAILURE_ERROR = "FlushFailureError";
static const char *OPAL_STDOUT_SINK_CLOSED_ERROR = "SinkClosedError";
static const char *OPAL_TERMINAL_WRITE_FAILURE_ERROR =
    "TerminalWriteFailureError";
static const char *OPAL_WRITE_COORDINATOR_UNAVAILABLE_ERROR =
    "WriteFailureError";
static const char *OPAL_FLUSH_COORDINATOR_UNAVAILABLE_ERROR =
    "FlushFailureError";
static const char *OPAL_TERMINAL_COORDINATOR_UNAVAILABLE_ERROR =
    "TerminalWriteFailureError";
static const char *OPAL_INVALID_CURSOR_POSITION_ERROR =
    "InvalidCursorPositionError";
static const char *OPAL_INVALID_DURATION_ERROR = "InvalidDurationError";
static const char *OPAL_INVALID_FRAME_RATE_ERROR = "InvalidFrameRateError";
static const char *OPAL_TERMINAL_FAKE_BACKEND_REQUIRED_ERROR =
    "TerminalSessionOpenError: FakeBackendNotInjected";
static const char *OPAL_TERMINAL_FAKE_END_OF_INPUT_ERROR =
    "TerminalSessionReadError: EndOfInput";
static const char *OPAL_TERMINAL_SESSION_CLOSED_ERROR =
    "TerminalSessionStateError: SessionClosed";

typedef struct OpalTerminalSession {
  int closed;
  int fake_backend;
  int read_count;
} OpalTerminalSession;

static FsVoidResult stdout_void_success(void) {
  FsVoidResult result = {NULL, NULL};
  return result;
}

static FsVoidResult stdout_void_error(const char *error) {
  FsVoidResult result = {NULL, error};
  return result;
}

static FsFrameClockResult frame_clock_result_success(OpalFrameClock *clock) {
  FsFrameClockResult result = {clock, NULL};
  return result;
}

static FsFrameClockResult frame_clock_result_error(const char *error) {
  FsFrameClockResult result = {NULL, error};
  return result;
}

static void opal_frame_clock_cleanup_all(void) {
  OpalFrameClockNode *node = OPAL_FRAME_CLOCKS;
  while (node) {
    OpalFrameClockNode *next = node->next;
    free(node->clock);
    free(node);
    node = next;
  }
  OPAL_FRAME_CLOCKS = NULL;
}

static void opal_frame_clock_register_for_cleanup(OpalFrameClock *clock) {
  OpalFrameClockNode *node =
      (OpalFrameClockNode *)malloc(sizeof(OpalFrameClockNode));
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
      fprintf(stderr,
              "Runtime error: failed to register frame clock cleanup\n");
      exit(1);
    }
    OPAL_FRAME_CLOCKS_CLEANUP_REGISTERED = 1;
  }
}

static const char *stdout_error_from_errno(const char *default_error) {
  if (errno == EPIPE) {
    return OPAL_STDOUT_SINK_CLOSED_ERROR;
  }
  return default_error;
}

static FILE *stdout_stream(void) { return stdout; }

static void opal_stdout_handles_cleanup_all(void) {
  OpalStdoutWriterNode *writer_node = OPAL_STDOUT_WRITERS;
  OpalStdoutTerminalNode *terminal_node = OPAL_STDOUT_TERMINALS;
  while (writer_node != NULL) {
    OpalStdoutWriterNode *next = writer_node->next;
    free(writer_node->writer);
    free(writer_node);
    writer_node = next;
  }
  while (terminal_node != NULL) {
    OpalStdoutTerminalNode *next = terminal_node->next;
    free(terminal_node->terminal);
    free(terminal_node);
    terminal_node = next;
  }
  OPAL_STDOUT_WRITERS = NULL;
  OPAL_STDOUT_TERMINALS = NULL;
}

static int opal_stdout_handles_register_cleanup(void) {
  if (!OPAL_STDOUT_HANDLES_CLEANUP_REGISTERED) {
    if (atexit(opal_stdout_handles_cleanup_all) != 0) {
      return 0;
    }
    OPAL_STDOUT_HANDLES_CLEANUP_REGISTERED = 1;
  }
  return 1;
}

static OpalStdoutWriter *opal_stdout_writer_new(void) {
  OpalStdoutWriterNode *node = NULL;
  OpalStdoutWriter *writer = NULL;
  if (!opal_stdout_handles_register_cleanup()) {
    return NULL;
  }
  writer = (OpalStdoutWriter *)malloc(sizeof(*writer));
  node = (OpalStdoutWriterNode *)malloc(sizeof(*node));
  if (writer == NULL || node == NULL) {
    free(writer);
    free(node);
    return NULL;
  }
  writer->stream = stdout_stream();
  writer->lease_epoch = OPAL_TERMINAL_LEASE_EPOCH;
  node->writer = writer;
  node->next = OPAL_STDOUT_WRITERS;
  OPAL_STDOUT_WRITERS = node;
  return writer;
}

static OpalStdoutTerminal *opal_stdout_terminal_new(void) {
  OpalStdoutTerminalNode *node = NULL;
  OpalStdoutTerminal *terminal = NULL;
  if (!opal_stdout_handles_register_cleanup()) {
    return NULL;
  }
  terminal = (OpalStdoutTerminal *)malloc(sizeof(*terminal));
  node = (OpalStdoutTerminalNode *)malloc(sizeof(*node));
  if (terminal == NULL || node == NULL) {
    free(terminal);
    free(node);
    return NULL;
  }
  terminal->stream = stdout_stream();
  terminal->lease_epoch = OPAL_TERMINAL_LEASE_EPOCH;
  node->terminal = terminal;
  node->next = OPAL_STDOUT_TERMINALS;
  OPAL_STDOUT_TERMINALS = node;
  return terminal;
}

static int diagnostic_lane_allows(void) {
  return OPAL_TERMINAL_STATE == OPAL_TERMINAL_FREE ||
         OPAL_TERMINAL_STATE == OPAL_TERMINAL_ACTIVE ||
         OPAL_TERMINAL_STATE == OPAL_TERMINAL_PAUSED;
}

static const char *terminal_state_name(int state) {
  switch (state) {
  case OPAL_TERMINAL_FREE:
    return "Free";
  case OPAL_TERMINAL_OPENING:
    return "Opening";
  case OPAL_TERMINAL_ACTIVE:
    return "Active";
  case OPAL_TERMINAL_PAUSED:
    return "Paused";
  case OPAL_TERMINAL_RESTORE_PENDING:
    return "RestorePending";
  case OPAL_TERMINAL_FAILED_OPEN_RECOVERY:
    return "FailedOpenRecovery";
  case OPAL_TERMINAL_FAILED_CLOSE_RECOVERY:
    return "FailedCloseRecovery";
  default:
    return "Unknown";
  }
}

static const char *terminal_operation_name(const char *operation) {
  return operation;
}

static const char *stdout_coordinator_error(const char *family,
                                            const char *operation,
                                            uint64_t lease_epoch,
                                            int has_lease) {
  static char message[256];
  if (OPAL_TERMINAL_STATE != OPAL_TERMINAL_FREE ||
      (has_lease && lease_epoch != OPAL_TERMINAL_LEASE_EPOCH)) {
    (void)snprintf(message, sizeof(message),
                   "%s: TerminalCoordinatorUnavailable { state: %s, operation: %s }",
                   family, terminal_state_name(OPAL_TERMINAL_STATE),
                   terminal_operation_name(operation));
    return message;
  }
  return NULL;
}

static FsStringResult stdout_string_error(const char *error) {
  FsStringResult result = {NULL, error};
  return result;
}

static FsStringResult stdout_string_success(char *value) {
  FsStringResult result = {value, NULL};
  return result;
}

static FsHandleResult stdout_handle_error(const char *error) {
  FsHandleResult result = {NULL, error};
  return result;
}

static FsHandleResult stdout_handle_success(void *value) {
  FsHandleResult result = {value, NULL};
  return result;
}

static FsBooleanResult stdout_boolean_error(const char *error) {
  FsBooleanResult result = {0, error};
  return result;
}

static FsBooleanResult stdout_boolean_success(int8_t value) {
  FsBooleanResult result = {value, NULL};
  return result;
}

#ifdef OPAL_ENABLE_INTERNAL_TESTING
void opal_terminal_test_reset(void) {
  OPAL_TERMINAL_STATE = OPAL_TERMINAL_FREE;
  OPAL_TERMINAL_LEASE_EPOCH = 0;
}

void opal_terminal_test_set_state(int state) { OPAL_TERMINAL_STATE = state; }

int opal_terminal_test_reserve_opening(void) {
  uint64_t next_epoch = 0;
  if (OPAL_TERMINAL_STATE != OPAL_TERMINAL_FREE) {
    return 0;
  }
  if (OPAL_TERMINAL_LEASE_EPOCH == UINT64_MAX) {
    return 0;
  }
  next_epoch = OPAL_TERMINAL_LEASE_EPOCH + 1;
  OPAL_TERMINAL_LEASE_EPOCH = next_epoch;
  OPAL_TERMINAL_STATE = OPAL_TERMINAL_OPENING;
  return 1;
}

void opal_terminal_test_return_free(void) {
  OPAL_TERMINAL_STATE = OPAL_TERMINAL_FREE;
}
#endif

static FsVoidResult stdout_write_stream(FILE *stream, const char *value) {
  const char *safe_value = value ? value : "";
  size_t value_length = strlen(safe_value);

  if (stream == NULL) {
    return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
  }

  clearerr(stream);
  errno = 0;
  if (value_length > 0) {
    size_t written = fwrite(safe_value, 1, value_length, stream);
    if (written != value_length || ferror(stream)) {
      return stdout_void_error(
          stdout_error_from_errno(OPAL_STDOUT_WRITE_FAILURE_ERROR));
    }
  }

  return stdout_void_success();
}

static FsVoidResult stdout_flush_stream(FILE *stream) {
  if (stream == NULL) {
    return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
  }

  clearerr(stream);
  errno = 0;
  if (fflush(stream) != 0) {
    return stdout_void_error(
        stdout_error_from_errno(OPAL_STDOUT_FLUSH_FAILURE_ERROR));
  }

  return stdout_void_success();
}

static FsVoidResult terminal_write_stream(FILE *stream, const char *value) {
  const char *safe_value = value ? value : "";
  size_t value_length = strlen(safe_value);

  if (stream == NULL) {
    return stdout_void_error(OPAL_STDOUT_SINK_CLOSED_ERROR);
  }

  clearerr(stream);
  errno = 0;
  if (value_length > 0) {
    size_t written = fwrite(safe_value, 1, value_length, stream);
    if (written != value_length || ferror(stream)) {
      return stdout_void_error(
          stdout_error_from_errno(OPAL_TERMINAL_WRITE_FAILURE_ERROR));
    }
  }

  return stdout_void_success();
}

static FILE *terminal_stream(OpalStdoutTerminal *terminal) {
  return terminal ? terminal->stream : NULL;
}

static int8_t terminal_supports_ansi_stream(FILE *stream) {
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

static FsVoidResult terminal_write_cursor_move(FILE *stream, int32_t row,
                                               int32_t column) {
  char escape_sequence[64];
  int written = 0;

  if (row < 0 || column < 0) {
    return terminal_invalid_cursor_position_error();
  }

  written = snprintf(escape_sequence, sizeof(escape_sequence),
                     "\x1b[%" PRId32 ";%" PRId32 "H", row + 1, column + 1);
  if (written < 0 || (size_t)written >= sizeof(escape_sequence)) {
    return stdout_void_error(OPAL_TERMINAL_WRITE_FAILURE_ERROR);
  }

  return terminal_write_stream(stream, escape_sequence);
}

static FsVoidResult terminal_write_rows(FILE *stream, const char **rows,
                                        int64_t count) {
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

static char *duplicate_without_trailing_newline(const char *source) {
  char *raw = opal_strdup(source);
  if (!raw) {
    fprintf(stderr, "Runtime error: out of memory\n");
    exit(1);
  }
  size_t len = strlen(raw);
  if (len > 0 && raw[len - 1] == '\n') {
    raw[--len] = '\0';
    if (len > 0 && raw[len - 1] == '\r') {
      raw[--len] = '\0';
    }
  }

  size_t trimmed_len = strlen(raw);
  char *out = (char *)malloc(trimmed_len + 1);
  if (!out) {
    fprintf(stderr, "Runtime error: out of memory\n");
    exit(1);
  }
  memcpy(out, raw, trimmed_len + 1);
  free(raw);
  return out;
}

FsStringResult take_input(void) {
  char *line = NULL;
  size_t len = 0;
  ssize_t read = 0;
  const char *coordinator_error = stdout_coordinator_error(
      "StandardInputReadError", "TakeInput", 0, 0);

  if (coordinator_error != NULL) {
    return stdout_string_error(coordinator_error);
  }

  read = opal_getline(&line, &len, stdin);
  if (read == -1) {
    if (line != NULL) {
      free(line);
    }
    if (feof(stdin)) {
      return stdout_string_error("StandardInputReadError: EndOfInput");
    }
    return stdout_string_error("StandardInputReadError: ReadFailure");
  }

  char *result = duplicate_without_trailing_newline(line);
  free(line);
  return stdout_string_success(result);
}

#define OPAL_DIAGNOSTIC_BUFFER_CAP 256
#define OPAL_DIAGNOSTIC_TRUNCATION "...[truncated]"

static int diagnostic_scalar_is_hazardous(uint32_t code) {
  return code <= 0x1FU || (code >= 0x7FU && code <= 0x9FU) ||
         code == 0x061CU || (code >= 0x200EU && code <= 0x200FU) ||
         (code >= 0x202AU && code <= 0x202EU) ||
         (code >= 0x2060U && code <= 0x2064U) ||
         (code >= 0x2066U && code <= 0x206FU) ||
         (code >= 0xFDD0U && code <= 0xFDEFU) ||
         (code & 0xFFFFU) >= 0xFFFEU;
}

static size_t diagnostic_append_escape(char *output, size_t offset,
                                       uint32_t code) {
  int written;
  if (code <= 0xFFU) {
    written = snprintf(output + offset, OPAL_DIAGNOSTIC_BUFFER_CAP - offset,
                       "\\x%02" PRIX32, code);
  } else {
    written = snprintf(output + offset, OPAL_DIAGNOSTIC_BUFFER_CAP - offset,
                       "\\u{%" PRIX32 "}", code);
  }
  if (written < 0) {
    return offset;
  }
  return offset + (size_t)written;
}

static size_t diagnostic_sanitize(const char *value, char *output) {
  const unsigned char *input = (const unsigned char *)(value ? value : "");
  size_t input_offset = 0;
  size_t output_offset = 0;
  const size_t marker_length = sizeof(OPAL_DIAGNOSTIC_TRUNCATION) - 1U;

  while (input[input_offset] != '\0') {
    uint32_t code = input[input_offset];
    size_t width = 1U;
    if (code >= 0xC2U && code <= 0xDFU && input[input_offset + 1U] >= 0x80U &&
        input[input_offset + 1U] <= 0xBFU) {
      code = ((code & 0x1FU) << 6U) | (input[input_offset + 1U] & 0x3FU);
      width = 2U;
    } else if (code >= 0xE0U && code <= 0xEFU &&
               input[input_offset + 1U] >= 0x80U &&
               input[input_offset + 1U] <= 0xBFU &&
               input[input_offset + 2U] >= 0x80U &&
               input[input_offset + 2U] <= 0xBFU) {
      code = ((code & 0x0FU) << 12U) |
             ((input[input_offset + 1U] & 0x3FU) << 6U) |
             (input[input_offset + 2U] & 0x3FU);
      width = 3U;
    } else if (code >= 0xF0U && code <= 0xF4U &&
               input[input_offset + 1U] >= 0x80U &&
               input[input_offset + 1U] <= 0xBFU &&
               input[input_offset + 2U] >= 0x80U &&
               input[input_offset + 2U] <= 0xBFU &&
               input[input_offset + 3U] >= 0x80U &&
               input[input_offset + 3U] <= 0xBFU) {
      code = ((code & 0x07U) << 18U) |
             ((input[input_offset + 1U] & 0x3FU) << 12U) |
             ((input[input_offset + 2U] & 0x3FU) << 6U) |
             (input[input_offset + 3U] & 0x3FU);
      width = 4U;
    }

    if (diagnostic_scalar_is_hazardous(code)) {
      size_t escaped_length = code <= 0xFFU ? 4U : 10U;
      if (output_offset + escaped_length + marker_length >=
          OPAL_DIAGNOSTIC_BUFFER_CAP) {
        memcpy(output + output_offset, OPAL_DIAGNOSTIC_TRUNCATION,
               marker_length);
        output_offset += marker_length;
        break;
      }
      output_offset = diagnostic_append_escape(output, output_offset, code);
    } else {
      if (output_offset + width + marker_length >= OPAL_DIAGNOSTIC_BUFFER_CAP) {
        memcpy(output + output_offset, OPAL_DIAGNOSTIC_TRUNCATION,
               marker_length);
        output_offset += marker_length;
        break;
      }
      memcpy(output + output_offset, input + input_offset, width);
      output_offset += width;
    }
    input_offset += width;
  }
  output[output_offset] = '\0';
  return output_offset;
}

void print_string(const char *s) {
  static char diagnostic_buffer[OPAL_DIAGNOSTIC_BUFFER_CAP];
  if (!diagnostic_lane_allows()) {
    return;
  }
  (void)diagnostic_sanitize(s, diagnostic_buffer);
  puts(diagnostic_buffer);
}

FsVoidResult print_text_sync(const char *value) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_WRITE_COORDINATOR_UNAVAILABLE_ERROR, "PrintText", 0, 0);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  return stdout_write_stream(stdout_stream(), value);
}

FsVoidResult flush_standard_output_sync(void) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_FLUSH_COORDINATOR_UNAVAILABLE_ERROR, "FlushStandardOutput", 0, 0);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
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
  OpalFrameClock *clock = NULL;

  if (frames_per_second <= 0) {
    return frame_clock_result_error(OPAL_INVALID_FRAME_RATE_ERROR);
  }
  if (opal_monotonic_time_ms(&now_ms) != 0) {
    return frame_clock_result_error(OPAL_INVALID_FRAME_RATE_ERROR);
  }

  clock = (OpalFrameClock *)malloc(sizeof(*clock));
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

FsVoidResult frame_clock_wait_next_sync(OpalFrameClock *clock) {
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

FsHandleResult stdout_writer(void) {
  const char *coordinator_error = stdout_coordinator_error(
      "StandardOutputHandleError", "StdoutWriter", 0, 0);
  if (coordinator_error != NULL) {
    return stdout_handle_error(coordinator_error);
  }
  OpalStdoutWriter *writer = opal_stdout_writer_new();
  if (writer == NULL) {
    return stdout_handle_error("StandardOutputHandleError: AllocationFailure");
  }
  return stdout_handle_success(writer);
}

FsVoidResult writer_write_sync(OpalStdoutWriter *writer, const char *value) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_WRITE_COORDINATOR_UNAVAILABLE_ERROR, "WriterWrite",
      writer ? writer->lease_epoch : 0, writer != NULL);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  return stdout_write_stream(writer ? writer->stream : NULL, value);
}

FsVoidResult writer_flush_sync(OpalStdoutWriter *writer) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_FLUSH_COORDINATOR_UNAVAILABLE_ERROR, "WriterFlush",
      writer ? writer->lease_epoch : 0, writer != NULL);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  return stdout_flush_stream(writer ? writer->stream : NULL);
}

FsHandleResult stdout_terminal(void) {
  const char *coordinator_error = stdout_coordinator_error(
      "StandardOutputHandleError", "StdoutTerminal", 0, 0);
  if (coordinator_error != NULL) {
    return stdout_handle_error(coordinator_error);
  }
  OpalStdoutTerminal *terminal = opal_stdout_terminal_new();
  if (terminal == NULL) {
    return stdout_handle_error("StandardOutputHandleError: AllocationFailure");
  }
  return stdout_handle_success(terminal);
}

FsBooleanResult terminal_supports_ansi(OpalStdoutTerminal *terminal) {
  const char *coordinator_error = stdout_coordinator_error(
      "StandardOutputCapabilityError", "TerminalSupportsAnsi",
      terminal ? terminal->lease_epoch : 0, terminal != NULL);
  if (coordinator_error != NULL) {
    return stdout_boolean_error(coordinator_error);
  }
  if (terminal == NULL) {
    return stdout_boolean_error("StandardOutputCapabilityError: InvalidHandle");
  }
  return stdout_boolean_success(terminal_supports_ansi_stream(terminal_stream(terminal)));
}

FsVoidResult terminal_clear_screen_on_sync(OpalStdoutTerminal *terminal) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_TERMINAL_COORDINATOR_UNAVAILABLE_ERROR, "TerminalClearScreenOn",
      terminal ? terminal->lease_epoch : 0, terminal != NULL);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  return terminal_write_stream(terminal_stream(terminal),
                               "\x1b[2J\x1b[3J\x1b[H");
}

FsVoidResult terminal_move_cursor_on_sync(OpalStdoutTerminal *terminal,
                                          int32_t row, int32_t column) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_TERMINAL_COORDINATOR_UNAVAILABLE_ERROR, "TerminalMoveCursorOn",
      terminal ? terminal->lease_epoch : 0, terminal != NULL);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  return terminal_write_cursor_move(terminal_stream(terminal), row, column);
}

FsVoidResult terminal_draw_rows_sync(OpalStdoutTerminal *terminal,
                                     const char **rows, int64_t count) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_TERMINAL_COORDINATOR_UNAVAILABLE_ERROR, "TerminalDrawRows",
      terminal ? terminal->lease_epoch : 0, terminal != NULL);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  return terminal_write_rows(terminal_stream(terminal), rows, count);
}

FsVoidResult terminal_clear_screen_sync(void) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_TERMINAL_COORDINATOR_UNAVAILABLE_ERROR, "TerminalClearScreen", 0,
      0);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  FsHandleResult terminal_result = stdout_terminal();
  if (terminal_result.error != NULL) {
    return stdout_void_error(terminal_result.error);
  }
  return terminal_clear_screen_on_sync((OpalStdoutTerminal *)terminal_result.value);
}

FsVoidResult terminal_move_cursor_sync(int32_t row, int32_t column) {
  const char *coordinator_error = stdout_coordinator_error(
      OPAL_TERMINAL_COORDINATOR_UNAVAILABLE_ERROR, "TerminalMoveCursor", 0,
      0);
  if (coordinator_error != NULL) {
    return stdout_void_error(coordinator_error);
  }
  FsHandleResult terminal_result = stdout_terminal();
  if (terminal_result.error != NULL) {
    return stdout_void_error(terminal_result.error);
  }
  return terminal_move_cursor_on_sync((OpalStdoutTerminal *)terminal_result.value,
                                      row, column);
}

static int opal_terminal_fake_backend_enabled(void) {
  const char *enabled = getenv("OPAL_TERMINAL_FAKE_BACKEND");
  return enabled != NULL && strcmp(enabled, "1") == 0;
}

static char *opal_terminal_duplicate_cstr(const char *text) {
  const char *safe_text = text ? text : "";
  size_t length = strlen(safe_text);
  char *copy = (char *)malloc(length + 1u);
  if (copy == NULL) {
    return NULL;
  }
  memcpy(copy, safe_text, length + 1u);
  return copy;
}

FsHandleResult terminal_session_open_sync(void *options) {
  (void)options;
  if (!opal_terminal_fake_backend_enabled()) {
    return stdout_handle_error(OPAL_TERMINAL_FAKE_BACKEND_REQUIRED_ERROR);
  }
  OpalTerminalSession *session =
      (OpalTerminalSession *)calloc(1u, sizeof(OpalTerminalSession));
  if (session == NULL) {
    return stdout_handle_error("TerminalSessionOpenError: AllocationFailure");
  }
  session->fake_backend = 1;
  OPAL_TERMINAL_STATE = OPAL_TERMINAL_ACTIVE;
  return stdout_handle_success(session);
}

FsHandleResult terminal_session_read_event_sync(void *opaque_session, void *wait,
                                                void *cancellation_token) {
  (void)wait;
  (void)cancellation_token;
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_handle_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  const char *events = getenv("OPAL_TERMINAL_FAKE_EVENTS");
  if (events == NULL || events[0] == '\0' || session->read_count > 0) {
    return stdout_handle_error(OPAL_TERMINAL_FAKE_END_OF_INPUT_ERROR);
  }
  session->read_count += 1;
  char *event_copy = opal_terminal_duplicate_cstr(events);
  if (event_copy == NULL) {
    return stdout_handle_error("TerminalSessionReadError: AllocationFailure");
  }
  return stdout_handle_success(event_copy);
}

FsVoidResult terminal_session_write_sync(void *opaque_session, void *trusted_output) {
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_void_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  return terminal_write_stream(stdout, (const char *)trusted_output);
}

FsVoidResult terminal_session_flush_sync(void *opaque_session) {
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_void_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  return stdout_flush_stream(stdout);
}

FsVoidResult terminal_session_clear_screen_sync(void *opaque_session) {
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_void_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  return terminal_write_stream(stdout, "\x1b[2J\x1b[3J\x1b[H");
}

FsVoidResult terminal_session_move_cursor_sync(void *opaque_session, int32_t row,
                                              int32_t column) {
  char escape_sequence[64];
  int written = 0;
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_void_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  if (row < 1 || column < 1) {
    return terminal_invalid_cursor_position_error();
  }
  written = snprintf(escape_sequence, sizeof(escape_sequence),
                     "\x1b[%" PRId32 ";%" PRId32 "H", row, column);
  if (written < 0 || (size_t)written >= sizeof(escape_sequence)) {
    return stdout_void_error(OPAL_TERMINAL_WRITE_FAILURE_ERROR);
  }
  return terminal_write_stream(stdout, escape_sequence);
}

FsVoidResult terminal_session_draw_rows_sync(void *opaque_session,
                                            const char **rows, int64_t count) {
  int64_t index = 0;
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_void_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  if (count <= 0 || rows == NULL) {
    return stdout_void_success();
  }
  for (index = 0; index < count; index++) {
    FsVoidResult write_result = terminal_write_stream(stdout, rows[index]);
    if (write_result.error != NULL) {
      return write_result;
    }
    write_result = terminal_write_stream(stdout, "\n");
    if (write_result.error != NULL) {
      return write_result;
    }
  }
  return stdout_void_success();
}

FsVoidResult terminal_session_bell_sync(void *opaque_session) {
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_void_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  return terminal_write_stream(stdout, "\a");
}

FsHandleResult terminal_session_close_sync(void *opaque_session) {
  OpalTerminalSession *session = (OpalTerminalSession *)opaque_session;
  if (session == NULL || session->closed) {
    return stdout_handle_error(OPAL_TERMINAL_SESSION_CLOSED_ERROR);
  }
  session->closed = 1;
  OPAL_TERMINAL_STATE = OPAL_TERMINAL_FREE;
  return stdout_handle_success(opal_terminal_duplicate_cstr("TerminalCloseOutcome.Ok"));
}
