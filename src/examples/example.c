/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

#include <pfs.h>
#include <glib.h>
#include <gtk/gtk.h>
#include <adwaita.h>


static void
on_done(PfsFileSelector *selector, gboolean success, gpointer user_data)
{
  g_auto(GStrv) selection = NULL;
  GMainLoop *loop = user_data;

  g_assert (PFS_IS_FILE_SELECTOR (selector));
  g_assert (loop);

  g_main_loop_quit (loop);

  g_message ("Dialog done, success: %d", success);
  if (!success)
    return;

  selection = pfs_file_selector_get_selected (selector);
  for (guint i = 0; i < g_strv_length (selection); i++)
    g_message ("    Uri: %s\n", selection[i]);
}


int
main(int argc, char *argv[])
{
  g_autoptr (GOptionContext) opt_context = NULL;
  g_autoptr (GError) err = NULL;
  g_autoptr (PfsFileSelector) selector = NULL;
  g_autoptr (GMainLoop) loop = g_main_loop_new (NULL, FALSE);
  g_autofree char *directory = NULL;
  const GOptionEntry options [] = {
    {"directory", 'D', 0, G_OPTION_ARG_STRING, &directory,
     "Directory to show file chooser for", NULL},
    { NULL, 0, 0, G_OPTION_ARG_NONE, NULL, NULL, NULL }
  };

  opt_context = g_option_context_new ("- a simple file chooser");
  g_option_context_add_main_entries (opt_context, options, NULL);
  if (!g_option_context_parse (opt_context, &argc, &argv, &err)) {
    g_warning ("%s", err->message);
    return 1;
  }
  directory = directory ?: g_strdup ("/home");

  gtk_init ();
  adw_init ();
  pfs_init ();

  selector = pfs_file_selector_new ();
  pfs_file_selector_set_current_directory (selector, directory);
  pfs_file_selector_set_accept_label (selector, "Open…");
  pfs_file_selector_set_mode (selector, PFS_FILE_SELECTOR_MODE_OPEN_FILE);
  g_signal_connect (selector, "done", G_CALLBACK (on_done), loop);

  gtk_window_present (GTK_WINDOW (selector));

  g_main_loop_run (loop);
}
