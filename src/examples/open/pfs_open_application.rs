/*
 * Copyright 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::process::Command;

use pfs::file_selector::{FileSelector, FileSelectorMode};

use crate::config::LOG_DOMAIN;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PfsOpenApplication {
        pub hold_guard: RefCell<Option<gio::ApplicationHoldGuard>>,
        pub hold_count: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PfsOpenApplication {
        const NAME: &'static str = "PfsOpenApplication";
        type Type = super::PfsOpenApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for PfsOpenApplication {
        fn constructed(&self) {
            self.parent_constructed();

            self.hold_count.set(0);
        }
    }

    impl ApplicationImpl for PfsOpenApplication {
        fn activate(&self) {
            let application = self.obj();

            let home = glib::home_dir();
            application.open_directory(&gio::File::for_path(&home));
        }

        fn open(&self, files: &[gio::File], _hint: &str) {
            for file in files.iter() {
                self.obj().open_directory(file);
            }
        }
    }

    impl GtkApplicationImpl for PfsOpenApplication {}
    impl AdwApplicationImpl for PfsOpenApplication {}
}

glib::wrapper! {
    pub struct PfsOpenApplication(ObjectSubclass<imp::PfsOpenApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PfsOpenApplication {
    pub fn new(application_id: &str) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", gio::ApplicationFlags::HANDLES_OPEN)
            .build()
    }

    fn open_directory(&self, file: &gio::File) {
        let imp = self.imp();
        let uri = file.uri();

        glib::g_message!(LOG_DOMAIN, "Opening {uri:#?}");

        if imp.hold_count.replace(imp.hold_count.get() + 1) == 0 {
            *self.imp().hold_guard.borrow_mut() = Some(self.hold());
        }

        let file_selector = glib::Object::builder::<FileSelector>()
            .property("accept_label", gettextrs::gettext("Open"))
            .property("title", "Select a File")
            .property("current-folder", file)
            .build();

        file_selector.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                self,
                move |selector: FileSelector, success: bool| {
                    glib::g_debug!(LOG_DOMAIN, "File dialog done, result: {success:#?}");
                    let imp = this.imp();
                    let selected = selector.selected();

                    if success {
                        let uris = match selected {
                            None => vec!["".to_string()],
                            Some(vec) => vec,
                        };
                        glib::g_message!(LOG_DOMAIN, "Opening {uris:#?}");

                        Command::new("gio")
                            .arg("open")
                            .arg(&uris[0])
                            .spawn()
                            .expect("Failed to open {uris[0]:?}")
                            .wait()
                            .expect("Failed to spawn gio");
                    }

                    if imp.hold_count.replace(imp.hold_count.get() - 1) == 1 {
                        // Drop the application ref count
                        this.imp().hold_guard.replace(None);
                    }
                }
            ),
        );

        file_selector.set_mode(FileSelectorMode::OpenFile);
        file_selector.present();
    }
}
