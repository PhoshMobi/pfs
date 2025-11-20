use gtk::prelude::*;
use gtk::{gio, glib};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        assert_eq!(gtk::init().is_ok(), true);
        pfs::init::init();

        let file_selector = glib::Object::builder::<pfs::file_selector::FileSelector>()
            .property("accept_label", "Done")
            .property("title", "Select a File")
            .property("current-folder", gio::File::for_path("/tmp"))
            .build();

        assert_eq!(file_selector.selected_choices().is_none(), true);
        assert_eq!(file_selector.current_folder().is_some(), true);
        assert_eq!(file_selector.current_folder().unwrap().uri(), "file:///tmp");
    }
}
