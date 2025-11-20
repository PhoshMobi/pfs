/*
 * Copyright 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

mod config;
mod pfs_open_application;

use self::pfs_open_application::PfsOpenApplication;

use config::{GETTEXT_PACKAGE, LOCALEDIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::glib;
use gtk::prelude::*;

fn main() -> glib::ExitCode {
    let app_id = "mobi.phosh.FileOpen";

    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    pfs::init::init();

    let app = PfsOpenApplication::new(app_id);
    glib::set_prgname(Some(app_id));
    app.run()
}
