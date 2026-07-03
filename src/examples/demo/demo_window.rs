/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;

use pfs::file_selector::{FileSelector, FileSelectorBuilder, FileSelectorMode};

use crate::config::LOG_DOMAIN;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/mobi/phosh/FileSelectorDemo/demo-window.ui")]
    pub struct PfsDemoWindow {
        #[template_child]
        pub selected_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub choices_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub filter_row: TemplateChild<adw::ActionRow>,

        pub file_selector: RefCell<Option<FileSelector>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PfsDemoWindow {
        const NAME: &'static str = "PfsDemoWindow";
        type Type = super::PfsDemoWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("win.open-file", None, move |win, _, _| {
                win.open_file(false);
            });
            klass.install_action("win.open-files", None, move |win, _, _| {
                win.open_file(true);
            });
            klass.install_action("win.save-file", None, move |win, _, _| {
                win.save_file();
            });
            klass.install_action("win.save-files", None, move |win, _, _| {
                win.save_files();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PfsDemoWindow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for PfsDemoWindow {}
    impl WindowImpl for PfsDemoWindow {}
    impl ApplicationWindowImpl for PfsDemoWindow {}
    impl AdwApplicationWindowImpl for PfsDemoWindow {}
}

glib::wrapper! {
    pub struct PfsDemoWindow(ObjectSubclass<imp::PfsDemoWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[gtk::template_callbacks]
impl PfsDemoWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn set_result(&self, selector: FileSelector, success: bool) {
        glib::g_debug!(LOG_DOMAIN, "File dialog done, result: {success:#?}");
        let selected = selector.selected();

        let uris = match selected {
            None => vec!["".to_string()],
            Some(vec) => vec,
        };

        self.imp().selected_row.set_subtitle(&uris[0]);
    }

    pub fn open_file(&self, multiple: bool) {
        glib::g_debug!(LOG_DOMAIN, "Open File (multiple: {multiple})");
        let filters = gio::ListStore::with_type(gtk::FileFilter::static_type());
        filters.append(
            &glib::Object::builder::<gtk::FileFilter>()
                .property("mime-types", glib::StrV::from(["image/jpeg", "image/png"]))
                .property("name", "Image files")
                .build(),
        );
        filters.append(
            &glib::Object::builder::<gtk::FileFilter>()
                .property("mime-types", glib::StrV::from(["text/plain"]))
                .property("name", "Text files")
                .build(),
        );
        let all_files = glib::Object::builder::<gtk::FileFilter>()
            .property("patterns", glib::StrV::from(["*"]))
            .property("name", "All files")
            .build();
        filters.append(&all_files);
        let pos = filters.find(&all_files).unwrap();

        let title = if multiple {
            "Select Files"
        } else {
            "Select a File"
        };
        let file_selector = FileSelectorBuilder::new()
            .accept_label("Done")
            .title(title)
            .current_folder(gio::File::for_path("/home"))
            .filters(filters.into())
            .current_filter(pos)
            .multiple(multiple)
            .build();

        let empty: Vec<(String, String)> = Vec::new();
        let choices = [
            (
                "encoding",
                "Encoding",
                [("utf8", "Unicode (UTF-8)"), ("latin15", "Western")].to_variant(),
                "latin15",
            ),
            ("reencode", "Reencode", empty.to_variant(), "false"),
        ]
        .to_variant();
        file_selector.set_choices(&choices);
        file_selector.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                self,
                move |selector: FileSelector, success: bool| {
                    glib::g_debug!(LOG_DOMAIN, "File dialog done, result: {success:#?}");
                    let selected = selector.selected();

                    this.imp().selected_row.set_subtitle("");
                    this.imp().choices_row.set_subtitle("");
                    this.imp().filter_row.set_subtitle("");

                    if success {
                        let uris = match selected {
                            None => vec!["".to_string()],
                            Some(vec) => vec,
                        };
                        let pretty_uris = uris.join("\n");
                        this.imp().selected_row.set_subtitle(&pretty_uris);

                        let text = match selector.selected_choices() {
                            Some(choices) => choices.to_string(),
                            None => "".to_string(),
                        };
                        this.imp().choices_row.set_subtitle(&text);

                        let pos = selector.current_filter();
                        let text = match selector.filters() {
                            Some(filters) => match filters.item(pos) {
                                Some(filter) => filter
                                    .downcast::<gtk::FileFilter>()
                                    .unwrap()
                                    .name()
                                    .unwrap()
                                    .to_string(),
                                None => "".to_string(),
                            },
                            None => "".to_string(),
                        };
                        this.imp().filter_row.set_subtitle(&text);
                    }
                }
            ),
        );

        file_selector.set_mode(FileSelectorMode::OpenFile);
        file_selector.set_transient_for(Some(self));
        file_selector.present();
        *self.imp().file_selector.borrow_mut() = Some(file_selector);
    }

    pub fn save_file(&self) {
        glib::g_debug!(LOG_DOMAIN, "Save File");
        let file_selector = FileSelectorBuilder::new()
            .accept_label("Save")
            .title("Save File")
            .current_folder(gio::File::for_path("/home"))
            .filename("newfile.txt")
            .build();

        file_selector.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                self,
                move |selector: FileSelector, success: bool| {
                    this.set_result(selector, success);
                }
            ),
        );

        file_selector.set_mode(FileSelectorMode::SaveFile);
        file_selector.set_transient_for(Some(self));
        file_selector.present();
        *self.imp().file_selector.borrow_mut() = Some(file_selector);
    }

    pub fn save_files(&self) {
        glib::g_debug!(LOG_DOMAIN, "Save Files");
        let file_selector = FileSelectorBuilder::new()
            .title("Save Files")
            .accept_label("Done")
            .current_folder(gio::File::for_path("/home"))
            .build();

        file_selector.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                self,
                move |selector: FileSelector, success: bool| {
                    this.set_result(selector, success);
                }
            ),
        );

        file_selector.set_mode(FileSelectorMode::SaveFiles);
        file_selector.set_transient_for(Some(self));
        file_selector.present();
        *self.imp().file_selector.borrow_mut() = Some(file_selector);
    }
}
