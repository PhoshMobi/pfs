/*
 * Copyright 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::{prelude::*, subclass::prelude::*};
use glib::subclass::Signal;
use glib::translate::*;
use glib_macros::{clone, Properties};
use gtk::{gdk, gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::{config::LOG_DOMAIN, file_selector::FileSelector, file_selector::FileSelectorMode};

#[derive(Debug, Copy, Clone, Default, PartialEq, gio::glib::Enum)]
#[enum_type(name = "PfsFilePropsType")]
pub enum FilePropsType {
    #[default]
    File,
    Directory,
    //TODO: MountPoint,
}

pub mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/file-props.ui")]
    #[properties(wrapper_type = super::FileProps)]
    pub struct FileProps {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub type_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub size_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub access_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub modified_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub created_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub timestamp_group: TemplateChild<adw::PreferencesGroup>,

        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,

        // The file we show the info for
        #[property(get, set, construct)]
        pub file: RefCell<Option<gio::File>>,

        #[property(get, explicit_notify)]
        pub parent_folder: RefCell<Option<gio::File>>,

        #[property(get, explicit_notify, builder(FilePropsType::default()))]
        pub file_type: RefCell<FilePropsType>,

        done: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileProps {
        const NAME: &'static str = "PfsFileProps";
        type Type = super::FileProps;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::NO_MODIFIER_MASK,
                "window.close",
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FileProps {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_fileinfo();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("done")
                    .param_types([bool::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for FileProps {}
    impl WindowImpl for FileProps {}
    impl AdwWindowImpl for FileProps {}

    impl FileProps {
        pub(super) fn send_done(&self, success: bool, close: bool) {
            if !self.done.get() {
                glib::g_debug!(LOG_DOMAIN, "Done, success: {success}");
                self.obj().emit_by_name::<()>("done", &[&success]);
                self.done.replace(true);
            }

            if close {
                self.obj().upcast_ref::<gtk::Window>().close();
            }
        }
    }
}

glib::wrapper! {
    pub struct FileProps(ObjectSubclass<imp::FileProps>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for FileProps {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl FileProps {
    pub fn new() -> Self {
        Self::default()
    }

    fn update_info(&self, info: &gio::FileInfo) {
        let imp = self.imp();
        let mut have_thumbnail = false;
        let mut have_timestamp = false;

        let size = info.size();
        imp.size_label.set_label(&glib::format_size(size as u64));
        imp.size_label.set_visible(true);

        if let Some(created) = info.creation_date_time() {
            if let Ok(fmt) = created.format_iso8601() {
                imp.created_row.set_subtitle(&fmt);
                imp.created_row.set_visible(true);
                have_timestamp = true;
            }
        }

        if let Some(modified) = info.modification_date_time() {
            if let Ok(fmt) = modified.format_iso8601() {
                imp.modified_row.set_subtitle(&fmt);
                imp.modified_row.set_visible(true);
                have_timestamp = true;
            }
        }

        if let Some(access) = info.access_date_time() {
            if let Ok(fmt) = access.format_iso8601() {
                imp.access_row.set_subtitle(&fmt);
                imp.access_row.set_visible(true);
                have_timestamp = true;
            }
        }

        if have_timestamp {
            imp.timestamp_group.set_visible(true);
        }

        if let Some(content_type) = info.content_type() {
            if content_type == "inode/directory" {
                imp.file_type.replace(FilePropsType::Directory);
                self.notify_file_type();
                imp.type_label.set_label(&gettextrs::gettext("Directory"));
            } else {
                imp.type_label.set_label(&content_type);
            }
        }

        if let Some(path) = info.attribute_byte_string("thumbnail::path") {
            if info.boolean("thumbnail::is-valid") {
                imp.icon.set_from_file(Some(path));
                have_thumbnail = true;
                imp.icon.set_pixel_size(256);
            }
        }

        if !have_thumbnail {
            if let Some(icon) = info.icon() {
                imp.icon.set_from_gicon(&icon);
                imp.icon.set_pixel_size(128);
            }
        }

        if let Some(file) = self.file() {
            if let Some(parent_folder) = file.parent() {
                *imp.parent_folder.borrow_mut() = Some(parent_folder);
            } else {
                *imp.parent_folder.borrow_mut() = None;
            }
            self.notify_parent_folder();
        }
    }

    fn clear_info(&self) {
        let imp = self.imp();
        let unknown = gettextrs::gettext("Unknown");

        imp.size_label.set_label(&unknown);

        imp.size_label.set_visible(false);
        imp.timestamp_group.set_visible(false);
        imp.created_row.set_visible(false);
        imp.modified_row.set_visible(false);
        imp.access_row.set_visible(false);
        imp.type_label.set_label(&unknown);
        imp.icon.set_icon_name(Some("image-missing-symbolic"));
        imp.icon.set_pixel_size(128);
    }

    fn setup_fileinfo(&self) {
        let c = glib::MainContext::default();

        /* TODO: get fileinfo and fill properties with it */
        let Some(file) = self.file() else {
            return;
        };

        self.clear_info();
        let future = clone!(
            #[weak(rename_to = this)]
            self,
            async move {
                match file
                    .query_info_future(
                        &[
                            "standard::content-type",
                            "standard::display-name",
                            "standard::icon",
                            "standard::size",
                            "thumbnail::*",
                            "time::access",
                            "time::created",
                            "time::modified",
                        ]
                        .join(","),
                        gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
                        glib::Priority::DEFAULT,
                    )
                    .await
                {
                    Ok(info) => this.update_info(&info),
                    Err(err) => {
                        let imp = this.imp();

                        let msg = gettextrs::gettext("Failed to get info for {}").replacen(
                            "{}",
                            this.file().unwrap().uri().as_str(),
                            1,
                        );
                        imp.toast_overlay.add_toast(adw::Toast::new(&msg));
                        glib::g_warning!(LOG_DOMAIN, "Failed to get info: {err}");
                    }
                }
            }
        );
        c.spawn_local(future);
    }

    #[template_callback]
    fn on_close_requested(&self) -> bool {
        self.imp().send_done(false, false);
        false
    }

    #[template_callback]
    fn on_accept_clicked(&self) {
        glib::g_debug!(LOG_DOMAIN, "Props done");
        self.imp().send_done(true, true);
    }

    #[template_callback]
    fn file_to_string(&self, file: Option<gio::File>) -> String {
        let Some(file) = file else {
            return "".to_string();
        };

        let basename = file.basename().unwrap_or_default();
        basename.to_str().unwrap_or_default().to_string()
    }

    #[template_callback]
    fn file_type_to_size_label_visible(&self, file_type: FilePropsType) -> bool {
        file_type == FilePropsType::File
    }

    #[template_callback]
    fn parent_folder_to_row_visible(&self) -> bool {
        self.parent_folder().is_some()
    }

    #[template_callback]
    fn on_open_parent_folder_clicked(&self) {
        let file_selector = glib::Object::builder::<FileSelector>()
            .property("accept_label", gettextrs::gettext("Done"))
            .property("title", gettextrs::gettext("Browse Directory"))
            .property("current-folder", self.parent_folder())
            .build();

        file_selector.set_mode(FileSelectorMode::OpenFile);
        file_selector.present();
    }
}
