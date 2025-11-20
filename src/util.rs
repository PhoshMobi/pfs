/*
 * Copyright 2024-2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use gtk::gio::prelude::*;
use gtk::{gio, glib};

#[macro_export]
macro_rules! stateful_action {
    ($actions_group:expr, $name:expr, $state:expr, $callback:expr) => {
        let simple_action = gio::SimpleAction::new_stateful($name, None, &$state.to_variant());
        simple_action.connect_activate($callback);
        $actions_group.add_action(&simple_action);
    };
    ($actions_group:expr, $name:expr, $param_type:expr, $state:expr, $callback:expr) => {
        let simple_action =
            gio::SimpleAction::new_stateful($name, $param_type, &$state.to_variant());
        simple_action.connect_activate($callback);
        $actions_group.add_action(&simple_action);
    };
}

pub static SPECIAL_DIRS: [(glib::UserDirectory, &str); 5] = [
    (glib::UserDirectory::Documents, "folder-documents-symbolic"),
    (glib::UserDirectory::Downloads, "folder-download-symbolic"),
    (glib::UserDirectory::Music, "folder-music-symbolic"),
    (glib::UserDirectory::Pictures, "folder-pictures-symbolic"),
    (glib::UserDirectory::Videos, "folder-videos-symbolic"),
];

pub fn folder_to_name(file: gio::File) -> String {
    let uri = file.uri();
    match uri.as_str() {
        "recent:///" => return gettextrs::gettext("Recent"),
        "trash:///" => return gettextrs::gettext("Trash"),
        _ => {}
    };

    let name = match file.path() {
        Some(path) => path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string(),
        None => uri.split(":").next().unwrap_or_default().to_string(),
    };

    name
}

pub fn folder_to_icon_name(file: gio::File) -> &'static str {
    let uri = file.uri();
    match uri.as_str() {
        "recent:///" => return "document-open-recent-symbolic",
        "trash:///" => return "user-trash-symbolic",
        _ => {}
    }

    let home = gio::File::for_path(glib::home_dir());
    if home.equal(&file) {
        return "user-home-symbolic";
    }

    for (dir, icon) in &SPECIAL_DIRS {
        let Some(dir) = glib::user_special_dir(*dir) else {
            continue;
        };
        let folder = gio::File::for_path(dir);

        if folder.equal(&file) {
            return icon;
        }
    }

    "folder-symbolic"
}

// Check if folder has a valid path (e.g. isn't recent:/// or trash:///
pub fn is_valid_folder(folder: &Option<gio::File>) -> bool {
    if folder.is_none() {
        return false;
    }

    folder.as_ref().unwrap().path().is_some()
}

pub fn is_schema_installed() -> bool {
    let source = gio::SettingsSchemaSource::default();
    if source.is_none() {
        return false;
    }
    let source = source.unwrap();

    source.lookup("mobi.phosh.FileSelector", true).is_some()
}
