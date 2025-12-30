/*
 * Copyright 2024-2025 Phosh.mobi e.V.
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

use crate::{
    config::LOG_DOMAIN, dir_stack::DirStack, dir_view::DirView, places_box::PlacesBox,
    stateful_action, util,
};

#[derive(Debug, Copy, Clone, Default, PartialEq, gio::glib::Enum)]
#[enum_type(name = "PfsFileSelectorMode")]
pub enum FileSelectorMode {
    #[default]
    OpenFile,
    SaveFile,
    SaveFiles,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, gio::glib::Enum)]
#[enum_type(name = "PfsSortMode")]
pub enum SortMode {
    #[default]
    #[enum_value(nick = "name")]
    DisplayName = 0,
    #[enum_value(nick = "mtime")]
    ModificationTime = 1,
}

pub mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/file-selector.ui")]
    #[properties(wrapper_type = super::FileSelector)]
    pub struct FileSelector {
        #[template_child]
        pub dir_view: TemplateChild<DirView>,

        #[template_child]
        pub places_box: TemplateChild<PlacesBox>,

        #[template_child]
        pub dir_stack: TemplateChild<DirStack>,

        #[template_child]
        pub accept_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub window_title: TemplateChild<adw::WindowTitle>,

        #[template_child]
        pub bottom_sheet: TemplateChild<adw::BottomSheet>,

        #[template_child]
        pub choices_menu_button: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        done: Cell<bool>,

        pub(super) choices_actions: RefCell<Option<gio::SimpleActionGroup>>,

        pub(super) settings: RefCell<Option<gio::Settings>>,

        //
        // Properties mapping to the portal spec
        //
        #[property(get, set)]
        pub accept_label: RefCell<String>,

        #[property(get, set)]
        pub title: RefCell<String>,

        // Select directory instead of files
        #[property(get, set)]
        pub directory: Cell<bool>,

        // The filters
        #[property(get, set, construct)]
        pub filters: RefCell<Option<gio::ListModel>>,

        // Position in filters that is currently selected
        #[property(get, set = Self::set_current_filter, explicit_notify,
                   construct, default=gtk::INVALID_LIST_POSITION)]
        pub current_filter: Cell<u32>,

        // The current folder to open
        #[property(get, set)]
        pub current_folder: RefCell<Option<gio::File>>,

        // The file name (basename) when saving a file
        #[property(get, set = Self::set_filename)]
        pub filename: RefCell<String>,

        // Whether this is OpenFile, SaveFile or SaveFiles
        #[property(get, set = Self::set_mode, builder(FileSelectorMode::default()))]
        pub mode: RefCell<FileSelectorMode>,

        // The additional choices to present
        #[property(get, set = Self::set_choices, builder(glib::VariantTy::ARRAY))]
        pub choices: RefCell<Option<glib::Variant>>,

        // The user selected choices
        #[property(get = Self::get_selected_choices, builder(glib::VariantTy::ARRAY))]
        pub selected_choices: RefCell<Option<glib::Variant>>,

        // Whether to close the window after a selection
        #[property(get, set, construct, default = true)]
        pub close_on_done: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileSelector {
        const NAME: &'static str = "PfsFileSelector";
        type Type = super::FileSelector;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("file-selector.accept", None, move |file_selector, _, _| {
                file_selector.imp().on_accept_clicked();
            });

            klass.set_accessible_role(gtk::AccessibleRole::Group);

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
    impl ObjectImpl for FileSelector {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.setup_gsettings();
            obj.setup_gactions();
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

    impl WidgetImpl for FileSelector {}
    impl WindowImpl for FileSelector {}
    impl AdwWindowImpl for FileSelector {}

    #[gtk::template_callbacks]
    impl FileSelector {
        pub(super) fn send_done(&self, success: bool, close: bool) {
            if !self.done.get() {
                glib::g_debug!(LOG_DOMAIN, "Done, success: {success:#?}");
                self.obj().emit_by_name::<()>("done", &[&success]);
                self.done.replace(true);
            }

            if close && self.close_on_done.get() {
                self.obj().close();
            }
        }

        fn set_current_filter(&self, pos: u32) {
            let obj = self.obj();

            if obj.current_filter() == pos {
                return;
            }

            self.current_filter.replace(pos);
            obj.notify_current_filter();

            let filters = obj.filters();
            let mut filter: Option<gtk::FileFilter> = None;

            if pos != gtk::INVALID_LIST_POSITION {
                if let Some(value) = filters {
                    filter = match value.item(pos) {
                        Some(object) => object.downcast_ref::<gtk::FileFilter>().cloned(),
                        None => None,
                    }
                }
            }

            self.dir_view.set_type_filter(filter);
        }

        fn set_filename(&self, filename: String) {
            let obj = self.obj();

            if obj.filename() == filename {
                return;
            }

            // TODO: check validity

            *self.filename.borrow_mut() = filename;
            obj.notify_filename();
        }

        fn set_mode(&self, mode: FileSelectorMode) {
            let obj = self.obj();

            if *self.mode.borrow() == mode {
                return;
            }

            let directories_only = match mode {
                FileSelectorMode::OpenFile | FileSelectorMode::SaveFile => false,
                FileSelectorMode::SaveFiles => true,
            };
            obj.set_directory(directories_only);

            self.search_bar.set_key_capture_widget(None::<&gtk::Widget>);
            if mode == FileSelectorMode::OpenFile {
                self.search_bar
                    .set_key_capture_widget(Some(obj.upcast_ref::<gtk::Widget>()));
            }

            *self.mode.borrow_mut() = mode;
            obj.notify_mode();
        }

        fn set_choices_menu(&self, actions: gio::SimpleActionGroup, menu: &gio::Menu) {
            let obj = self.obj();

            obj.insert_action_group("custom-choices", Some(&actions));

            self.choices_menu_button.set_menu_model(Some(menu));
            self.choices_menu_button.set_visible(menu.n_items() > 0);

            *self.choices_actions.borrow_mut() = Some(actions);
        }

        pub fn set_choices(&self, choices: Option<glib::Variant>) {
            let actions = gio::SimpleActionGroup::new();
            let menu = gio::Menu::new();

            let Some(choices) = choices else {
                self.set_choices_menu(actions, &menu);
                *self.choices.borrow_mut() = choices;
                return;
            };

            let iter = choices.iter();
            for variant in iter {
                let Some((choice_id, label, choices_iter, selected)) =
                    <(String, String, glib::Variant, String)>::from_variant(&variant)
                else {
                    glib::g_critical!(LOG_DOMAIN, "Invalid choices format");
                    return;
                };

                if choices_iter.n_children() > 0 {
                    let submenu = gio::Menu::new();
                    let submenu_item = gio::MenuItem::new_submenu(Some(&label), &submenu);
                    menu.append_item(&submenu_item);

                    let action = gio::SimpleAction::new_stateful(
                        &choice_id,
                        Some("".to_variant().type_()),
                        &"".to_variant(),
                    );
                    actions.add_action(&action);

                    for choices_list in choices_iter.iter() {
                        let Some(choice_tuple) = <(String, String)>::from_variant(&choices_list)
                        else {
                            glib::g_critical!(LOG_DOMAIN, "Invalid choices list format");
                            return;
                        };

                        let (option_id, option_label) = choice_tuple;
                        let item = gio::MenuItem::new(
                            Some(&option_label),
                            Some(&format!("custom-choices.{choice_id}::{option_id}")),
                        );

                        submenu.append_item(&item);
                        if option_id == selected {
                            action.set_state(&option_id.to_variant());
                        }
                    }
                } else {
                    let action = gio::SimpleAction::new_stateful(
                        &choice_id,
                        None,
                        &(selected == "true").to_variant(),
                    );
                    actions.add_action(&action);
                    menu.append(Some(&label), Some(&format!("custom-choices.{choice_id}")));
                }
            }

            self.set_choices_menu(actions, &menu);
            *self.choices.borrow_mut() = Some(choices);
        }

        fn get_selected_choices(&self) -> Option<glib::Variant> {
            let action_group = self.choices_actions.borrow().clone()?;
            let action_names = action_group.list_actions();
            let mut ret: Vec<(String, String)> = Vec::new();

            for name in action_names {
                let action = action_group.lookup_action(&name).unwrap();
                let type_ = action.state_type().unwrap();
                let state = action.state().unwrap();

                if type_.as_str() == "b" {
                    let state: bool = state.get().unwrap();
                    ret.push((name.to_string(), state.to_string()));
                } else if type_.as_str() == "s" {
                    ret.push((name.to_string(), state.get().unwrap()));
                } else {
                    glib::g_critical!(LOG_DOMAIN, "Action {name:#?} has invalid format");
                }
            }
            Some(ret.to_variant())
        }

        #[template_callback]
        fn on_accept_clicked(&self) {
            glib::g_debug!(LOG_DOMAIN, "Selection done");

            if self.obj().mode() == FileSelectorMode::SaveFile {
                let selected = self.obj().selected().unwrap();
                let first = selected.first().unwrap();
                let file = gio::File::for_uri(first);

                if file.query_exists(None::<&gio::Cancellable>) {
                    self.obj().confirm_overwrite(&file);
                    return;
                }
            }

            self.send_done(true, true);
        }

        #[template_callback]
        fn on_new_uri(&self, uri: &str) {
            glib::g_debug!(LOG_DOMAIN, "New uri {uri:#?}");
            self.obj().set_current_folder(gio::File::for_uri(uri));
            self.bottom_sheet.get().set_open(false);
            self.search_entry.set_text("");
        }

        #[template_callback]
        fn on_new_filename(&self, filename: String) {
            if self.obj().mode() != FileSelectorMode::SaveFile {
                return;
            }
            glib::g_debug!(LOG_DOMAIN, "New filename: {filename:#?}");
            if !filename.is_empty() {
                self.set_filename(filename);
            }
        }

        #[template_callback]
        fn on_close_requested(&self) -> bool {
            self.send_done(false, false);
            false
        }

        #[template_callback]
        fn folder_to_label(&self) -> String {
            let Some(file) = self.obj().current_folder() else {
                return "Unknown".to_string();
            };
            util::folder_to_name(file)
        }

        #[template_callback]
        fn folder_to_icon_name(&self) -> &str {
            let Some(file) = self.obj().current_folder() else {
                return "folder-symbolic";
            };
            util::folder_to_icon_name(file)
        }

        #[template_callback]
        fn folder_to_tooltip(&self) -> String {
            let Some(file) = self.obj().current_folder() else {
                return "".to_string();
            };
            let path = file.path().unwrap_or_default();
            path.to_str().unwrap_or_default().to_string()
        }

        #[template_callback]
        fn n_items_to_visible(&self) -> bool {
            let Some(filters) = self.obj().filters() else {
                return false;
            };
            filters.n_items() > 0
        }

        #[template_callback]
        fn filters_to_menu_model(&self) -> Option<gio::MenuModel> {
            let filters = self.obj().filters()?;
            let menu = gio::Menu::new();
            let mut pos = 0;
            for item in &filters {
                let item = item.unwrap();
                let Some(filter) = item.downcast_ref::<gtk::FileFilter>() else {
                    continue;
                };
                let name = filter.name().unwrap_or("Unknown filter".into());
                let action = format!("file-selector.set-filter::{pos}");
                menu.insert(pos, Some(&name), Some(&action));
                pos += 1;
            }

            Some(menu.into())
        }

        #[template_callback]
        fn can_accept_file_or_dir(
            &self,
            _mode: FileSelectorMode,
            folder: Option<&gio::File>,
            has_selection: bool,
            text: &str,
        ) -> bool {
            if self.obj().mode() == FileSelectorMode::SaveFile {
                if text.is_empty() {
                    return false;
                }

                util::is_valid_folder(folder)
            } else {
                has_selection
            }
        }

        #[template_callback]
        fn mode_to_filename_entry(&self, mode: FileSelectorMode) -> bool {
            match mode {
                FileSelectorMode::OpenFile | FileSelectorMode::SaveFiles => false,
                FileSelectorMode::SaveFile => true,
            }
        }

        #[template_callback]
        fn on_search_changed(&self, entry: gtk::Editable) {
            let search_term = entry.text();

            self.dir_view.set_search_term(search_term);
        }
    }
}

glib::wrapper! {
    pub struct FileSelector(ObjectSubclass<imp::FileSelector>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for FileSelector {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

impl FileSelector {
    pub fn new() -> Self {
        Self::default()
    }

    fn setup_gsettings(&self) {
        if !util::is_schema_installed() {
            glib::g_debug!(
                LOG_DOMAIN,
                "Not creating settings as schema is not available"
            );
            return;
        }

        let settings = gio::Settings::new("mobi.phosh.FileSelector");
        *self.imp().settings.borrow_mut() = Some(settings);
    }

    fn set_sort_mode(&self, name: &str, reversed: bool) {
        let enum_type = glib::EnumClass::with_type(SortMode::static_type()).unwrap();
        let mode = enum_type
            .value_by_nick(name)
            .expect("Invalid Sort mode {name}");

        let binding = self.imp().settings.borrow();
        if let Some(settings) = binding.as_ref() {
            let _ = settings.set_enum("sort-by", mode.value());
            let _ = settings.set_boolean("sort-reverse", reversed);
        }

        let m = unsafe { SortMode::from_glib(mode.value()) };
        self.imp().dir_view.get().set_sorting(m, reversed);
    }

    fn setup_gactions(&self) {
        let actions = gio::SimpleActionGroup::new();
        stateful_action!(
            actions,
            "show-hidden-files",
            false,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |action, _| {
                    let state = action.state().unwrap();
                    let action_state: bool = state.get().unwrap();
                    let show_hidden = !action_state;
                    action.set_state(&show_hidden.to_variant());

                    this.imp().dir_view.get().set_show_hidden(show_hidden);
                }
            )
        );

        let binding = self.imp().settings.borrow();
        let (sort_by_value, reversed) = if let Some(settings) = binding.as_ref() {
            let sort_by_value = settings.enum_("sort-by");
            let reversed = settings.boolean("sort-reverse");
            (sort_by_value, reversed)
        } else {
            (0, false)
        };

        let enum_type = glib::EnumClass::with_type(SortMode::static_type()).unwrap();
        let mode_name = enum_type.value(sort_by_value).unwrap().nick();
        let sort_by = (mode_name, reversed);
        stateful_action!(
            actions,
            "sort",
            Some(sort_by.to_variant().type_()),
            sort_by,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |action, param| {
                    let param = param.unwrap();
                    let new_state: (String, bool) = param.get().unwrap();
                    let (what, reversed) = new_state;

                    this.set_sort_mode(&what, reversed);
                    action.set_state(&(what, reversed).to_variant());
                }
            )
        );
        self.set_sort_mode(mode_name, reversed);

        let pos = self.imp().current_filter.get().to_string();
        stateful_action!(
            actions,
            "set-filter",
            Some("".to_variant().type_()),
            pos,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |action, param| {
                    let param = param.unwrap();
                    let new_pos: String = param.get().unwrap();

                    action.set_state(&new_pos.to_variant());
                    let pos = new_pos.parse::<u32>().unwrap();
                    this.set_current_filter(pos);
                }
            )
        );

        self.insert_action_group("file-selector", Some(&actions));

        // Keep `current-filter` in sync with action
        let filter_action = actions.lookup_action("set-filter").unwrap();
        self.bind_property("current-filter", &filter_action, "state")
            .sync_create()
            .bidirectional()
            .transform_to(|_, pos: u32| Some(pos.to_string().to_variant()))
            .transform_from(|_, state: glib::Variant| {
                let state: String = state.get().unwrap();
                Some(state.parse::<u32>().unwrap())
            })
            .build();
    }

    fn confirm_overwrite(&self, file: &gio::File) {
        let basename = file.basename().unwrap();
        let dirname = file.parent().unwrap().path().unwrap();
        let body = gettextrs::gettext("Overwrite existing file {} in {}?")
            .replacen("{}", basename.to_str().unwrap(), 1)
            .replacen("{}", dirname.to_str().unwrap(), 1);

        let dialog = adw::AlertDialog::builder()
            .title(gettextrs::gettext("Replace existing file?"))
            .body(&body)
            .close_response("cancel")
            .default_response("cancel")
            .build();

        dialog.add_response("cancel", &gettextrs::gettext("Cancel"));
        dialog.add_response("replace", &gettextrs::gettext("_Replace"));
        dialog.set_response_appearance("replace", adw::ResponseAppearance::Destructive);

        dialog.choose(
            Some(self),
            None::<&gio::Cancellable>,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |response| {
                    if response == "replace" {
                        this.imp().send_done(true, true);
                    }
                }
            ),
        );
    }

    pub fn selected(&self) -> Option<Vec<String>> {
        let items = self.imp().dir_view.get().selected();

        if self.mode() == FileSelectorMode::SaveFile {
            let path = self.current_folder().unwrap().path().unwrap();
            let file = gio::File::for_path(path.join(self.filename()));

            Some(vec![file.uri().to_string()])
        } else {
            items
        }
    }

    pub fn set_current_directory(&self, directory: String) {
        let file = gio::File::for_path(directory);

        self.set_current_folder(file);
    }

    pub fn select_item(&self, item: &gio::File) {
        self.imp().dir_view.select_item(item);
    }
}

// C bindings:

pub type PfsFileSelector = <imp::FileSelector as ObjectSubclass>::Instance;

use glib::ffi::GType;
#[no_mangle]
pub extern "C" fn pfs_file_selector_get_type() -> GType {
    <FileSelector as StaticType>::static_type().into_glib()
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_file_selector_new() -> *mut PfsFileSelector {
    FileSelector::new().into_glib_ptr()
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_file_selector_set_current_directory(
    fs: *mut PfsFileSelector,
    directory: *const std::ffi::c_char,
) {
    let obj = FileSelector::from_glib_ptr_borrow(&fs);
    let dir: Borrowed<glib::GString> = from_glib_borrow(directory);

    obj.set_current_directory(dir.to_string());
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_file_selector_set_accept_label(
    fs: *mut PfsFileSelector,
    accept_label: *const std::ffi::c_char,
) {
    let obj = FileSelector::from_glib_ptr_borrow(&fs);
    let label: Borrowed<glib::GString> = from_glib_borrow(accept_label);

    obj.set_accept_label(label.to_string());
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_file_selector_get_selected(
    fs: *mut PfsFileSelector,
) -> *const *mut std::ffi::c_char {
    let obj = FileSelector::from_glib_ptr_borrow(&fs);
    let strv: glib::StrV = obj.selected().unwrap().into();

    strv.into_raw()
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_file_selector_set_mode(fs: *mut PfsFileSelector, mode: i32) {
    let mode = FileSelectorMode::from_glib(mode);
    let obj = FileSelector::from_glib_ptr_borrow(&fs);

    obj.set_mode(mode);
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn pfs_file_selector_set_filename(
    fs: *mut PfsFileSelector,
    filename: *const std::ffi::c_char,
) {
    let obj = FileSelector::from_glib_ptr_borrow(&fs);
    let name: Borrowed<glib::GString> = from_glib_borrow(filename);

    if filename.is_null() {
        obj.set_filename("");
    } else {
        obj.set_filename(name.to_string());
    }
}
