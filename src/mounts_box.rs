/*
 * Copyright 2026 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::Signal;
use glib::translate::*;
use glib::Object;
use glib::Properties;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::{config::LOG_DOMAIN, mount_item::MountItem};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/mounts-box.ui")]
    #[properties(wrapper_type = super::MountsBox)]
    pub struct MountsBox {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub flow_box: TemplateChild<gtk::FlowBox>,

        #[property(get = Self::get_available, explicit_notify)]
        pub available: Cell<bool>,

        pub volume_monitor: RefCell<Option<gio::VolumeMonitor>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MountsBox {
        const NAME: &'static str = "PfsMountsBox";
        type Type = super::MountsBox;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MountsBox {
        fn constructed(&self) {
            self.parent_constructed();

            let update_mounts = || {
                let this = self.downgrade();

                move || {
                    if let Some(this) = this.upgrade() {
                        this.obj().update_mounts();
                    }
                }
            };

            let volume_monitor = gio::VolumeMonitor::get();
            volume_monitor.connect_volume_removed({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_volume_added({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_volume_changed({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_mount_removed({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_mount_added({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_mount_changed({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_drive_disconnected({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_drive_connected({
                let update = update_mounts();
                move |_, _| update()
            });
            volume_monitor.connect_drive_changed({
                let update = update_mounts();
                move |_, _| update()
            });

            *self.volume_monitor.borrow_mut() = Some(volume_monitor);
            self.obj().update_mounts();
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

    impl WidgetImpl for MountsBox {}
    impl BinImpl for MountsBox {}

    impl MountsBox {
        fn get_available(&self) -> bool {
            self.available.get()
        }
    }
}

glib::wrapper! {
    pub struct MountsBox(ObjectSubclass<imp::MountsBox>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for MountsBox {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl MountsBox {
    pub fn new() -> Self {
        Self::default()
    }

    #[template_callback]
    fn on_item_activated(&self, flowboxchild: gtk::FlowBoxChild) {
        let object = flowboxchild.child().unwrap();
        let item = object.downcast_ref::<MountItem>().unwrap();

        let uri: String = item.uri();
        glib::g_debug!(LOG_DOMAIN, "Should open mount {uri:#?}");

        if item.mounted() {
            self.emit_by_name::<()>("new-uri", &[&uri]);
        } else {
            let toplevel = self.root().unwrap();
            let mount_operation = gtk::MountOperation::new(toplevel.downcast_ref::<gtk::Window>());

            item.mount(
                Some(&mount_operation),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |uri| {
                        glib::g_debug!(LOG_DOMAIN, "Mounted URI: {uri}");
                        this.emit_by_name::<()>("new-uri", &[&uri]);
                    }
                ),
            );
        }
    }

    fn update_mounts(&self) {
        let imp = self.imp();
        let volume_monitor = imp.volume_monitor.borrow();
        let volume_monitor = volume_monitor
            .as_ref()
            .expect("Volume monitor is not initialized");
        let mut available = false;

        glib::g_debug!(LOG_DOMAIN, "Refreshing mounts…");
        imp.flow_box.remove_all();

        let drives = volume_monitor.connected_drives();
        for drive in drives.iter() {
            let volumes = drive.volumes();
            if !volumes.is_empty() {
                for volume in volumes.iter() {
                    let id = volume.identifier("class");
                    let name = volume.name();

                    // TODO: add item
                    if id != Some("network".into()) {
                        glib::g_debug!(LOG_DOMAIN, "'{name}' is a unhandled volume");
                        continue;
                    }
                    glib::g_warning!(LOG_DOMAIN, "'{name}' is a unhandled network volume");
                }
            } else {
                let name = drive.name();

                if drive.is_removable() {
                    // TODO: add item
                    glib::g_warning!(LOG_DOMAIN, "'{name}' is a unhandled removeable drive");
                } else {
                    glib::g_debug!(LOG_DOMAIN, "'{name}' has no usable volumes");
                }
            }
        }

        let volumes = volume_monitor.volumes();
        for volume in volumes.iter() {
            let id = volume.identifier("class");
            let name = volume.name();

            if volume.drive().is_some() {
                glib::g_debug!(LOG_DOMAIN, "'{name}' has drive, ignoring");
                continue;
            }

            if id != Some("network".into()) {
                glib::g_debug!(LOG_DOMAIN, "'{name}' is not a network volume");
                continue;
            }

            glib::g_debug!(LOG_DOMAIN, "'{name}' is a network volume");
            let m = volume.get_mount();
            let item = Object::builder::<MountItem>().build();

            if let Some(mount) = m {
                // mounted, data from mount
                item.set_place(mount.name());
                item.set_gicon(mount.symbolic_icon());
                item.set_mounted(true);
                let loc = mount.default_location();
                let uri = loc.uri();
                glib::g_debug!(LOG_DOMAIN, "'{name}' is mounted at {uri}");
                item.set_uri(uri);
            } else {
                // not mounted, data from volume
                item.set_place(volume.name());
                item.set_gicon(volume.symbolic_icon());
                item.set_mounted(false);
                glib::g_debug!(LOG_DOMAIN, "'{name}' is not mounted");
                // Keep volume around for mounting
                item.set_volume(volume);
            };

            imp.flow_box.append(&item);
            available = true;
        }

        let child_name = if available { "flow_box" } else { "placeholder" };
        imp.stack.set_visible_child_name(child_name);

        if self.available() != available {
            imp.available.set(true);
            self.notify_available();
        }
    }
}
