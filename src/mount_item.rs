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

        #[template_child]
        pub context_menu: TemplateChild<gtk::PopoverMenu>,

        #[template_child]
        pub context_menu_model: TemplateChild<gio::Menu>,

        #[property(get, set)]
        pub volume: RefCell<Option<gio::Volume>>,

        #[property(get, set)]
        pub gmount: RefCell<Option<gio::Mount>>,

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

        #[property(get, set)]
        local: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MountItem {
        const NAME: &'static str = "PfsMountItem";
        type Type = super::MountItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("mount-item.connect", None, move |item, _, _| {
                let _ = item.activate_action("mounts-box.mount", Some(&item.uri().to_variant()));
            });
            klass.install_action("mount-item.disconnect", None, move |item, _, _| {
                let _ = item.activate_action("mounts-box.unmount", Some(&item.uri().to_variant()));
            });
            klass.install_action("mount-item.mount", None, move |item, _, _| {
                let _ = item.activate_action("mounts-box.mount", Some(&item.uri().to_variant()));
            });
            klass.install_action("mount-item.unmount", None, move |item, _, _| {
                let _ = item.activate_action("mounts-box.unmount", Some(&item.uri().to_variant()));
            });
            klass.install_action("mount-item.eject", None, move |item, _, _| {
                let _ = item.activate_action("mounts-box.eject", Some(&item.uri().to_variant()));
            });
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

        fn dispose(&self) {
            if self.context_menu.parent().is_some() {
                self.context_menu.unparent();
            }
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

        volume.clone().mount(
            gio::MountMountFlags::NONE,
            mount_operation,
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |result| match result {
                    Ok(()) => {
                        if let Some(mount) = volume.get_mount() {
                            let loc = mount.default_location();
                            this.set_mounted(true);
                            on_mounted(loc.uri().into());
                        } else {
                            glib::g_warning!(LOG_DOMAIN, "Volume mounted but no mount was found");
                        }
                    }
                    Err(error) => {
                        glib::g_warning!(LOG_DOMAIN, "Failed to mount volume: {}", error);
                    }
                }
            ),
        );
    }

    pub fn unmount<F, M>(&self, mount_operation: Option<&M>, on_unmounted: F)
    where
        M: IsA<gio::MountOperation>,
        F: FnOnce(String) + 'static,
    {
        if let Some(mount) = self.gmount() {
            mount.clone().unmount_with_operation(
                gio::MountUnmountFlags::NONE,
                mount_operation,
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |result| match result {
                        Ok(()) => {
                            glib::g_debug!(LOG_DOMAIN, "Mount {} umounted", this.place());
                            on_unmounted(this.uri());
                        }
                        Err(error) => {
                            glib::g_warning!(LOG_DOMAIN, "Failed to unmount mount: {}", error);
                        }
                    }
                ),
            );
        } else {
            glib::g_critical!(LOG_DOMAIN, "No mount to unmount: {}", self.place());
        }
    }

    pub fn eject<F, M>(&self, mount_operation: Option<&M>, on_unmounted: F)
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

        if !volume.can_eject() {
            glib::g_critical!(LOG_DOMAIN, "Volume can not be ejected {}", self.place());
            return;
        }

        volume.clone().eject_with_operation(
            gio::MountUnmountFlags::NONE,
            mount_operation,
            None::<&gio::Cancellable>,
            glib::clone!(
                #[strong(rename_to = this)]
                self,
                move |result| match result {
                    Ok(()) => {
                        glib::g_debug!(LOG_DOMAIN, "Volume {} ejected", this.place());
                        on_unmounted(this.uri());
                    }
                    Err(error) => {
                        glib::g_warning!(LOG_DOMAIN, "Failed to eject volume: {}", error);
                    }
                }
            ),
        );
    }

    fn show_context_menu(&self, x: f64, y: f64) {
        let imp = self.imp();
        let mut has_item = false;

        for action in [
            "mount-item.mount",
            "mount-item.unmount",
            "mount-item.eject",
            "mount-item.connect",
            "mount-item.disconnect",
        ] {
            self.action_set_enabled(action, false);
        }

        if self.local() {
            if self.mounted() {
                if self.volume().expect("No volume").can_eject() {
                    self.action_set_enabled("mount-item.eject", true);
                    has_item = true;
                } else if let Some(mount) = self.gmount() {
                    if mount.can_unmount() {
                        self.action_set_enabled("mount-item.unmount", true);
                        has_item = true;
                    }
                } else {
                    glib::g_critical!(
                        LOG_DOMAIN,
                        "Mounted but can't find mount in {}",
                        self.place()
                    );
                }
            } else {
                self.action_set_enabled("mount-item.mount", true);
                has_item = true;
            }
        } else {
            if self.mounted() {
                if let Some(mount) = self.gmount() {
                    if mount.can_unmount() {
                        self.action_set_enabled("mount-item.unmount", true);
                        has_item = true;
                    }
                } else {
                    glib::g_critical!(
                        LOG_DOMAIN,
                        "Mounted but can't find mount in {}",
                        self.place()
                    );
                }
            } else {
                self.action_set_enabled("mount-item.connect", true);
                has_item = true;
            }
        }

        if !has_item {
            return;
        }

        let popover = &imp.context_menu;
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
