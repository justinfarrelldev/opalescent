#ifndef OPAL_RUNTIME_H
#define OPAL_RUNTIME_H

#include <stdint.h>

void opal_runtime_init(void);

extern uint64_t opal_runtime_string_index_span_start;
extern uint64_t opal_runtime_string_index_span_len;
extern const char* opal_runtime_string_index_source_path;
extern const char* opal_runtime_string_index_source_text;

typedef struct { int8_t value;   const char* error; } ParseResultI8;
typedef struct { int16_t value;  const char* error; } ParseResultI16;
typedef struct { int32_t value;  const char* error; } ParseResultI32;
#ifndef OPAL_PARSE_RESULT_I64_DEFINED
typedef struct { int64_t value;  const char* error; } ParseResultI64;
#define OPAL_PARSE_RESULT_I64_DEFINED 1
#endif
typedef struct { uint8_t value;  const char* error; } ParseResultU8;
typedef struct { uint16_t value; const char* error; } ParseResultU16;
typedef struct { uint32_t value; const char* error; } ParseResultU32;
typedef struct { uint64_t value; const char* error; } ParseResultU64;
typedef struct { float value;    const char* error; } ParseResultF32;
typedef struct { double value;   const char* error; } ParseResultF64;

#ifndef OPAL_FS_STRING_RESULT_DEFINED
typedef struct { char* value; const char* error; } FsStringResult;
#define OPAL_FS_STRING_RESULT_DEFINED 1
#endif
typedef struct { void* value; const char* error; } FsHandleResult;
#ifndef OPAL_FS_BOOLEAN_RESULT_DEFINED
typedef struct { int8_t value; const char* error; } FsBooleanResult;
#define OPAL_FS_BOOLEAN_RESULT_DEFINED 1
#endif

void opal_runtime_error(const char* message);

FsStringResult take_input(void);
void print_string(const char* s);

void print_int8(int8_t n);
void print_int16(int16_t n);
void print_int32(int32_t n);
void print_int64(int64_t n);
void print_uint8(uint8_t n);
void print_uint16(uint16_t n);
void print_uint32(uint32_t n);
void print_uint64(uint64_t n);
void print_float32(float n);
void print_float64(double n);

int8_t  random_int8(int8_t min, int8_t max);
int16_t random_int16(int16_t min, int16_t max);
int32_t random_int32(int32_t min, int32_t max);
int64_t random_int64(int64_t min, int64_t max);
uint8_t  random_uint8(uint8_t min, uint8_t max);
uint16_t random_uint16(uint16_t min, uint16_t max);
uint32_t random_uint32(uint32_t min, uint32_t max);
uint64_t random_uint64(uint64_t min, uint64_t max);

ParseResultI8  string_to_int8(const char* s);
ParseResultI16 string_to_int16(const char* s);
ParseResultI32 string_to_int32(const char* s);
ParseResultI64 string_to_int64(const char* s);
ParseResultU8  string_to_uint8(const char* s);
ParseResultU16 string_to_uint16(const char* s);
ParseResultU32 string_to_uint32(const char* s);
ParseResultU64 string_to_uint64(const char* s);
ParseResultF32 string_to_float32(const char* s);
ParseResultF64 string_to_float64(const char* s);

char* int8_to_string(int8_t value);
char* int16_to_string(int16_t value);
char* int32_to_string(int32_t value);
char* int64_to_string(int64_t value);
char* uint8_to_string(uint8_t value);
char* uint16_to_string(uint16_t value);
char* uint32_to_string(uint32_t value);
char* uint64_to_string(uint64_t value);
char* float32_to_string(float value);
char* float64_to_string(double value);
char* bool_to_string(int8_t value);
int64_t string_length(const char* value);
char* string_index(const char* value, int64_t index);
int64_t string_find_index_or(const char* value, const char* search_text, int64_t fallback_index);
ParseResultI64 string_find_last_index_of_text(const char* value, const char* search_text);
typedef struct OpalStringBuilder OpalStringBuilder;
typedef struct { void* value; const char* error; } StringBuilderVoidResult;
typedef struct { char* value; const char* error; } StringBuilderStringResult;
StringBuilderStringResult string_join(const char** values, int64_t count, const char* separator);
OpalStringBuilder* string_builder_new(void);
StringBuilderVoidResult string_builder_push(OpalStringBuilder* builder, const char* value);
StringBuilderStringResult string_builder_finish(OpalStringBuilder* builder);
int64_t array_length(const void* array, int64_t length);
void opal_array_bounds_error(uint64_t index, uint64_t length);

/* `Bytes` stdlib surface. `OpalBytes` is an opaque owned heap pointer
 * passed across the FFI boundary as `i8*`. Fallible helpers mirror the
 * `ParseResult*` `{value, error}` convention so `guard`/`propagate`
 * lowering is identical. */
