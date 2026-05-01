use gtk::gio;
use gtk::prelude::*;

use pfs::file_selector::FileSelectorBuilder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_file_selector() {
        assert_eq!(gtk::init().is_ok(), true);
        pfs::init::init();

        let file_selector = FileSelectorBuilder::new()
            .accept_label("Done")
            .title("Select a File")
            .current_folder(gio::File::for_path("/tmp"))
            .build();

        assert_eq!(file_selector.selected_choices().is_none(), true);
        assert_eq!(file_selector.current_folder().is_some(), true);
        assert_eq!(file_selector.current_folder().unwrap().uri(), "file:///tmp");
    }
}
