#ifndef OPAL_TESTING_SCREEN_H
#define OPAL_TESTING_SCREEN_H
#include <stddef.h>
#include <stdint.h>
typedef struct { char text[12]; } OpalTestingCell;
typedef struct {
    size_t rows, columns, row, column, saved_row, saved_column;
    OpalTestingCell *cells;
    unsigned escape, parameters[8], parameter;
    char utf8[5];
    size_t utf8_length, utf8_needed;
} OpalTestingScreen;
int opal_testing_screen_init(OpalTestingScreen *screen, size_t columns, size_t rows);
void opal_testing_screen_feed(OpalTestingScreen *screen, const char *bytes, size_t length);
char *opal_testing_screen_row(OpalTestingScreen *screen, size_t row);
#endif
