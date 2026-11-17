//! Real PTY transport and incremental normalized screen observations.
#![cfg(target_os = "linux")]

#[test]
fn synchronous_terminal_driver_observes_screen_cursor_exit_and_transcript() {
    let directory = tempfile::tempdir().expect("terminal driver");
    let script = directory.path().join("child.sh");
    std::fs::write(&script,"test -z \"$OPAL_TERMINAL_FAKE_BACKEND\" || exit 42\npwd > actual-cwd\nstty size > actual-size\nprintf '\\033[2J\\033[Hcafé NORMAL\\033[2;3HX'\nread value\ntest \"$value\" = hello || exit 43\nexit 7\n").expect("child script");
    let source = directory.path().join("driver.c");
    std::fs::write(&source,r#"
#include "opal_portability.h"
#include "opal_testing_terminal.h"
#include "opal_testing_screen.h"
#include <stdlib.h>
#include <string.h>
#include <time.h>
char *opal_string_alloc(uint64_t bytes) {return malloc((size_t)bytes);}
int64_t terminal_text_cell_width(const char *text) {
    if (!strcmp(text, "界")) return 2;
    if (!strcmp(text, "\xcc\x81")) return 0;
    return 1;
}
static void pause_read(void) {struct timespec delay = {0, 5000000}; nanosleep(&delay, NULL);}
int main(int argc, char **argv) {
    if (argc != 3) return 1;
    OpalTestingScreen screen;
    if (!opal_testing_screen_init(&screen, 12, 4)) return 2;
    opal_testing_screen_feed(&screen, "\033[Hca", 5);
    const char split[] = {'f', (char)0xc3};
    opal_testing_screen_feed(&screen, split, sizeof(split));
    const char last[] = {(char)0xa9, 0};
    opal_testing_screen_feed(&screen, last, 1);
    char *line = opal_testing_screen_row(&screen, 0);
    if (strcmp(line, "café")) return 3;
    free(line);
    opal_testing_screen_feed(&screen, "\033[2J\033[H界X", strlen("\033[2J\033[H界X"));
    line = opal_testing_screen_row(&screen, 0);
    if (strcmp(line, "界X") || screen.column != 3) return 4;
    free(line); free(screen.cells);
    OpalTestingWorkspaceResult launched = terminal_process_launch("/bin/sh", argv[1], argv[2], 12, 4);
    if (launched.error || !launched.value) return 5;
    void *process = launched.value;
    int ready = 0;
    for (unsigned i = 0; i < 200 && !ready; ++i) {
        line = terminal_process_row(process, 0);
        ready = !strcmp(line, "café NORMAL"); free(line);
        if (!ready) pause_read();
    }
    if (!ready || terminal_process_cursor_line(process) != 1 || terminal_process_cursor_column(process) != 3) return 6;
    if (terminal_process_send_text(process, "hello").error || terminal_process_send_key(process, "Enter").error) return 7;
    for (unsigned i = 0; i < 200 && !terminal_process_exited(process); ++i) pause_read();
    if (!terminal_process_exited(process) || terminal_process_exit_code(process) != 7) return 8;
    char *transcript = terminal_process_transcript(process);
    if (!strstr(transcript, "café NORMAL") || !strstr(transcript, "hello")) return 9;
    free(transcript);
    if (terminal_process_stop(process).error) return 10;
    return 0;
}
"#).expect("C driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    let executable = directory.path().join("driver");
    let compiler = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Wpedantic", "-Werror"])
        .arg("-I")
        .arg(&runtime)
        .arg(&source)
        .arg(runtime.join("opal_testing_terminal.c"))
        .arg(runtime.join("opal_testing_screen.c"))
        .args(["-lutil", "-o"])
        .arg(&executable)
        .output()
        .expect("compiler");
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let output = std::process::Command::new(executable)
        .arg(script)
        .arg(directory.path())
        .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
        .env("OPAL_TERMINAL_FAKE_EVENTS", "invalid fake input")
        .output()
        .expect("driver");
    assert!(
        output.status.success(),
        "driver status {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("actual-cwd"))
            .expect("actual directory")
            .trim(),
        directory.path().to_str().expect("directory")
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("actual-size"))
            .expect("actual terminal geometry")
            .trim(),
        "4 12"
    );
}