typedef struct OpalBytes OpalBytes;
typedef struct { OpalBytes* value; const char* error; } BytesResult;

OpalBytes* bytes_new(void);
int32_t    bytes_length(OpalBytes* bytes);
char*      bytes_to_hex(OpalBytes* bytes);
OpalBytes* bytes_concatenate(OpalBytes* left, OpalBytes* right);
BytesResult bytes_from_hex(const char* hex);
BytesResult bytes_slice(OpalBytes* source, int32_t start, int32_t end);
char* path_from(const char* raw);
char* join_path_components(const char* base, const char** components, int64_t count);
char* path_parent_directory(const char* path);
char* path_file_name(const char* path);
char* path_file_extension(const char* path);
char* normalize_path(const char* path);
char* path_to_string(const char* path);

/* Filesystem stdlib surface result types.
 *
 * FsResult success/failure sentinel contract (frozen ABI rule for lowering):
 * - Success is represented exclusively by `error == NULL`.
 * - Failure is represented exclusively by `error != NULL`.
 * - The payload field(s) (`value`, and `count` for array results) are only
 *   semantically valid on success and MUST be treated as undefined on failure.
 * - `FsVoidResult` has no payload semantics; callers and lowering must still
 *   use only `error` as the sentinel, and ignore `value` on both paths.
 *
 * This contract applies to all filesystem result wrappers used by guard/
 * propagate lowering, including:
 * - FsPathResult
 * - FsBytesResult
 * - FsStringResult
 * - FsStringArrayResult
 * - FsVoidResult
 * - FsBooleanResult
 * - FsMetadataResult
 * - FsPathArrayResult
 *
 * Infallible lexical path helper policy (char* ABI, no Fs*Result wrappers):
 * - `path_from(raw)` returns a heap-owned duplicate of `raw`; NULL/empty input
 *   returns the empty-string sentinel "".
 * - `normalize_path(path)` is lexical-only (no filesystem probes): separators are
 *   collapsed, `.` segments removed, and `..` segments resolved. Absolute paths
 *   preserve their leading separator. If an absolute path would escape above
 *   root via `..`, normalization returns the empty-string sentinel "".
 * - `join_path_components(base, parts)` is lexical-only: absolute components
 *   reset the accumulator, separators are deduplicated to one path separator,
 *   and the final path is normalized with the same `.`/`..` rules as
 *   `normalize_path`.
 * - Trailing-separator behavior is preserved for non-empty normalized paths;
 *   empty input and root-escape normalization both resolve to "".
 */
