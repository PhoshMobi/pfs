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
use gtk::{glib, CompositeTemplate};
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/places-item.ui")]
    #[properties(wrapper_type = super::PlacesItem)]
    pub struct PlacesItem {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub label: TemplateChild<gtk::Label>,

        #[property(get, set)]
        place: RefCell<String>,

        #[property(get, set)]
        icon_name: RefCell<String>,

        #[property(get, set)]
        uri: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlacesItem {
        const NAME: &'static str = "PfsPlacesItem";
        type Type = super::PlacesItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl PlacesItem {}

    #[glib::derived_properties]
    impl ObjectImpl for PlacesItem {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for PlacesItem {}
    impl BinImpl for PlacesItem {}
}

glib::wrapper! {
    pub struct PlacesItem(ObjectSubclass<imp::PlacesItem>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PlacesItem {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl PlacesItem {
    pub fn new() -> Self {
        Self::default()
    }
}
