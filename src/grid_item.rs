/*
 * Copyright 2024 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib_macros::Properties;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};

use crate::dir_view::ThumbnailMode;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/grid-item.ui")]
    #[properties(wrapper_type = super::GridItem)]
    pub struct GridItem {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub label: TemplateChild<gtk::Label>,

        #[property(get, set = Self::set_fileinfo)]
        fileinfo: RefCell<Option<gio::FileInfo>>,

        #[property(get, set)]
        icon_size: Cell<u32>,

        #[property(get, set = Self::set_thumbnail_mode, builder(ThumbnailMode::default()))]
        pub thumbnail_mode: RefCell<ThumbnailMode>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GridItem {
        const NAME: &'static str = "PfsGridItem";
        type Type = super::GridItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl GridItem {
        fn update_image(&self) {
            let mut have_thumbnail = false;

            let borrowed = self.fileinfo.borrow();
            let Some(info) = borrowed.as_ref() else {
                return;
            };
            if *self.thumbnail_mode.borrow() != ThumbnailMode::Never {
                if let Some(path) = info.attribute_byte_string("thumbnail::path") {
                    let valid = info.boolean("thumbnail::is-valid");
                    if valid {
                        self.icon.get().set_from_file(Some(path));
                        have_thumbnail = true;
                    }
                }
            }

            if !have_thumbnail {
                if let Some(icon) = info.icon() {
                    self.icon.get().set_from_gicon(&icon);
                }
            }
        }

        fn set_fileinfo(&self, info: gio::FileInfo) {
            self.label.get().set_label(&info.display_name());

            *self.fileinfo.borrow_mut() = Some(info);
            self.update_image();
        }

        fn set_thumbnail_mode(&self, mode: ThumbnailMode) {
            if *self.thumbnail_mode.borrow() == mode {
                return;
            }

            self.thumbnail_mode.replace(mode);
            self.update_image();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for GridItem {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_icon_size(32);
        }
    }

    impl WidgetImpl for GridItem {}
    impl BinImpl for GridItem {}
}

glib::wrapper! {
    pub struct GridItem(ObjectSubclass<imp::GridItem>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for GridItem {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl GridItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_thumbnail(&self, path: String) {
        let imp = self.imp();

        if *imp.thumbnail_mode.borrow() != ThumbnailMode::Never {
            imp.icon.get().set_from_file(Some(path));
        }
    }
}
