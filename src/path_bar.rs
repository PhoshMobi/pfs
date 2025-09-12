/*
 * Copyright 2024 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::Signal;
use glib_macros::{clone, Properties};
use gtk::{gio, glib, CompositeTemplate};
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::config::LOG_DOMAIN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/path-bar.ui")]
    #[properties(wrapper_type = super::PathBar)]
    pub struct PathBar {
        #[template_child]
        pub path_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        // The current folder
        #[property(get, set = Self::set_folder)]
        folder: RefCell<Option<gio::File>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PathBar {
        const NAME: &'static str = "PfsPathBar";
        type Type = super::PathBar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl PathBar {
        fn set_folder(&self, folder: Option<gio::File>) {
            let Some(folder) = folder else { return };

            while let Some(child) = self.path_box.first_child() {
                self.path_box.remove(&child);
            }

            let Some(path) = folder.path() else {
                self.path_box.set_visible(false);
                return;
            };

            self.path_box.set_visible(true);
            for part in &path {
                let button = gtk::Button::new();
                button.set_label(part.to_str().unwrap());
                self.path_box.append(&button);
                button.connect_clicked(clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |clicked_button| {
                        let mut child = this.path_box.first_child().unwrap();

                        let mut pathbuf = std::path::PathBuf::new();
                        loop {
                            let button = child.downcast_ref::<gtk::Button>().unwrap();
                            pathbuf.push(button.label().unwrap());

                            if child == *clicked_button.upcast_ref::<gtk::Widget>() {
                                break;
                            }

                            child = match child.next_sibling() {
                                Some(child) => child,
                                None => break,
                            };
                        }

                        let file = gio::File::for_path(pathbuf);
                        let uri = file.uri();
                        glib::g_debug!(LOG_DOMAIN, "Selected path {uri:#?}");

                        this.obj().emit_by_name::<()>("new-uri", &[&uri]);
                    }
                ));
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PathBar {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("new-uri")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for PathBar {}
    impl BinImpl for PathBar {}
}

glib::wrapper! {
    pub struct PathBar(ObjectSubclass<imp::PathBar>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PathBar {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl PathBar {
    pub fn new() -> Self {
        Self::default()
    }
}
