/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::PfsDemoWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PfsDemoApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for PfsDemoApplication {
        const NAME: &'static str = "PfsDemoApplication";
        type Type = super::PfsDemoApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for PfsDemoApplication {
        fn constructed(&self) {
            self.parent_constructed();

            gio::resources_register_include_impl(include_bytes!(concat!(
                env!("PFS_RESOURCE_DIR"),
                "/examples/demo/",
                "demo.gresource"
            )))
            .expect("Failed to register pfs resources.");

            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for PfsDemoApplication {
        fn activate(&self) {
            let application = self.obj();
            let window = application.active_window().unwrap_or_else(|| {
                let window = PfsDemoWindow::new(&*application);
                window.upcast()
            });

            window.present();
        }
    }

    impl GtkApplicationImpl for PfsDemoApplication {}
    impl AdwApplicationImpl for PfsDemoApplication {}
}

glib::wrapper! {
    pub struct PfsDemoApplication(ObjectSubclass<imp::PfsDemoApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PfsDemoApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, about_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Phosh File Selector Demo")
            .application_icon("mobi.phosh.FileSelectorDemo")
            .developer_name("Guido Günther")
            .version(VERSION)
            .developers(vec!["Guido Günther"])
            // Translators: Replace "translator-credits" with your name/username, and optionally an email or URL.
            .translator_credits(&gettext("translator-credits"))
            .copyright("© 2024-2025 Phosh.mobi e.V.")
            .build();

        about.present(Some(&window));
    }
}