typedef struct { void*      value; const char* error; } FsVoidResult;
typedef struct { OpalBytes* value; const char* error; } FsBytesResult;
typedef struct OpalStdoutWriter OpalStdoutWriter;
typedef struct OpalStdoutTerminal OpalStdoutTerminal;
typedef struct OpalFrameClock OpalFrameClock;
typedef struct { OpalFrameClock* value; const char* error; } FsFrameClockResult;
FsVoidResult print_text_sync(const char* value);
FsVoidResult flush_standard_output_sync(void);
FsHandleResult stdout_writer(void);
FsVoidResult writer_write_sync(OpalStdoutWriter* writer, const char* value);
FsVoidResult writer_flush_sync(OpalStdoutWriter* writer);
FsHandleResult stdout_terminal(void);
FsBooleanResult terminal_supports_ansi(OpalStdoutTerminal* terminal);
FsVoidResult terminal_clear_screen_on_sync(OpalStdoutTerminal* terminal);
FsVoidResult terminal_move_cursor_on_sync(OpalStdoutTerminal* terminal, int32_t row, int32_t column);
FsVoidResult terminal_draw_rows_sync(OpalStdoutTerminal* terminal, const char** rows, int64_t count);
FsVoidResult terminal_clear_screen_sync(void);
FsVoidResult terminal_move_cursor_sync(int32_t row, int32_t column);
#ifdef OPAL_ENABLE_INTERNAL_TESTING
#define OPAL_TERMINAL_TEST_FREE 0
#define OPAL_TERMINAL_TEST_OPENING 1
#define OPAL_TERMINAL_TEST_ACTIVE 2
#define OPAL_TERMINAL_TEST_PAUSED 3
#define OPAL_TERMINAL_TEST_RESTORE_PENDING 4
#define OPAL_TERMINAL_TEST_FAILED_OPEN_RECOVERY 5
#define OPAL_TERMINAL_TEST_FAILED_CLOSE_RECOVERY 6
void opal_terminal_test_reset(void);
void opal_terminal_test_set_state(int state);
int opal_terminal_test_reserve_opening(void);
void opal_terminal_test_return_free(void);
void opal_terminal_test_reset_invalid_options_errors(void);
int32_t opal_terminal_test_options_use_alternate_screen(void* options);
int64_t opal_terminal_test_options_mouse_tracking_tag(void* options);
int32_t opal_terminal_test_options_maximum_retained_bytes(void* options);
int32_t opal_terminal_test_options_maximum_correlated_bytes(void* options);
int32_t opal_terminal_test_options_maximum_retained_events(void* options);
int32_t opal_terminal_test_options_maximum_correlated_events(void* options);
int32_t opal_terminal_test_invalid_options_kind(const char* error);
uint64_t opal_terminal_test_invalid_options_required(const char* error);
uint64_t opal_terminal_test_invalid_options_configured(const char* error);
#endif
FsVoidResult sleep_ms_sync(int32_t milliseconds);
FsFrameClockResult frame_clock_new(int32_t frames_per_second);
FsVoidResult frame_clock_wait_next_sync(OpalFrameClock* clock);
FsHandleResult system_wait_set_new(void);
FsHandleResult system_wait_set_register(void* wait_set, void* source);
FsVoidResult system_wait_set_remove(void* wait_set, void* registration);
FsHandleResult system_wait_set_register_owned(void* wait_set, void* source);
FsVoidResult system_owned_wait_registration_retarget(void* registration, void* source);
FsVoidResult system_owned_wait_registration_remove(void* registration);
FsHandleResult system_wait_set_wait_sync(void* wait_set, void* cancellation);
FsHandleResult cancellation_source_new(void);
void* cancellation_token(void* source);
void cancellation_request(void* source);
FsHandleResult monotonic_timer_new(void);
void* monotonic_timer_readiness_source(void* timer);
ParseResultU64 monotonic_timer_arm(void* timer, void* deadline);
ParseResultU64 monotonic_timer_disarm(void* timer);
uint64_t monotonic_timer_generation(void* timer);
FsHandleResult monotonic_timer_deadline(void* timer);
void* monotonic_clock_now(void);
FsHandleResult process_control_source_new(void);
void* process_control_readiness_source(void* source);
FsHandleResult process_control_poll(void* source);
FsVoidResult process_control_acknowledge_suspend(void* source, uint64_t generation);
FsVoidResult process_control_resume_application(void* source, uint64_t generation);
void* terminal_session_options_default(void);
FsHandleResult terminal_session_options_with_feature_policy(void* options, void* policy);
FsHandleResult terminal_session_options_with_resource_limits(void* options, void* limits);
FsHandleResult terminal_session_options_validate(void* options);
FsHandleResult trusted_terminal_output_from_application_text(const char* text);
FsHandleResult terminal_session_open_sync(void* options);
void* terminal_session_state(void* session);
void* terminal_session_capabilities(void* session);
void* terminal_capabilities_feature(void* capabilities, void* feature);
void* terminal_capabilities_trusted_paste_framing(void* capabilities);
void* terminal_capabilities_color(void* capabilities);
FsHandleResult terminal_session_size_sync(void* session);
FsHandleResult terminal_session_read_event_sync(void* session, void* wait, void* cancellation_token);
FsVoidResult terminal_session_write_sync(void* session, void* trusted_output);
FsVoidResult terminal_session_flush_sync(void* session);
FsVoidResult terminal_session_clear_screen_sync(void* session);
FsVoidResult terminal_session_move_cursor_sync(void* session, int32_t row, int32_t column);
FsVoidResult terminal_session_draw_rows_sync(void* session, const char** rows, int64_t count);
FsVoidResult terminal_session_bell_sync(void* session);
FsVoidResult terminal_session_set_cursor_visible_sync(void* session, int8_t visible);
FsVoidResult terminal_session_set_cursor_shape_sync(void* session, void* shape);
FsHandleResult terminal_session_close_sync(void* session);
void* __opal_using_cleanup_terminal_session_close_sync(void* session);
FsHandleResult opal_terminal_constrain_i32_range(int32_t value, int32_t min, int32_t max);
FsHandleResult opal_terminal_constrain_u8_control_code(uint8_t value);
void system_wait_set_drop(void* wait_set);
void process_control_source_drop(void* source);
void monotonic_timer_drop(void* timer);
void cancellation_source_drop(void* source);
#ifndef OPAL_FS_STRING_RESULT_DEFINED
typedef struct { char*      value; const char* error; } FsStringResult;
#define OPAL_FS_STRING_RESULT_DEFINED 1
#endif
#ifndef OPAL_ERROR_TRUNCATION_DEFINED
typedef struct OpalErrorTruncation {
    int8_t cause_depth;
    int8_t suppressed_count;
    int8_t bytes;
} OpalErrorTruncation;
#define OPAL_ERROR_TRUNCATION_DEFINED 1
#endif
char* opal_error_new(const char* variant_name);
char* opal_error_attach_cause(const char* primary, const char* cause);
FsStringResult error_cause(const char* error_value);
int64_t error_suppressed_length(const char* error_value);
FsStringResult error_suppressed_at(const char* error_value, int64_t index);
OpalErrorTruncation* error_attachment_truncation(const char* error_value);
int8_t error_attachment_truncation_cause_depth(OpalErrorTruncation* truncation);
int8_t error_attachment_truncation_suppressed_count(OpalErrorTruncation* truncation);
int8_t error_attachment_truncation_bytes(OpalErrorTruncation* truncation);
#ifndef OPAL_FS_BOOLEAN_RESULT_DEFINED
typedef struct { int8_t     value; const char* error; } FsBooleanResult;
#define OPAL_FS_BOOLEAN_RESULT_DEFINED 1
#endif
typedef struct { int32_t    value; const char* error; } FsInt32Result;
typedef struct { int64_t    value; const char* error; } FsInt64Result;
typedef struct { char*      value; const char* error; } FsPathResult;
typedef struct { char**     value; int64_t count; const char* error; } FsPathArrayResult;
#ifndef OPAL_FS_STRING_ARRAY_RESULT_DEFINED
typedef struct { char**     value; int64_t count; const char* error; } FsStringArrayResult;
#define OPAL_FS_STRING_ARRAY_RESULT_DEFINED 1
#endif
#ifndef OPAL_FS_STRING_RESULT_TYPES_DEFINED
#define OPAL_FS_STRING_RESULT_TYPES_DEFINED 1
#endif
typedef struct { void*      value; const char* error; } FsMetadataResult;
typedef struct { void*      value; const char* error; } FsPermissionsResult;

