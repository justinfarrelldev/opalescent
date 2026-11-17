#ifndef OPAL_TESTING_TERMINAL_H
#define OPAL_TESTING_TERMINAL_H
#include "opal_testing_workspace.h"
#include <stdbool.h>
#include <stdint.h>
/* ApplicationExecutable and TerminalProcess are opaque test-library handles. */
const char *application_executable(void);
OpalTestingWorkspaceResult terminal_process_launch(const char *executable, const char *argument,
    const char *directory, int64_t columns, int64_t rows);
OpalTestingWorkspaceResult terminal_process_send_text(void *process, const char *text);
OpalTestingWorkspaceResult terminal_process_send_key(void *process, const char *key);
char *terminal_process_row(void *process, int64_t row);
int64_t terminal_process_cursor_line(void *process);
int64_t terminal_process_cursor_column(void *process);
bool terminal_process_exited(void *process);
int64_t terminal_process_exit_code(void *process);
char *terminal_process_transcript(void *process);
OpalTestingWorkspaceResult terminal_process_stop(void *process);
#endif
