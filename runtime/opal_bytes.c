/*
 * opal_bytes.c - Runtime implementation of the `Bytes` stdlib surface.
 *
 * Mirrors the Rust-side immutable Bytes API at `src/stdlib/bytes.rs` so that
 * Opalescent programs calling `bytes_*` builtins observe identical behaviour
 * regardless of which backend invokes them.
 *
 * Representation
 * --------------
 * `Bytes` values passed across the FFI boundary are owned heap pointers to
 * `OpalBytes { size_t length; uint8_t* data; }`. `data` is always either a
 * non-null heap buffer of exactly `length` bytes, or NULL when `length == 0`.
 * Constructors always allocate the header; the language treats the header
 * pointer as opaque (`i8*`).
 *
 * Error model
 * -----------
 * Fallible helpers return `{ OpalBytes*, const char* }` where a non-null
 * second field carries a static error string and the first field is NULL.
 * This matches the `ParseResult*` convention used by `opal_parse.c` and
 * integrates transparently with `guard` / `propagate` in the front-end.
 */

#include "opal_portability.h"
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  size_t length;
  uint8_t *data;
} OpalBytes;
#define OPAL_BYTES_TYPE_DEFINED 1

typedef struct {
  OpalBytes *value;
  const char *error;
} BytesResult;

void opal_runtime_error(const char *message);

/* ------------------------------------------------------------------------
 * Internal helpers
 * ------------------------------------------------------------------------ */

static OpalBytes *opal_bytes_allocate(size_t length) {
  OpalBytes *header = (OpalBytes *)malloc(sizeof(OpalBytes));
  if (header == NULL) {
    opal_runtime_error("out of memory: failed to allocate Bytes header");
    return NULL;
  }
  header->length = length;
  if (length == 0) {
    header->data = NULL;
    return header;
  }
  header->data = (uint8_t *)malloc(length);
  if (header->data == NULL) {
    free(header);
    opal_runtime_error("out of memory: failed to allocate Bytes buffer");
    return NULL;
  }
  return header;
}

static int opal_bytes_hex_char_to_nibble(char c, uint8_t *out_nibble) {
  if (c >= '0' && c <= '9') {
    *out_nibble = (uint8_t)(c - '0');
    return 1;
  }
  if (c >= 'a' && c <= 'f') {
    *out_nibble = (uint8_t)(10 + (c - 'a'));
    return 1;
  }
  if (c >= 'A' && c <= 'F') {
    *out_nibble = (uint8_t)(10 + (c - 'A'));
    return 1;
  }
  return 0;
}

static char opal_bytes_nibble_to_hex_char(uint8_t nibble) {
  static const char *lut = "0123456789abcdef";
  return lut[nibble & 0x0F];
}

/* ------------------------------------------------------------------------
 * Constructors and accessors
 * ------------------------------------------------------------------------ */

OpalBytes *bytes_new(void) { return opal_bytes_allocate(0); }

int32_t bytes_length(OpalBytes *bytes) {
  if (bytes == NULL) {
    opal_runtime_error("bytes_length called on NULL Bytes pointer");
    return 0;
  }
  if (bytes->length > (size_t)INT32_MAX) {
    opal_runtime_error("bytes_length overflow: buffer exceeds int32 range");
    return 0;
  }
  return (int32_t)bytes->length;
}

/* ------------------------------------------------------------------------
 * Combinators
 * ------------------------------------------------------------------------ */

OpalBytes *bytes_concatenate(OpalBytes *left, OpalBytes *right) {
  if (left == NULL || right == NULL) {
    opal_runtime_error("bytes_concatenate called with NULL Bytes pointer");
    return NULL;
  }
  size_t total = left->length + right->length;
  if (total < left->length) {
    opal_runtime_error(
        "bytes_concatenate overflow: combined length exceeds size_t");
    return NULL;
  }
  OpalBytes *combined = opal_bytes_allocate(total);
  if (combined == NULL) {
    return NULL;
  }
  if (left->length > 0 && left->data != NULL) {
    memcpy(combined->data, left->data, left->length);
  }
  if (right->length > 0 && right->data != NULL) {
    memcpy(combined->data + left->length, right->data, right->length);
  }
  return combined;
}

