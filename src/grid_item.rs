/*
 * Copyright 2024-2025 Phosh.mobi e.V.
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

use crate::{
    config::LOG_DOMAIN, dir_view::ThumbnailMode, file_props::FileProps, file_selector::FileSelector,
};

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

        #[template_child]
        pub context_menu: TemplateChild<gtk::PopoverMenu>,

        #[property(get, set = Self::set_fileinfo)]
        pub fileinfo: RefCell<Option<gio::FileInfo>>,

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

            klass.install_action("grid-item.show-property", None, move |item, _, _| {
                item.show_properties();
            });
            klass.install_action("grid-item.copy-name", None, move |item, _, _| {
                item.copy_to_clipboard();
            });
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
                        self.icon.set_from_file(Some(path));
                        have_thumbnail = true;
                    }
                }
            }

            if !have_thumbnail {
                if let Some(icon) = info.icon() {
                    self.icon.set_from_gicon(&icon);
                }
            }
        }

        fn set_fileinfo(&self, info: gio::FileInfo) {
            self.label.set_label(&info.display_name());

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

        fn dispose(&self) {
            if self.context_menu.parent().is_some() {
                self.context_menu.unparent();
            }
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
            imp.icon.set_from_file(Some(path));
        }
    }

    fn get_file_selector(&self) -> FileSelector {
        self.root()
            .and_then(|w| w.downcast_ref::<FileSelector>().cloned())
            .expect("FileSelector must be at the root")
    }

    fn show_properties(&self) {
        let imp = self.imp();

        let fileinfo = imp.fileinfo.borrow();
        let info = fileinfo.as_ref().unwrap();
        let file = info
            .attribute_object("standard::file")
            .unwrap()
            .downcast::<gio::File>()
            .unwrap();

        let uri = file.uri();
        glib::g_debug!(LOG_DOMAIN, "Showing properties for {uri}");

        let file_props = glib::Object::builder::<FileProps>()
            .property("file", &file)
            .build();

        file_props.set_transient_for(Some(&self.get_file_selector()));
        file_props.present();
    }

    fn copy_to_clipboard(&self) {
        let imp = self.imp();

        let filename = imp.label.text();

        self.clipboard().set_text(&filename);

        let toast_message = gettextrs::gettext("Copied to clipboard");
        let toast = adw::Toast::builder()
            .title(&toast_message)
            .timeout(2)
            .build();

        self.get_file_selector().show_toast(toast);
    }

    fn show_context_menu(&self, x: f64, y: f64) {
        // Disable context menu when used as portal
        if self.get_file_selector().close_on_done() {
            return;
        }

        let imp = self.imp();
        let popover = &imp.context_menu;

        popover.unparent();
        popover.set_parent(self);
        popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    }

    #[template_callback]
    fn on_long_press_pressed(&self, x: f64, y: f64) {
        self.show_context_menu(x, y);
    }

    #[template_callback]
    fn on_right_click_pressed(&self, _n_press: i32, x: f64, y: f64) {
        self.show_context_menu(x, y);
    }
}
