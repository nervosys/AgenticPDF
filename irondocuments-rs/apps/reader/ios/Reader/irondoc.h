// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The C surface of the Rust core. Referenced from the bridging header so Swift
// sees these as ordinary functions.
//
// Every function returning `char *` hands over ownership; the caller must pass
// it to irondoc_string_free. Reader.swift wraps each one so no call site has to
// remember.

#ifndef IRONDOC_H
#define IRONDOC_H

#include <stdbool.h>
#include <stddef.h>

char  *irondoc_open(const unsigned char *data, size_t len);
char  *irondoc_execute(const char *action, const char *params);
char  *irondoc_render_page(float width, float height, float zoom);
size_t irondoc_save(unsigned char *out, size_t capacity);
bool   irondoc_is_dirty(void);
char  *irondoc_capabilities(void);
void   irondoc_string_free(char *pointer);

#endif
