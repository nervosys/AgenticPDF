// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The C surface of the Rust core. Referenced from the bridging header so Swift
// sees these as ordinary functions.
//
// Every function returning `char *` hands over ownership; the caller must pass
// it to apdf_string_free. Reader.swift wraps each one so no call site has to
// remember.

#ifndef APDF_H
#define APDF_H

#include <stdbool.h>
#include <stddef.h>

char  *apdf_open(const unsigned char *data, size_t len);
char  *apdf_execute(const char *action, const char *params);
char  *apdf_render_page(float width, float height, float zoom);
size_t apdf_save(unsigned char *out, size_t capacity);
bool   apdf_is_dirty(void);
char  *apdf_capabilities(void);
void   apdf_string_free(char *pointer);

#endif
