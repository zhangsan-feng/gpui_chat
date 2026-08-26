use gpui::SharedString;

mod external;

pub use external::choose_files;

#[derive(Clone, Debug, Default)]
pub struct FilePickerOptions {
    pub files: bool,
    pub directories: bool,
    pub multiple: bool,
    pub prompt: Option<SharedString>,
}

impl FilePickerOptions {
    pub fn single_file() -> Self {
        Self {
            files: true,
            ..Default::default()
        }
    }

    pub fn multiple_files() -> Self {
        Self {
            files: true,
            multiple: true,
            ..Default::default()
        }
    }
}

impl From<FilePickerOptions> for gpui::PathPromptOptions {
    fn from(options: FilePickerOptions) -> Self {
        Self {
            files: options.files,
            directories: options.directories,
            multiple: options.multiple,
            prompt: options.prompt,
        }
    }
}
