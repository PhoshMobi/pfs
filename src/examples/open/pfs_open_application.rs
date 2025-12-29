/*
 * Copyright 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib_macros::clone;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::process::Command;

use pfs::file_props::FileProps;
use pfs::file_selector::{FileSelector, FileSelectorMode};

use crate::config::LOG_DOMAIN;

const FILE_MANAGER1_NAME: &str = "org.freedesktop.FileManager1";
const FILE_MANAGER1_XML: &str = r#"
<node>
  <interface name='org.freedesktop.FileManager1'>
     <method name='ShowFolders'>
      <arg type='as' name='URIs' direction='in'/>
      <arg type='s' name='StartupId' direction='in'/>
    </method>
    <method name='ShowItems'>
      <arg type='as' name='URIs' direction='in'/>
      <arg type='s' name='StartupId' direction='in'/>
    </method>
    <method name='ShowItemProperties'>
      <arg type='as' name='URIs' direction='in'/>
      <arg type='s' name='StartupId' direction='in'/>
    </method>
  </interface>
</node>
"#;

#[derive(Debug, glib::Variant)]
struct ShowFolders {
    uris: Vec<String>,
    _startup_id: String,
}

#[derive(Debug, glib::Variant)]
struct ShowItems {
    uris: Vec<String>,
    _startup_id: String,
}

#[derive(Debug, glib::Variant)]
struct ShowItemProperties {
    uris: Vec<String>,
    _startup_id: String,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
enum FileManager1 {
    ShowFolders(ShowFolders),
    ShowItems(ShowItems),
    ShowItemProperties(ShowItemProperties),
}

mod imp {
    use super::*;

    impl DBusMethodCall for FileManager1 {
        fn parse_call(
            _obj_path: &str,
            _interface: Option<&str>,
            method: &str,
            params: glib::Variant,
        ) -> Result<Self, glib::Error> {
            match method {
                "ShowFolders" => Ok(params.get::<ShowFolders>().map(Self::ShowFolders)),
                "ShowItems" => Ok(params.get::<ShowItems>().map(Self::ShowItems)),
                "ShowItemProperties" => Ok(params
                    .get::<ShowItemProperties>()
                    .map(Self::ShowItemProperties)),
                _ => Err(glib::Error::new(
                    gio::DBusError::UnknownMethod,
                    "No such method",
                )),
            }
            .and_then(|p| {
                p.ok_or_else(|| glib::Error::new(gio::DBusError::InvalidArgs, "Invalid parameters"))
            })
        }
    }

    #[derive(Debug, Default)]
    pub struct PfsOpenApplication {
        pub hold_guard: RefCell<Option<gio::ApplicationHoldGuard>>,
        pub hold_count: Cell<u32>,
        registration_id: RefCell<Option<gio::RegistrationId>>,
        owner_id: RefCell<Option<gio::OwnerId>>,
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

        fn dbus_register(
            &self,
            connection: &gio::DBusConnection,
            object_path: &str,
        ) -> Result<(), glib::Error> {
            self.parent_dbus_register(connection, object_path)?;

            if let Ok(id) = self.obj().register_object(connection) {
                glib::g_debug!(LOG_DOMAIN, "Exported FileManager1 DBus interface");
                self.registration_id.replace(Some(id));
            } else {
                glib::g_warning!(LOG_DOMAIN, "Failed to export FileManager1 DBus interface");
            }

            let id = gio::bus_own_name_on_connection(
                connection,
                FILE_MANAGER1_NAME,
                gio::BusNameOwnerFlags::REPLACE | gio::BusNameOwnerFlags::ALLOW_REPLACEMENT,
                |_connection, name| {
                    glib::g_debug!(LOG_DOMAIN, "Owned {name} DBus name");
                },
                clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |_connection, name| {
                        glib::g_warning!(LOG_DOMAIN, "Lost {name} DBus name");
                        this.owner_id.replace(None);
                    }
                ),
            );

            self.owner_id.replace(Some(id));
            Ok(())
        }

        fn dbus_unregister(&self, connection: &gio::DBusConnection, object_path: &str) {
            self.parent_dbus_unregister(connection, object_path);
            if let Some(id) = self.registration_id.take() {
                if connection.unregister_object(id).is_ok() {
                    glib::g_debug!(LOG_DOMAIN, "Unregistered object");
                } else {
                    glib::g_warning!(LOG_DOMAIN, "Could not unregister object");
                }
            }

            if let Some(owner_id) = self.owner_id.replace(None) {
                gio::bus_unown_name(owner_id);
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

    fn app_release(&self) {
        let imp = self.imp();

        if imp.hold_count.replace(imp.hold_count.get() - 1) == 1 {
            // Drop the gapplication ref count
            self.imp().hold_guard.replace(None);
        }
    }

    fn app_hold(&self) {
        let imp = self.imp();

        if imp.hold_count.replace(imp.hold_count.get() + 1) == 0 {
            // Bump the gapplication ref count
            *self.imp().hold_guard.borrow_mut() = Some(self.hold());
        }
    }

    fn show_open_error(&self, parent: &FileSelector, err_msg: &str) {
        self.app_hold();

        let dialog = adw::AlertDialog::new(
            Some(&gettextrs::gettext("Error opening file")),
            Some(err_msg),
        );

        dialog.add_response("ok", &gettextrs::gettext("Ok"));
        dialog.choose(
            Some(parent),
            None::<&gio::Cancellable>,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |_response| {
                    this.app_release();
                }
            ),
        );
    }

    fn spawn_gio(&self, uri: &str, parent: &FileSelector) -> bool {
        let result = Command::new("gio").arg("open").arg(uri).status();

        if let Ok(result) = result {
            if result.success() {
                return true;
            }
        }

        let msg = &gettextrs::gettext("Failed open {}").replacen("{}", uri, 1);
        self.show_open_error(parent, msg);
        false
    }

    fn open_directory(&self, dir: &gio::File) -> FileSelector {
        let uri = dir.uri();

        glib::g_message!(LOG_DOMAIN, "Opening {uri}");

        self.app_hold();

        let file_selector = glib::Object::builder::<FileSelector>()
            .property("accept_label", gettextrs::gettext("Open"))
            .property("title", gettextrs::gettext("Select a File"))
            .property("current-folder", dir)
            .property("close-on-done", false)
            .build();

        file_selector.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                self,
                move |selector: FileSelector, success: bool| {
                    glib::g_debug!(LOG_DOMAIN, "File dialog done, result: {success}");
                    let selected = selector.selected();

                    if success {
                        if let Some(uris) = selected {
                            for uri in &uris {
                                glib::g_message!(LOG_DOMAIN, "Opening {uri}");
                                this.spawn_gio(uri, &selector);
                            }
                        } else {
                            this.show_open_error(&selector, "Nothing selected");
                        }
                    }
                    this.app_release();
                }
            ),
        );

        file_selector.set_mode(FileSelectorMode::OpenFile);
        file_selector.present();

        file_selector
    }

    fn select_item(&self, file: &gio::File) {
        if let Some(parent) = file.parent() {
            let file_selector = self.open_directory(&parent);
            file_selector.select_item(file);
        }
    }

    fn show_item_properties(&self, file: &gio::File) {
        let uri = file.uri();

        glib::g_message!(LOG_DOMAIN, "Showing props for {uri}");

        self.app_hold();

        let file_props = glib::Object::builder::<FileProps>()
            .property("file", file)
            .build();

        file_props.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                self,
                move |_props: FileProps, success: bool| {
                    glib::g_debug!(LOG_DOMAIN, "File props dialog done, result: {success}");

                    this.app_release();
                }
            ),
        );

        file_props.present();
    }

    fn register_object(
        &self,
        connection: &gio::DBusConnection,
    ) -> Result<gio::RegistrationId, glib::Error> {
        let file_manager1 = gio::DBusNodeInfo::for_xml(FILE_MANAGER1_XML)
            .ok()
            .and_then(|e| e.lookup_interface("org.freedesktop.FileManager1"))
            .expect("FileManagaer1 interface");

        connection
            .register_object("/org/freedesktop/FileManager1", &file_manager1)
            .typed_method_call::<FileManager1>()
            .invoke_and_return_future_local(glib::clone!(
                #[weak_allow_none(rename_to = this)]
                self.imp(),
                move |_, sender, call| {
                    glib::g_message!(LOG_DOMAIN, "Method call from {sender:?}");
                    let app = this.clone();
                    async move {
                        match call {
                            FileManager1::ShowFolders(ShowFolders { uris, _startup_id }) => {
                                if let Some(app) = app {
                                    for uri in &uris {
                                        app.obj().open_directory(&gio::File::for_uri(uri));
                                    }
                                }
                                Ok(None)
                            }
                            FileManager1::ShowItems(ShowItems { uris, _startup_id }) => {
                                if let Some(app) = app {
                                    for uri in &uris {
                                        app.obj().select_item(&gio::File::for_uri(uri));
                                    }
                                }
                                Ok(None)
                            }
                            FileManager1::ShowItemProperties(ShowItemProperties {
                                uris,
                                _startup_id,
                            }) => {
                                if let Some(app) = app {
                                    for uri in &uris {
                                        app.obj().show_item_properties(&gio::File::for_uri(uri));
                                    }
                                }
                                Ok(None)
                            }
                        }
                    }
                }
            ))
            .build()
    }
}
