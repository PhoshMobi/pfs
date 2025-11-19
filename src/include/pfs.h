/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#pragma once

#include <glib-object.h>
#include <gio/gio.h>
#include <gtk/gtk.h>
#include <adwaita.h>

G_BEGIN_DECLS

typedef enum
{
  PFS_FILE_SELECTOR_MODE_OPEN_FILE,
  PFS_FILE_SELECTOR_MODE_SAVE_FILE,
  PFS_FILE_SELECTOR_MODE_SAVE_FILES,
} PfsFileSelectorMode;

#define PFS_TYPE_FILE_SELECTOR (pfs_file_selector_get_type())
G_DECLARE_FINAL_TYPE(PfsFileSelector, pfs_file_selector, PFS, FILE_SELECTOR, AdwWindow)

void             pfs_init (void);
PfsFileSelector *pfs_file_selector_new (void);
void             pfs_file_selector_set_current_directory (PfsFileSelector      *self,
                                                          const char           *directory);
void             pfs_file_selector_set_accept_label      (PfsFileSelector      *self,
                                                          const char           *accept_label);
GStrv            pfs_file_selector_get_selected          (PfsFileSelector      *self);
void             pfs_file_selector_set_mode              (PfsFileSelector      *self,
                                                          PfsFileSelectorMode   mode);
void             pfs_file_selector_set_filename          (PfsFileSelector      *self,
                                                          const char           *suggested);
G_END_DECLS
