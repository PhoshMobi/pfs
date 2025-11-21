/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use gettextrs::{bind_textdomain_codeset, bindtextdomain};
use gtk::{gdk, gio};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{GETTEXT_PACKAGE, LOCALEDIR};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    let _ = gtk::init();
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    gio::resources_register_include_impl(include_bytes!(concat!(
        env!("PFS_RESOURCE_DIR"),
        "/",
        "pfs.gresource"
    )))
    .expect("Failed to register pfs resources.");

    let display = gdk::Display::default().unwrap();
    let icon_theme = gtk::IconTheme::for_display(&display);
    gtk::IconTheme::add_resource_path(&icon_theme, "/mobi/phosh/FileSelector/icons");

    let provider = gtk::CssProvider::new();
    provider.load_from_file(&gio::File::for_uri(
        "resource:///mobi/phosh/FileSelector/style.css",
    ));

    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    INITIALIZED.store(true, Ordering::Release);
}

// C bindings:

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_init() {
    init()
}
