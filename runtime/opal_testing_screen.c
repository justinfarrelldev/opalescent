/* Incremental VT screen observations; application output is never replayed as input. */
#include "opal_testing_screen.h"
#include <stdlib.h>
#include <string.h>
extern char *opal_string_alloc(uint64_t bytes);
extern int64_t terminal_text_cell_width(const char *text);

int opal_testing_screen_init(OpalTestingScreen *s, size_t columns, size_t rows) {
    memset(s, 0, sizeof(*s));
    if (!columns || !rows || columns > 512 || rows > 512) return 0;
    s->cells = calloc(columns * rows, sizeof(*s->cells));
    if (!s->cells) return 0;
    s->columns = columns; s->rows = rows;
    return 1;
}

static void scroll(OpalTestingScreen *s) {
    if (s->row < s->rows) return;
    memmove(s->cells, s->cells + s->columns, (s->rows - 1) * s->columns * sizeof(*s->cells));
    memset(s->cells + (s->rows - 1) * s->columns, 0, s->columns * sizeof(*s->cells));
    s->row = s->rows - 1;
}

static void glyph(OpalTestingScreen *s, const char *text) {
    int64_t width = terminal_text_cell_width(text);
    if (width == 0) {
        if (s->column) {
            char *previous = s->cells[s->row * s->columns + s->column - 1].text;
            if (strlen(previous) + strlen(text) < sizeof(s->cells[0].text)) strcat(previous, text);
        }
        return;
    }
    if (s->column >= s->columns) {s->column = 0; ++s->row; scroll(s);}
    OpalTestingCell *cell = &s->cells[s->row * s->columns + s->column];
    memcpy(cell->text, text, strlen(text) + 1);
    ++s->column;
    if (width > 1 && s->column < s->columns) {
        s->cells[s->row * s->columns + s->column].text[0] = '\1';
        s->cells[s->row * s->columns + s->column].text[1] = 0;
        ++s->column;
    }
}

static size_t bounded(unsigned parameter, size_t extent) {
    size_t value = parameter ? (size_t)parameter - 1 : 0;
    return value < extent ? value : extent - 1;
}

static void csi(OpalTestingScreen *s, unsigned char command) {
    unsigned n = s->parameters[0] ? s->parameters[0] : 1;
    size_t start = s->row * s->columns + (s->column < s->columns ? s->column : s->columns - 1);
    switch (command) {
    case 'H': case 'f': s->row = bounded(s->parameters[0], s->rows); s->column = bounded(s->parameters[1], s->columns); break;
    case 'A': s->row = n > s->row ? 0 : s->row - n; break;
    case 'B': s->row = n >= s->rows - s->row ? s->rows - 1 : s->row + n; break;
    case 'C': s->column = n >= s->columns - (s->column < s->columns ? s->column : s->columns - 1) ? s->columns - 1 : s->column + n; break;
    case 'D': s->column = n > s->column ? 0 : s->column - n; break;
    case 'G': s->column = bounded(s->parameters[0], s->columns); break;
    case 'd': s->row = bounded(s->parameters[0], s->rows); break;
    case 'J':
        if (s->parameters[0] == 2 || s->parameters[0] == 3) memset(s->cells, 0, s->rows * s->columns * sizeof(*s->cells));
        else if (!s->parameters[0]) memset(s->cells + start, 0, (s->rows * s->columns - start) * sizeof(*s->cells));
        break;
    case 'K':
        if (s->parameters[0] == 2) memset(s->cells + s->row * s->columns, 0, s->columns * sizeof(*s->cells));
        else if (!s->parameters[0]) memset(s->cells + start, 0, ((s->row + 1) * s->columns - start) * sizeof(*s->cells));
        break;
    case 's': s->saved_row = s->row; s->saved_column = s->column; break;
    case 'u': s->row = s->saved_row; s->column = s->saved_column; break;
    default: break; /* SGR and private visibility/mode sequences do not occupy cells. */
    }
}

void opal_testing_screen_feed(OpalTestingScreen *s, const char *bytes, size_t length) {
    for (size_t i = 0; i < length; ++i) {
        unsigned char value = (unsigned char)bytes[i];
        if (s->escape == 1) {s->escape = value == '[' ? 2u : (value == ']' ? 3u : 0u); s->parameter = 0; memset(s->parameters, 0, sizeof(s->parameters)); continue;}
        if (s->escape == 3) {if (value == 7) s->escape = 0; else if (value == 27) s->escape = 4; continue;}
        if (s->escape == 4) {s->escape = value == '\\' ? 0u : 3u; continue;}
        if (s->escape == 2) {
            if (value >= '0' && value <= '9') {unsigned *p = &s->parameters[s->parameter]; if (*p < 100000u) *p = *p * 10u + (unsigned)(value - '0');}
            else if (value == ';') {if (s->parameter < 7) ++s->parameter;}
            else if (value >= 0x40 && value <= 0x7e) {csi(s, value); s->escape = 0;}
            continue;
        }
        if (s->utf8_needed) {
            if ((value & 0xc0u) != 0x80u) {s->utf8_needed = s->utf8_length = 0; glyph(s, "?");}
            else {s->utf8[s->utf8_length++] = (char)value; if (s->utf8_length == s->utf8_needed) {s->utf8[s->utf8_length] = 0; glyph(s, s->utf8); s->utf8_needed = s->utf8_length = 0;} continue;}
        }
        if (value == 27) s->escape = 1;
        else if (value == '\r') s->column = 0;
        else if (value == '\n') {++s->row; scroll(s);}
        else if (value == '\b') {if (s->column) --s->column;}
        else if (value == '\t') {s->column = ((s->column / 8) + 1) * 8; if (s->column >= s->columns) s->column = s->columns - 1;}
        else if (value >= 0x20 && value < 0x7f) {char text[2] = {(char)value, 0}; glyph(s, text);}
        else if (value >= 0xc2 && value <= 0xf4) {s->utf8[0] = (char)value; s->utf8_length = 1; s->utf8_needed = value < 0xe0 ? 2u : (value < 0xf0 ? 3u : 4u);}
    }
}

char *opal_testing_screen_row(OpalTestingScreen *s, size_t row) {
    if (row >= s->rows) abort();
    size_t end = s->columns;
    while (end && !s->cells[row * s->columns + end - 1].text[0]) --end;
    char *result = opal_string_alloc((uint64_t)(end * sizeof(s->cells[0].text) + 1));
    if (!result) abort();
    size_t offset = 0;
    for (size_t i = 0; i < end; ++i) {
        const char *text = s->cells[row * s->columns + i].text;
        if (text[0] == '\1') continue;
        if (!*text) result[offset++] = ' ';
        else {size_t size = strlen(text); memcpy(result + offset, text, size); offset += size;}
    }
    result[offset] = 0;
    return result;
}
