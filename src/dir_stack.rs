/*
 * Copyright 2024-2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Guido Günther <agx@sigxcpu.org>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::Signal;
use glib_macros::Properties;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::config::LOG_DOMAIN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/mobi/phosh/FileSelector/dir-stack.ui")]
    #[properties(wrapper_type = super::DirStack)]
    pub struct DirStack {
        #[template_child]
        pub next_btn: TemplateChild<gtk::Button>,

        #[template_child]
        pub prev_btn: TemplateChild<gtk::Button>,

        // The current folder
        #[property(get, set = Self::set_folder)]
        folder: RefCell<Option<gio::File>>,

        pub(super) is_updating: Cell<bool>,
        pub(super) position: Cell<usize>,
        pub(super) dirstack: RefCell<Vec<gio::File>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DirStack {
        const NAME: &'static str = "PfsDirStack";
        type Type = super::DirStack;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.set_accessible_role(gtk::AccessibleRole::Group);

            klass.install_action("dir.next", None, move |dirstack, _, _| {
                dirstack.goto(1);
            });
            klass.install_action("dir.prev", None, move |dirstack, _, _| {
                dirstack.goto(-1);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl DirStack {
        pub fn update_actions(&self, pos: usize, len: usize) {
            let enabled = len > 0 && pos < len - 1;
            self.obj().action_set_enabled("dir.next", enabled);

            let enabled = len > 0 && pos > 0;
            self.obj().action_set_enabled("dir.prev", enabled);
        }

        fn set_folder(&self, folder: Option<gio::File>) {
            let Some(folder) = folder else { return };

            // Update triggered by us, ignore
            if self.is_updating.get() {
                self.is_updating.replace(false);
                return;
            }

            let uri = folder.uri();

            let mut stack = self.dirstack.borrow_mut();
            // Drop tail if we enter a new subdir
            stack.truncate(self.position.get() + 1);
            stack.push(folder.clone());
            let pos = stack.len() - 1;
            self.position.replace(pos);

            glib::g_debug!(LOG_DOMAIN, "Stacking {uri:#?} at {pos:#?}");

            *self.folder.borrow_mut() = Some(folder);
            self.update_actions(pos, stack.len());
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DirStack {
        fn constructed(&self) {
            self.parent_constructed();
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

    impl WidgetImpl for DirStack {}
    impl BinImpl for DirStack {}
}

glib::wrapper! {
    pub struct DirStack(ObjectSubclass<imp::DirStack>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for DirStack {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

#[gtk::template_callbacks]
impl DirStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn goto(&self, skip: i64) {
        let mut pos = self.imp().position.get() as i64;
        let stack = self.imp().dirstack.borrow_mut();
        let len = stack.len() as i64;

        if skip == 0 {
            return;
        }

        if (skip < 0 && pos + skip >= 0) || (skip > 0 && skip + pos < len) {
            pos += skip;
        } else {
            panic!("Cannot skip {skip:#?} at {pos:#?} with {len:#?}");
        }

        let pos = pos as usize;
        self.imp().position.replace(pos);
        let uri = stack[pos].uri();
        self.imp().update_actions(pos, len as usize);

        self.imp().is_updating.replace(true);
        self.imp().obj().emit_by_name::<()>("new-uri", &[&uri]);
    }
}
