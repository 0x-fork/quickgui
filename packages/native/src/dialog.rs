use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use napi_derive::napi;
use quickgui::{
    FileDialogFilter, PathPromptOptions, PathPromptResponse, PlatformResponse, PromptButton,
    PromptLevel, SavePathOptions, SavePathResponse,
};

use crate::NativeEvent;

#[derive(Clone)]
#[napi(object)]
pub struct NativeDialogButton {
    pub label: String,
    pub role: Option<String>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeDialogOptions {
    pub level: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub buttons: Vec<NativeDialogButton>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeFileDialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeOpenDialogOptions {
    pub files: bool,
    pub directories: bool,
    pub multiple: bool,
    pub title: Option<String>,
    pub prompt: Option<String>,
    pub directory: Option<String>,
    pub suggested_name: Option<String>,
    pub filters: Vec<NativeFileDialogFilter>,
    pub shows_hidden_files: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeSaveDialogOptions {
    pub directory: String,
    pub title: Option<String>,
    pub suggested_name: Option<String>,
    pub prompt: Option<String>,
    pub filters: Vec<NativeFileDialogFilter>,
    pub shows_hidden_files: bool,
}

enum PendingDialogResponse {
    Alert(PlatformResponse<usize>),
    Open(PathPromptResponse),
    Save(SavePathResponse),
}

pub(crate) struct PendingDialog {
    window: Option<u32>,
    request: u32,
    response: PendingDialogResponse,
}

impl PendingDialog {
    pub(crate) fn alert(
        window: Option<u32>,
        request: u32,
        response: PlatformResponse<usize>,
    ) -> Self {
        Self {
            window,
            request,
            response: PendingDialogResponse::Alert(response),
        }
    }

    pub(crate) fn open(window: Option<u32>, request: u32, response: PathPromptResponse) -> Self {
        Self {
            window,
            request,
            response: PendingDialogResponse::Open(response),
        }
    }

    pub(crate) fn save(window: Option<u32>, request: u32, response: SavePathResponse) -> Self {
        Self {
            window,
            request,
            response: PendingDialogResponse::Save(response),
        }
    }

    pub(crate) fn request(&self) -> u32 {
        self.request
    }

    pub(crate) fn poll(&mut self, context: &mut Context<'_>) -> Poll<NativeEvent> {
        let (kind, value, paths, error) = match &mut self.response {
            PendingDialogResponse::Alert(response) => match Pin::new(response).poll(context) {
                Poll::Ready(Ok(index)) => ("alert-dialog", Some(index.to_string()), None, None),
                Poll::Ready(Err(error)) => ("alert-dialog", None, None, Some(error.to_string())),
                Poll::Pending => return Poll::Pending,
            },
            PendingDialogResponse::Open(response) => match Pin::new(response).poll(context) {
                Poll::Ready(Ok(Some(selected))) => {
                    match selected.into_iter().map(native_path_string).collect() {
                        Ok(paths) => ("open-dialog", None, Some(paths), None),
                        Err(error) => ("open-dialog", None, None, Some(error)),
                    }
                }
                Poll::Ready(Ok(None)) => ("open-dialog", None, None, None),
                Poll::Ready(Err(error)) => ("open-dialog", None, None, Some(error.to_string())),
                Poll::Pending => return Poll::Pending,
            },
            PendingDialogResponse::Save(response) => match Pin::new(response).poll(context) {
                Poll::Ready(Ok(Some(path))) => match native_path_string(path) {
                    Ok(path) => ("save-dialog", Some(path), None, None),
                    Err(error) => ("save-dialog", None, None, Some(error)),
                },
                Poll::Ready(Ok(None)) => ("save-dialog", None, None, None),
                Poll::Ready(Err(error)) => ("save-dialog", None, None, Some(error.to_string())),
                Poll::Pending => return Poll::Pending,
            },
        };
        Poll::Ready(NativeEvent {
            kind: kind.to_owned(),
            window: self.window.unwrap_or(0),
            target: self.request,
            value,
            paths,
            error,
        })
    }
}

fn native_path_string(path: PathBuf) -> Result<String, String> {
    path.into_os_string()
        .into_string()
        .map_err(|_| "the native file dialog returned a path that is not valid UTF-8".to_owned())
}

pub(crate) fn native_dialog_configuration(
    options: &NativeDialogOptions,
) -> Result<(PromptLevel, Vec<PromptButton>), String> {
    let level = match options.level.as_deref() {
        None | Some("info") => PromptLevel::Info,
        Some("warning") => PromptLevel::Warning,
        Some("critical") => PromptLevel::Critical,
        Some(level) => return Err(format!("unknown native dialog level `{level}`")),
    };
    let buttons = options
        .buttons
        .iter()
        .map(|button| match button.role.as_deref() {
            None | Some("other") => Ok(PromptButton::new(button.label.clone())),
            Some("default") => Ok(PromptButton::ok(button.label.clone())),
            Some("cancel") => Ok(PromptButton::cancel(button.label.clone())),
            Some(role) => Err(format!("unknown native dialog button role `{role}`")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((level, buttons))
}

pub(crate) fn native_open_dialog_options(options: NativeOpenDialogOptions) -> PathPromptOptions {
    let mut native_options = PathPromptOptions::new()
        .files(options.files)
        .directories(options.directories)
        .multiple(options.multiple)
        .filters(native_file_dialog_filters(options.filters))
        .shows_hidden_files(options.shows_hidden_files);
    if let Some(title) = options.title {
        native_options = native_options.title(title);
    }
    if let Some(prompt) = options.prompt {
        native_options = native_options.prompt(prompt);
    }
    if let Some(directory) = options.directory {
        native_options = native_options.directory(directory);
    }
    if let Some(name) = options.suggested_name {
        native_options = native_options.suggested_name(name);
    }
    native_options
}

pub(crate) fn native_save_dialog_options(options: NativeSaveDialogOptions) -> SavePathOptions {
    let mut native_options = SavePathOptions::new(options.directory)
        .filters(native_file_dialog_filters(options.filters))
        .shows_hidden_files(options.shows_hidden_files);
    if let Some(title) = options.title {
        native_options = native_options.title(title);
    }
    if let Some(name) = options.suggested_name {
        native_options = native_options.suggested_name(name);
    }
    if let Some(prompt) = options.prompt {
        native_options = native_options.prompt(prompt);
    }
    native_options
}

fn native_file_dialog_filters(filters: Vec<NativeFileDialogFilter>) -> Vec<FileDialogFilter> {
    filters
        .into_iter()
        .map(|filter| FileDialogFilter::new(filter.name, filter.extensions))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_options_preserve_level_button_order_and_roles() {
        let options = NativeDialogOptions {
            level: Some("warning".to_owned()),
            message: "Save changes?".to_owned(),
            detail: Some("Your edits have not been written yet.".to_owned()),
            buttons: vec![
                NativeDialogButton {
                    label: "Save".to_owned(),
                    role: Some("default".to_owned()),
                },
                NativeDialogButton {
                    label: "Don't Save".to_owned(),
                    role: None,
                },
                NativeDialogButton {
                    label: "Cancel".to_owned(),
                    role: Some("cancel".to_owned()),
                },
            ],
        };

        let (level, buttons) = native_dialog_configuration(&options).unwrap();
        assert_eq!(level, PromptLevel::Warning);
        assert_eq!(
            buttons,
            [
                PromptButton::ok("Save"),
                PromptButton::new("Don't Save"),
                PromptButton::cancel("Cancel"),
            ]
        );
    }

    #[test]
    fn dialog_options_reject_unknown_platform_semantics() {
        let options = NativeDialogOptions {
            level: Some("question".to_owned()),
            message: "Continue?".to_owned(),
            detail: None,
            buttons: vec![NativeDialogButton {
                label: "OK".to_owned(),
                role: Some("default".to_owned()),
            }],
        };
        assert!(native_dialog_configuration(&options).is_err());

        let options = NativeDialogOptions {
            level: None,
            message: "Continue?".to_owned(),
            detail: None,
            buttons: vec![NativeDialogButton {
                label: "OK".to_owned(),
                role: Some("destructive".to_owned()),
            }],
        };
        assert!(native_dialog_configuration(&options).is_err());
    }

    #[test]
    fn file_dialog_options_preserve_selection_and_destination_semantics() {
        let open = native_open_dialog_options(NativeOpenDialogOptions {
            files: true,
            directories: true,
            multiple: true,
            title: Some("Open content".to_owned()),
            prompt: Some("Choose".to_owned()),
            directory: Some("/tmp".to_owned()),
            suggested_name: Some("notes.md".to_owned()),
            filters: vec![NativeFileDialogFilter {
                name: "Markdown".to_owned(),
                extensions: vec!["md".to_owned()],
            }],
            shows_hidden_files: true,
        });
        assert!(open.files);
        assert!(open.directories);
        assert!(open.multiple);
        assert!(open.shows_hidden_files);
        assert_eq!(open.title.as_deref(), Some("Open content"));
        assert_eq!(open.prompt.as_deref(), Some("Choose"));
        assert_eq!(open.directory, Some(PathBuf::from("/tmp")));
        assert_eq!(open.suggested_name.as_deref(), Some("notes.md"));
        assert_eq!(open.filters[0].extensions[0].as_ref(), "md");

        let save = native_save_dialog_options(NativeSaveDialogOptions {
            directory: "/tmp".to_owned(),
            title: Some("Save content".to_owned()),
            suggested_name: Some("quickgui.txt".to_owned()),
            prompt: Some("Export".to_owned()),
            filters: vec![NativeFileDialogFilter {
                name: "Text".to_owned(),
                extensions: vec!["txt".to_owned()],
            }],
            shows_hidden_files: true,
        });
        assert_eq!(save.directory, PathBuf::from("/tmp"));
        assert_eq!(save.title.as_deref(), Some("Save content"));
        assert_eq!(save.suggested_name.as_deref(), Some("quickgui.txt"));
        assert_eq!(save.prompt.as_deref(), Some("Export"));
        assert_eq!(save.filters[0].extensions[0].as_ref(), "txt");
        assert!(save.shows_hidden_files);
    }
}
