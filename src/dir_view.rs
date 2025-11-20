/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::Signal;
use glib_macros::{clone, Properties};
use gtk::{gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::{config::LOG_DOMAIN, file_selector::SortMode, grid_item::GridItem, util};

#[derive(Debug, Copy, Clone, Default, PartialEq, gio::glib::Enum)]
#[enum_type(name = "PfsDirViewThumbnailMode")]
pub enum ThumbnailMode {
    #[default]
    Never,
    Local,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, gio::glib::Enum)]
#[enum_type(name = "PfsDirViewDisplayMode")]
pub enum DisplayMode {
    #[default]
    Content, // folder content is displayed
    Search,  // search results are displayed
    Loading, // folder content is loading
}

// Used to create thumbnails, optional, so any code using it should be fail-safe.
const THUMBNAILER_NAME: &str = "mobi.phosh.Thumbnailer";
const THUMBNAILER_PATH: &str = "/mobi/phosh/Thumbnailer";
const THUMBNAILER_IFACE: &str = "mobi.phosh.Thumbnailer";

// We will store the files without thumbnail in a map.
// Once we get no more files for these seconds, then we will send them for thumbnailing.
const THUMBNAILS_DEBOUNCE_SECS: u32 = 1;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/dir-view.ui")]
    #[properties(wrapper_type = super::DirView)]
    pub struct DirView {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,

        #[template_child]
        pub view_stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub directory_list: TemplateChild<gtk::DirectoryList>,

        #[template_child]
        pub sorted_list: TemplateChild<gtk::SortListModel>,

        #[template_child]
        pub filtered_list: TemplateChild<gtk::FilterListModel>,

        #[template_child]
        pub single_selection: TemplateChild<gtk::SingleSelection>,

        #[template_child]
        pub item_factory: TemplateChild<gtk::SignalListItemFactory>,

        // The folder to display
        #[property(get, set = Self::set_folder, explicit_notify)]
        folder: RefCell<Option<gio::File>>,

        // `true` if there's a selected item
        #[property(get, explicit_notify)]
        pub(super) has_selection: Cell<bool>,

        #[property(get, builder(DisplayMode::default()))]
        pub display_mode: Cell<DisplayMode>,

        // The current search term (if any)
        #[property(get, set = Self::set_search_term, explicit_notify)]
        pub(super) search_term: RefCell<Option<String>>,

        // Icon size of the items in the grid view
        #[property(get, set)]
        icon_size: Cell<u32>,

        // What to sort for
        #[property(get, set = Self::set_sort_mode, builder(SortMode::default()))]
        pub sort_mode: RefCell<SortMode>,

        // Whether sort is reversed
        #[property(get, set = Self::set_reversed, explicit_notify)]
        pub(super) reversed: Cell<bool>,

        // Whether to sort directories before files
        #[property(get, set)]
        pub(super) directories_first: Cell<bool>,

        // Whether to show hidden files
        #[property(get, set, set = Self::set_show_hidden, explicit_notify)]
        pub(super) show_hidden: Cell<bool>,

        // Whether to select a directory rather than a file
        #[property(get, set = Self::set_directories_only, explicit_notify)]
        pub(super) directories_only: Cell<bool>,

        // The current filter type filter
        #[property(get, set = Self::set_type_filter, construct, nullable, explicit_notify)]
        pub(super) type_filter: RefCell<Option<gtk::FileFilter>>,

        // The current filter type filter plus directories
        #[property(get, set = Self::set_type_filter, nullable, explicit_notify)]
        pub(super) real_filter: RefCell<Option<gtk::FileFilter>>,

        // Whether to show thumbnails
        #[property(get, set, builder(ThumbnailMode::default()))]
        pub thumbnail_mode: RefCell<ThumbnailMode>,

        pub cancellable: RefCell<gio::Cancellable>,
        pub debounce_id: RefCell<Option<glib::SourceId>>,
        pub no_thumbnails: RefCell<HashMap<String, GridItem>>,
        pub thumbnailer_proxy: RefCell<Option<gio::DBusProxy>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DirView {
        const NAME: &'static str = "PfsDirView";
        type Type = super::DirView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl DirView {
        // r/o property
        pub(super) fn set_has_selection(&self, has_selection: bool) {
            if has_selection == self.has_selection.get() {
                return;
            }

            self.has_selection.replace(has_selection);
            self.obj().notify_has_selection();
        }

        fn update_directory_selection(&self) {
            // In directory selection mode we have a selection whenever
            // we're in a valid dir (e.g. not in recent:///
            if !self.directories_only.get() {
                return;
            }

            let has_selection = util::is_valid_folder(&self.folder.borrow());
            self.set_has_selection(has_selection);
        }

        fn set_folder(&self, folder: Option<gio::File>) {
            let obj = self.obj();
            let oldfolder = self.folder.borrow().clone();

            if folder.is_none() {
                return;
            }

            let folder = folder.unwrap();
            if oldfolder.is_some() && folder.equal(&oldfolder.unwrap()) {
                return;
            }

            let uri = folder.uri();
            glib::g_debug!(LOG_DOMAIN, "Loading folder for {uri:#?}");

            self.no_thumbnails.borrow_mut().clear();

            *self.folder.borrow_mut() = Some(folder);
            obj.notify_folder();

            self.update_directory_selection();
        }

        fn set_show_hidden(&self, show_hidden: bool) {
            let obj = self.obj();

            if self.show_hidden.get() == show_hidden {
                return;
            }

            glib::g_debug!(LOG_DOMAIN, "show_hidden {show_hidden:#?}");

            self.show_hidden.replace(show_hidden);
            obj.notify_show_hidden();

            // Refilter
            let filter = self.filtered_list.filter().unwrap();
            let strict = if show_hidden {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::MoreStrict
            };
            filter.emit_by_name::<()>("changed", &[&strict]);
        }

        fn set_sort_mode(&self, mode: SortMode) {
            if *self.sort_mode.borrow() == mode {
                return;
            }

            let reversed = self.reversed.get();
            self.obj().set_sorting(mode, reversed);
        }

        fn set_reversed(&self, reversed: bool) {
            if self.reversed.get() == reversed {
                return;
            }

            let mode = *self.sort_mode.borrow();
            self.obj().set_sorting(mode, reversed);
        }

        fn set_directories_only(&self, directories_only: bool) {
            let obj = self.obj();

            if self.directories_only.get() == directories_only {
                return;
            }

            glib::g_debug!(LOG_DOMAIN, "directories_only {directories_only:#?}");

            self.directories_only.replace(directories_only);

            // Refilter
            let filter = self.filtered_list.filter().unwrap();
            let strict = if directories_only {
                gtk::FilterChange::MoreStrict
            } else {
                gtk::FilterChange::LessStrict
            };
            filter.emit_by_name::<()>("changed", &[&strict]);

            obj.notify_directories_only();
            self.update_directory_selection();
        }

        fn set_type_filter(&self, type_filter: Option<gtk::FileFilter>) {
            let obj = self.obj();

            if *self.type_filter.borrow() == type_filter {
                return;
            }

            // Ensure directories are always included in the filter so users can browse
            // through them. We don't modify the passed in filter as the user might read
            // it back
            if type_filter.is_some() {
                let filter = type_filter.clone().unwrap();
                let real_filter = gtk::FileFilter::from_gvariant(&filter.to_gvariant());
                real_filter.add_mime_type("inode/directory");

                let name = real_filter.name().unwrap_or_default();
                glib::g_debug!(LOG_DOMAIN, "Setting file filter to {name:#?}");
                *self.real_filter.borrow_mut() = Some(real_filter);
            } else {
                *self.real_filter.borrow_mut() = None;
                glib::g_debug!(LOG_DOMAIN, "Setting file filter to None");
            }

            *self.type_filter.borrow_mut() = type_filter;
            obj.notify_type_filter();
            obj.notify_real_filter();
        }

        fn set_search_term(&self, search_term: Option<String>) {
            let strict;
            let obj = self.obj();
            let mut new_term: Option<String> = None;

            {
                if let Some(term) = &search_term {
                    new_term = Some(term.trim().to_lowercase());
                }

                // old_term only borrowed in this block
                let old_term = self.search_term.borrow();
                if *old_term == new_term {
                    return;
                }

                #[allow(clippy::unnecessary_unwrap)]
                if old_term.is_none() || new_term.is_none() {
                    strict = gtk::FilterChange::Different;
                } else if old_term
                    .as_ref()
                    .unwrap()
                    .starts_with(new_term.as_ref().unwrap())
                {
                    strict = gtk::FilterChange::LessStrict;
                } else if new_term
                    .as_ref()
                    .unwrap()
                    .starts_with(old_term.as_ref().unwrap())
                {
                    strict = gtk::FilterChange::MoreStrict;
                } else {
                    strict = gtk::FilterChange::Different;
                }
            }

            let mode = if new_term.is_some() && !new_term.as_ref().unwrap().is_empty() {
                DisplayMode::Search
            } else {
                DisplayMode::Content
            };
            if self.display_mode.get() != mode {
                self.display_mode.replace(mode);
                obj.notify_display_mode();
            }

            *self.search_term.borrow_mut() = new_term;

            let filter = self.filtered_list.filter().unwrap();
            filter.emit_by_name::<()>("changed", &[&strict]);
            obj.notify_search_term();
        }

        fn on_thumbnail_files_ready(
            &self,
            result: std::result::Result<glib::Variant, glib::Error>,
        ) {
            if result.is_ok() {
                return;
            }

            let error = result.err().unwrap();
            if let Some(dbus_error) = error.kind::<gio::DBusError>() {
                if dbus_error != gio::DBusError::ServiceUnknown {
                    glib::g_warning!(LOG_DOMAIN, "ThumbnailFiles failed: {error}");
                }
            }
        }

        pub fn send_for_thumbnailing(&self) {
            let proxy = self.thumbnailer_proxy.borrow();
            let Some(ref proxy) = *proxy else {
                return;
            };

            let files: Vec<String> = self.no_thumbnails.borrow().keys().cloned().collect();
            let options: HashMap<&str, glib::Variant> = HashMap::new();
            let params = (files, options).to_variant();
            proxy.call(
                "ThumbnailFiles",
                Some(&params),
                gio::DBusCallFlags::NONE,
                -1,
                Some(&*self.cancellable.borrow()),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |result: std::result::Result<glib::Variant, glib::Error>| this
                        .on_thumbnail_files_ready(result)
                ),
            );
        }

        fn on_thumbnailing_done(&self, params: glib::Variant) {
            let (thumbnails, _options) = <(
                HashMap<String, glib::Variant>,
                HashMap<String, glib::Variant>,
            )>::from_variant(&params)
            .unwrap_or_default();
            let mut no_thumbnails = self.no_thumbnails.borrow_mut();

            for (file_uri, value_var) in &thumbnails {
                if let Some(item) = no_thumbnails.remove(file_uri) {
                    if let Some(path) = String::from_variant(value_var) {
                        item.set_thumbnail(path);
                    }
                }
            }
        }

        fn on_proxy_ready(&self, result: std::result::Result<gio::DBusProxy, glib::Error>) {
            match result {
                Ok(proxy) => {
                    proxy.connect_closure(
                        "g-signal::ThumbnailingDone",
                        false,
                        glib::closure_local!(
                            #[weak(rename_to = this)]
                            self,
                            move |_: &gio::DBusProxy,
                                  _: String,
                                  _: String,
                                  params: glib::Variant| this
                                .on_thumbnailing_done(params)
                        ),
                    );
                    *self.thumbnailer_proxy.borrow_mut() = Some(proxy);
                }
                Err(error) => {
                    glib::g_message!(LOG_DOMAIN, "Failed to load thumbnailer: {error}");
                }
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DirView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            *self.cancellable.borrow_mut() = gio::Cancellable::new();

            gio::DBusProxy::for_bus(
                gio::BusType::Session,
                gio::DBusProxyFlags::NONE,
                None,
                THUMBNAILER_NAME,
                THUMBNAILER_PATH,
                THUMBNAILER_IFACE,
                Some(&*self.cancellable.borrow()),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |result: std::result::Result<gio::DBusProxy, glib::Error>| this
                        .on_proxy_ready(result)
                ),
            );

            obj.setup_gsettings();
            obj.set_directories_first(true);
            obj.setup_sort_and_filter();
            obj.on_n_items_changed();

            obj.bind_property("folder", &self.directory_list.get(), "file")
                .sync_create()
                .build();
        }

        fn dispose(&self) {
            self.cancellable.borrow().cancel();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("new-uri")
                        .param_types([String::static_type()])
                        .build(),
                    // The UI should consider updating the displayed
                    // filename
                    Signal::builder("new-filename")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for DirView {}
    impl BinImpl for DirView {}
}

glib::wrapper! {
    pub struct DirView(ObjectSubclass<imp::DirView>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for DirView {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl DirView {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_directory(&self, fileinfo: &gio::FileInfo) -> bool {
        let content_type = fileinfo.content_type().unwrap_or_default();

        if content_type == "inode/directory" {
            return true;
        }
        false
    }

    #[template_callback]
    fn on_item_setup(&self, object: glib::Object) {
        let list_item = object.downcast_ref::<gtk::ListItem>().unwrap();
        let grid_item = GridItem::new();

        self.bind_property("icon-size", &grid_item, "icon-size")
            .sync_create()
            .build();

        self.bind_property("thumbnail-mode", &grid_item, "thumbnail-mode")
            .sync_create()
            .build();

        list_item.set_child(Some(&grid_item));
    }

    #[template_callback]
    fn on_item_bind(&self, object: glib::Object) {
        let list_item = object.downcast_ref::<gtk::ListItem>().unwrap();
        let item = list_item.item().unwrap();
        let info = item.downcast_ref::<gio::FileInfo>().unwrap();

        let widget = list_item.child().unwrap();
        let grid_item = widget.downcast_ref::<GridItem>().unwrap();

        grid_item.set_fileinfo(info);

        if info.boolean(gio::FILE_ATTRIBUTE_THUMBNAIL_IS_VALID) {
            return;
        }

        let imp = self.imp();

        if let Some(source_id) = imp.debounce_id.take() {
            source_id.remove();
        }

        let mut no_thumbnails = imp.no_thumbnails.borrow_mut();
        let binding = info.attribute_object("standard::file").unwrap();
        let file = binding.downcast_ref::<gio::File>().unwrap();
        no_thumbnails.insert(file.uri().to_string(), grid_item.clone());

        let source_id = glib::source::timeout_add_seconds_local_once(
            THUMBNAILS_DEBOUNCE_SECS,
            glib::clone!(
                #[weak(rename_to = this)]
                imp,
                move || {
                    *this.debounce_id.borrow_mut() = None;
                    this.send_for_thumbnailing();
                }
            ),
        );
        *imp.debounce_id.borrow_mut() = Some(source_id);
    }

    #[template_callback]
    fn on_selection_changed(&self, position: u32, n_items: u32) {
        glib::g_debug!(LOG_DOMAIN, "Selection changed {position:#?} {n_items:#?}");

        let selection = self.imp().single_selection.get();
        let selected_item = selection.selected_item();
        let mut is_selected = false;

        if let Some(info) = selected_item {
            let fileinfo = info.downcast_ref::<gio::FileInfo>().unwrap();
            let object = fileinfo.attribute_object("standard::file").unwrap();
            let file = object.downcast_ref::<gio::File>().unwrap();

            if self.is_directory(fileinfo) {
                let uri = file.uri();

                glib::g_debug!(LOG_DOMAIN, "Should open {uri:#?}");
                self.imp().obj().emit_by_name::<()>("new-uri", &[&uri]);
            } else {
                is_selected = true;
                let filename = file.basename();
                self.imp()
                    .obj()
                    .emit_by_name::<()>("new-filename", &[&filename]);
            }
        }

        if self.directories_only() {
            return;
        }

        self.imp().set_has_selection(is_selected);
    }

    #[template_callback]
    fn on_n_items_changed(&self) {
        let n_items = self.imp().filtered_list.get().n_items();
        let pagename = if n_items > 0 { "folder" } else { "empty" };
        self.imp().view_stack.get().set_visible_child_name(pagename);
    }

    #[template_callback]
    fn on_activate(&self, pos: u32) {
        glib::g_debug!(LOG_DOMAIN, "Item Activated {pos:#?}");

        self.imp().single_selection.set_selected(pos);
        // Only accept when we have a selection
        if !self.has_selection() {
            return;
        }

        let _ = self
            .upcast_ref::<gtk::Widget>()
            .activate_action("file-selector.accept", None);
    }

    #[template_callback]
    fn searching_to_status_page_icon(&self) -> &str {
        match self.display_mode() {
            DisplayMode::Search => "nautilus-folder-search-symbolic",
            DisplayMode::Content | DisplayMode::Loading => "folder-symbolic",
        }
    }

    #[template_callback]
    fn searching_to_status_page_title(&self) -> String {
        match self.display_mode() {
            DisplayMode::Search => gettextrs::gettext("Search is empty"),
            DisplayMode::Content => gettextrs::gettext("Folder is empty"),
            DisplayMode::Loading => gettextrs::gettext("Folder is loading…"),
        }
    }

    #[template_callback]
    fn on_loading_changed(&self) {
        let mode = if self.imp().directory_list.is_loading() {
            DisplayMode::Loading
        } else {
            DisplayMode::Content
        };
        self.imp().display_mode.replace(mode);
        self.imp().obj().notify_display_mode();
    }

    #[template_callback]
    fn loading_to_status_page_spinner(&self) -> bool {
        matches!(self.display_mode(), DisplayMode::Loading)
    }

    pub fn selected(&self) -> Option<Vec<String>> {
        let vec = if self.directories_only() {
            match self.folder().unwrap().path() {
                None => return None,
                Some(_) => vec![self.folder().unwrap().uri().to_string()],
            }
        } else {
            let selected = self.imp().single_selection.get().selected_item();
            let item = selected?;

            let file = item
                .downcast_ref::<gio::FileInfo>()
                .unwrap()
                .attribute_object("standard::file")
                .unwrap();

            let uri = file.downcast_ref::<gio::File>().unwrap().uri();
            glib::g_debug!(LOG_DOMAIN, "Uri {uri:#?}");

            vec![uri.to_string()]
        };
        Some(vec)
    }

    fn sort_by_name(&self, info1: &gio::FileInfo, info2: &gio::FileInfo) -> gtk::Ordering {
        match info1.display_name().cmp(&info2.display_name()) {
            Ordering::Less => {
                if self.imp().reversed.get() {
                    return gtk::Ordering::Larger;
                }
                gtk::Ordering::Smaller
            }
            Ordering::Greater => {
                if self.imp().reversed.get() {
                    return gtk::Ordering::Smaller;
                }
                gtk::Ordering::Larger
            }
            Ordering::Equal => gtk::Ordering::Equal,
        }
    }

    fn sort_by_modification_time(
        &self,
        info1: &gio::FileInfo,
        info2: &gio::FileInfo,
    ) -> gtk::Ordering {
        match info1
            .modification_date_time()
            .cmp(&info2.modification_date_time())
        {
            Ordering::Less => {
                if self.imp().reversed.get() {
                    return gtk::Ordering::Larger;
                }
                gtk::Ordering::Smaller
            }
            Ordering::Greater => {
                if self.imp().reversed.get() {
                    return gtk::Ordering::Smaller;
                }
                gtk::Ordering::Larger
            }
            Ordering::Equal => gtk::Ordering::Equal,
        }
    }

    fn setup_sort_and_filter(&self) {
        let sorter = gtk::CustomSorter::new(clone!(
            #[weak(rename_to = this)]
            self,
            #[upgrade_or]
            gtk::Ordering::Equal,
            move |obj1, obj2| {
                let info1 = obj1
                    .downcast_ref::<gio::FileInfo>()
                    .expect("Should be file info");
                let info2 = obj2
                    .downcast_ref::<gio::FileInfo>()
                    .expect("Should be file info");

                if this.directories_first() {
                    let is_dir1 = this.is_directory(info1);
                    let is_dir2 = this.is_directory(info2);

                    if is_dir1 && !is_dir2 {
                        return gtk::Ordering::Smaller;
                    }

                    if is_dir2 && !is_dir1 {
                        return gtk::Ordering::Larger;
                    }
                }

                let mode = *this.imp().sort_mode.borrow();
                match mode {
                    SortMode::DisplayName => this.sort_by_name(info1, info2),
                    SortMode::ModificationTime => this.sort_by_modification_time(info1, info2),
                }
            }
        ));
        self.imp().sorted_list.set_sorter(Some(&sorter));

        let custom_filter = gtk::CustomFilter::new(clone!(
            #[weak(rename_to = this)]
            self,
            #[upgrade_or]
            true,
            move |obj| {
                let info = obj
                    .downcast_ref::<gio::FileInfo>()
                    .expect("Should be file info");
                let search_term = this.imp().search_term.borrow();

                if search_term.is_some()
                    && !info
                        .display_name()
                        .trim()
                        .to_lowercase()
                        .starts_with(search_term.as_ref().unwrap())
                {
                    return false;
                }

                if this.imp().directories_only.get() && !this.is_directory(info) {
                    return false;
                }

                if this.imp().show_hidden.get() {
                    return true;
                }

                if info.display_name().starts_with('.') {
                    return false;
                }
                return true;
            }
        ));
        self.imp().filtered_list.set_filter(Some(&custom_filter));
    }

    fn setup_gsettings(&self) {
        if !util::is_schema_installed() {
            glib::g_debug!(
                LOG_DOMAIN,
                "Not binding to settings as schema is not available"
            );
            self.set_icon_size(96);
            self.set_thumbnail_mode(ThumbnailMode::Local);
            return;
        }

        let settings = gio::Settings::new("mobi.phosh.FileSelector");
        settings.bind("icon-size", self, "icon-size").build();
        settings
            .bind("thumbnail-mode", self, "thumbnail-mode")
            .build();
    }

    pub fn set_sorting(&self, sort_mode: SortMode, reversed: bool) {
        glib::g_debug!(
            LOG_DOMAIN,
            "Sorting mode {sort_mode:#?}, reversed: {reversed:#?}"
        );

        *self.imp().sort_mode.borrow_mut() = sort_mode;
        self.imp().reversed.replace(reversed);

        self.notify_sort_mode();
        self.notify_reversed();

        // Resort
        let sorter = self.imp().sorted_list.sorter().unwrap();
        let change = gtk::SorterChange::Inverted;
        sorter.emit_by_name::<()>("changed", &[&change]);
    }
}
