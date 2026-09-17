/*
 * Copyright 2026 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};

use crate::config::LOG_DOMAIN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/mount-item.ui")]
    #[properties(wrapper_type = super::MountItem)]
    pub struct MountItem {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub label: TemplateChild<gtk::Label>,

        #[property(get, set)]
        pub volume: RefCell<Option<gio::Volume>>,

        #[property(get, set)]
        place: RefCell<String>,

        #[property(get, set)]
        gicon: RefCell<Option<gio::Icon>>,

        #[property(get, set)]
        uri: RefCell<String>,

        #[property(get, set = Self::set_mounted)]
        mounted: Cell<bool>,

        // Where the volume is mounted
        #[property(get, set)]
        location: RefCell<Option<gio::File>>,

        #[property(get, set)]
        busy: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MountItem {
        const NAME: &'static str = "PfsMountItem";
        type Type = super::MountItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl MountItem {
        fn set_mounted(&self, mounted: bool) {
            self.mounted.replace(mounted);

            if mounted {
                self.obj().remove_css_class("dim-label");
            } else {
                self.obj().add_css_class("dim-label");
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MountItem {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MountItem {}
    impl BinImpl for MountItem {}
}

glib::wrapper! {
    pub struct MountItem(ObjectSubclass<imp::MountItem>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for MountItem {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl MountItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mount<F, M>(&self, mount_operation: Option<&M>, on_mounted: F)
    where
        M: IsA<gio::MountOperation>,
        F: FnOnce(String) + 'static,
    {
        let volume = {
            let volume = self.imp().volume.borrow();

            let Some(volume) = volume.as_ref() else {
                glib::g_warning!(LOG_DOMAIN, "No volume in {}", self.place());
                return;
            };
            volume.clone()
        };

        self.set_mounted(true);
        volume.clone().mount(
            gio::MountMountFlags::NONE,
            mount_operation,
            None::<&gio::Cancellable>,
            move |result| match result {
                Ok(()) => {
                    if let Some(mount) = volume.get_mount() {
                        let loc = mount.default_location();
                        on_mounted(loc.uri().into());
                    } else {
                        glib::g_warning!(LOG_DOMAIN, "Volume mounted but no mount was found");
                    }
                }
                Err(error) => {
                    glib::g_warning!(LOG_DOMAIN, "Failed to mount volume: {}", error);
                }
            },
        );
    }
}