BytesResult bytes_slice(OpalBytes *source, int32_t start, int32_t end) {
  BytesResult result = {NULL, NULL};
  if (source == NULL) {
    result.error = "bytes_slice called with NULL Bytes pointer";
    return result;
  }
  if (start < 0 || end < 0) {
    result.error = "bytes_slice range must not contain negative bounds";
    return result;
  }
  if (start > end) {
    result.error = "bytes_slice range start must not exceed end";
    return result;
  }
  size_t start_unsigned = (size_t)start;
  size_t end_unsigned = (size_t)end;
  if (end_unsigned > source->length) {
    result.error = "bytes_slice range end exceeds buffer length";
    return result;
  }
  size_t span = end_unsigned - start_unsigned;
  OpalBytes *sub = opal_bytes_allocate(span);
  if (sub == NULL) {
    return result;
  }
  if (span > 0 && source->data != NULL) {
    memcpy(sub->data, source->data + start_unsigned, span);
  }
  result.value = sub;
  return result;
}

/* ------------------------------------------------------------------------
 * Hex encoding
 * ------------------------------------------------------------------------ */

char *bytes_to_hex(OpalBytes *bytes) {
  if (bytes == NULL) {
    opal_runtime_error("bytes_to_hex called with NULL Bytes pointer");
    return NULL;
  }
  /* Each byte becomes two hex chars; plus terminating NUL. */
  if (bytes->length > (SIZE_MAX - 1) / 2) {
    opal_runtime_error("bytes_to_hex overflow: encoded length exceeds size_t");
    return NULL;
  }
  size_t encoded_length = (bytes->length * 2) + 1;
  char *buffer = (char *)malloc(encoded_length);
  if (buffer == NULL) {
    opal_runtime_error("out of memory: failed to allocate hex encoding buffer");
    return NULL;
  }
  for (size_t index = 0; index < bytes->length; ++index) {
    uint8_t byte = bytes->data[index];
    buffer[(index * 2) + 0] =
        opal_bytes_nibble_to_hex_char((uint8_t)(byte >> 4));
    buffer[(index * 2) + 1] =
        opal_bytes_nibble_to_hex_char((uint8_t)(byte & 0x0F));
  }
  buffer[bytes->length * 2] = '\0';
  return buffer;
}

BytesResult bytes_from_hex(const char *hex) {
  BytesResult result = {NULL, NULL};
  if (hex == NULL) {
    result.error = "bytes_from_hex called with NULL string pointer";
    return result;
  }
  size_t hex_length = strlen(hex);
  if ((hex_length & 1U) != 0) {
    result.error = "bytes_from_hex requires an even-length hexadecimal input";
    return result;
  }
  size_t decoded_length = hex_length / 2;
  OpalBytes *decoded = opal_bytes_allocate(decoded_length);
  if (decoded == NULL) {
    return result;
  }
  for (size_t pair_index = 0; pair_index < decoded_length; ++pair_index) {
    uint8_t high_nibble = 0;
    uint8_t low_nibble = 0;
    if (!opal_bytes_hex_char_to_nibble(hex[pair_index * 2], &high_nibble)) {
      free(decoded->data);
      free(decoded);
      result.error =
          "bytes_from_hex encountered an invalid hexadecimal character";
      return result;
    }
    if (!opal_bytes_hex_char_to_nibble(hex[(pair_index * 2) + 1],
                                       &low_nibble)) {
      free(decoded->data);
      free(decoded);
      result.error =
          "bytes_from_hex encountered an invalid hexadecimal character";
      return result;
    }
    decoded->data[pair_index] = (uint8_t)((high_nibble << 4) | low_nibble);
  }
  result.value = decoded;
  return result;
}