FsPathResult current_working_directory_sync(void);
FsPathResult current_executable_path_sync(void);
FsPathResult current_executable_directory_sync(void);
FsVoidResult set_current_working_directory_sync(const char* path);
FsStringResult get_environment_variable(const char* name);
FsStringResult get_environment_variable_or(const char* name, const char* default_value);
FsBooleanResult environment_variable_exists(const char* name);
void exit_process(int32_t code);
FsBytesResult read_contents_sync(const char* path);
FsStringResult read_text_sync(const char* path);
FsStringResult read_first_line_sync(const char* path);
FsStringArrayResult read_lines_sync(const char* path);
FsStringArrayResult string_split_lines(const char* value);
int8_t string_is_blank(const char* value);
FsStringResult string_trim_whitespace(const char* value);
FsStringResult string_take_prefix(const char* value, int64_t count);
FsStringResult string_take_suffix(const char* value, int64_t count);
FsStringResult string_extract_range(const char* value, int64_t start, int64_t end);
FsStringResult string_insert_at(const char* value, int64_t scalar_index, const char* inserted);
FsStringResult string_delete_range(const char* value, int64_t start, int64_t end);
FsStringResult string_replace_range(const char* value, int64_t start, int64_t end, const char* replacement);
typedef struct { char* value; int64_t used_cells; const char* error; } FsStringInt64Result;
int64_t terminal_text_cell_width(const char* value);
FsStringInt64Result terminal_text_clip_to_cells(const char* value, int64_t max_cells);
FsBytesResult read_bytes_at_offset_sync(const char* path, int64_t offset, int64_t length);
FsVoidResult write_contents_sync(const char* path, OpalBytes* data);
FsVoidResult write_text_sync(const char* path, const char* text);
FsVoidResult write_contents_atomic_sync(const char* path, OpalBytes* data);
FsVoidResult write_text_atomic_sync(const char* path, const char* text);
FsVoidResult append_contents_sync(const char* path, OpalBytes* data);
FsVoidResult append_text_sync(const char* path, const char* text);
FsVoidResult write_bytes_at_offset_sync(const char* path, int64_t offset, OpalBytes* data);
FsVoidResult create_file_sync(const char* path);
FsVoidResult delete_file_sync(const char* path);
FsVoidResult copy_file_sync(const char* source, const char* destination);
FsVoidResult move_path_sync(const char* source, const char* destination);
FsBooleanResult path_exists_sync(const char* path);
FsMetadataResult read_metadata_sync(const char* path);
FsMetadataResult read_metadata_nofollow_sync(const char* path);
FsVoidResult create_directory_sync(const char* path);
FsVoidResult create_directory_recursive_sync(const char* path);
FsVoidResult delete_directory_sync(const char* path);
FsVoidResult delete_directory_recursive_sync(const char* path);
FsPathArrayResult list_directory_sync(const char* path);
FsBooleanResult is_file_sync(const char* path);
FsBooleanResult is_file_nofollow_sync(const char* path);
FsBooleanResult is_directory_sync(const char* path);
FsBooleanResult is_directory_nofollow_sync(const char* path);

#endif
