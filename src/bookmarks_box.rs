/*
 * Copyright 2026 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::Signal;
use glib::translate::*;
use glib::Object;
use glib::Properties;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::{bookmarks_item::BookmarksItem, config::LOG_DOMAIN};

const CONFIG_DIR_NAME: &str = "pfs";
const BOOKMARKS_FILE_NAME: &str = "bookmarks.xbel";

async fn init_bookmarks() -> Option<(PathBuf, glib::BookmarkFile)> {
    let mut config_dir = glib::user_config_dir();
    config_dir.push(CONFIG_DIR_NAME);
    let config_dir_file = gio::File::for_path(&config_dir);

    if let Err(error) = config_dir_file
        .make_directory_future(glib::Priority::DEFAULT)
        .await
    {
        if !error.matches(gio::IOErrorEnum::Exists) {
            glib::g_critical!(
                LOG_DOMAIN,
                "Failed to make configuration directory: {error}"
            );
            return None;
        }
    }

    let Some(bookmarks_file_path) = config_dir_file.child(BOOKMARKS_FILE_NAME).path() else {
        glib::g_critical!(LOG_DOMAIN, "Failed to get path to bookmarks file");
        return None;
    };
    let mut bookmarks_file = glib::BookmarkFile::new();

    if let Err(error) = bookmarks_file.load_from_file(&bookmarks_file_path) {
        if error.matches(glib::FileError::Noent) {
            if let Err(error) = bookmarks_file.to_file(&bookmarks_file_path) {
                glib::g_critical!(LOG_DOMAIN, "Failed to initialize bookmarks file: {error}");
                return None;
            }
        } else {
            glib::g_critical!(LOG_DOMAIN, "Failed to load bookmarks file: {error}");
            return None;
        }
    }

    Some((bookmarks_file_path, bookmarks_file))
}

fn save_bookmarks(bookmarks_file_path: &PathBuf, bookmarks_file: &glib::BookmarkFile) {
    if let Err(error) = bookmarks_file.to_file(bookmarks_file_path) {
        glib::g_critical!(LOG_DOMAIN, "Failed to save bookmarks file: {error}");
    }
}

fn create_widget(object: &Object) -> gtk::Widget {
    let info = object.downcast_ref::<gio::FileInfo>().unwrap();

    let binding = info.attribute_object("standard::file").unwrap();
    let file = binding.downcast_ref::<gio::File>().unwrap();

    let item = Object::builder::<BookmarksItem>()
        .property("place", info.display_name())
        .property("gicon", info.icon())
        .property("uri", file.uri())
        .build();
    item.into()
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/bookmarks-box.ui")]
    #[properties(wrapper_type = super::BookmarksBox)]
    pub struct BookmarksBox {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub flow_box: TemplateChild<gtk::FlowBox>,

        #[property(get = Self::get_available, explicit_notify)]
        pub available: Cell<bool>,

        pub bookmarks_file: RefCell<Option<(PathBuf, glib::BookmarkFile)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BookmarksBox {
        const NAME: &'static str = "PfsBookmarksBox";
        type Type = super::BookmarksBox;
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
    impl ObjectImpl for BookmarksBox {
        fn constructed(&self) {
            self.parent_constructed();

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = this)]
                self,
                async move {
                    let (bookmarks_file_path, bookmarks_file) = match init_bookmarks().await {
                        Some((bookmarks_file_path, bookmarks_file)) => {
                            (bookmarks_file_path, bookmarks_file)
                        }
                        None => return,
                    };
                    this.on_init_bookmarks_ready(bookmarks_file_path, bookmarks_file);
                }
            ));
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

    impl WidgetImpl for BookmarksBox {}
    impl BinImpl for BookmarksBox {}

    impl BookmarksBox {
        fn get_available(&self) -> bool {
            self.available.get()
        }

        fn on_items_changed(&self, model: gio::ListModel) {
            let n_items = model.n_items();
            let page_name = if n_items > 0 {
                "flow_box"
            } else {
                "placeholder"
            };
            self.stack.set_visible_child_name(page_name);
        }

        fn on_init_bookmarks_ready(
            &self,
            bookmarks_file_path: PathBuf,
            bookmarks_file: glib::BookmarkFile,
        ) {
            let model = gtk::BookmarkList::new(
                Some(&bookmarks_file_path),
                Some("standard::display-name,standard::icon"),
            );
            self.flow_box.bind_model(Some(&model), create_widget);

            *self.bookmarks_file.borrow_mut() = Some((bookmarks_file_path, bookmarks_file));
            self.available.set(true);
            self.obj().notify_available();

            model.connect_closure(
                "items-changed",
                false,
                glib::closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |model: gio::ListModel, _: u32, _: u32, _: u32| this
                        .on_items_changed(model)
                ),
            );
            self.on_items_changed(model.upcast::<gio::ListModel>());
        }
    }
}

glib::wrapper! {
    pub struct BookmarksBox(ObjectSubclass<imp::BookmarksBox>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for BookmarksBox {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl BookmarksBox {
    pub fn new() -> Self {
        Self::default()
    }

    #[template_callback]
    fn on_item_activated(&self, flowboxchild: gtk::FlowBoxChild) {
        let object = flowboxchild.child().unwrap();
        let item = object.downcast_ref::<BookmarksItem>().unwrap();

        let uri: String = item.uri();
        glib::g_debug!(LOG_DOMAIN, "Should open bookmark {uri:#?}");
        self.emit_by_name::<()>("new-uri", &[&uri]);
    }

    pub fn add_bookmark(&self, uri: &str) {
        let mut borrow = self.imp().bookmarks_file.borrow_mut();
        let (bookmarks_file_path, bookmarks_file) = borrow.as_mut().unwrap();

        glib::g_debug!(LOG_DOMAIN, "Adding bookmark {uri:#?}");
        bookmarks_file.add_application(uri, None, None);
        save_bookmarks(bookmarks_file_path, bookmarks_file);
    }

    pub fn del_bookmark(&self, uri: &str) {
        let mut borrow = self.imp().bookmarks_file.borrow_mut();
        let (bookmarks_file_path, bookmarks_file) = borrow.as_mut().unwrap();

        glib::g_debug!(LOG_DOMAIN, "Deleting bookmark {uri:#?}");
        bookmarks_file.remove_item(uri).unwrap();
        save_bookmarks(bookmarks_file_path, bookmarks_file);
    }

    pub fn is_bookmark(&self, uri: &str) -> bool {
        let borrow = self.imp().bookmarks_file.borrow();
        let (_, bookmarks_file) = borrow.as_ref().unwrap();

        bookmarks_file.has_item(uri)
    }
}
