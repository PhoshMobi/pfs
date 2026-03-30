/*
 * Copyright 2026 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/bookmarks-item.ui")]
    #[properties(wrapper_type = super::BookmarksItem)]
    pub struct BookmarksItem {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub label: TemplateChild<gtk::Label>,

        #[property(get, set)]
        place: RefCell<String>,

        #[property(get, set)]
        gicon: RefCell<Option<gio::Icon>>,

        #[property(get, set)]
        uri: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BookmarksItem {
        const NAME: &'static str = "PfsBookmarksItem";
        type Type = super::BookmarksItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl BookmarksItem {}

    #[glib::derived_properties]
    impl ObjectImpl for BookmarksItem {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for BookmarksItem {}
    impl BinImpl for BookmarksItem {}
}

glib::wrapper! {
    pub struct BookmarksItem(ObjectSubclass<imp::BookmarksItem>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for BookmarksItem {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl BookmarksItem {
    pub fn new() -> Self {
        Self::default()
    }
}
