use super::*;

#[test]
fn clipboard_payloads_validate_kind_and_image_format() {
    let unknown = NativeClipboardItem {
        entries: vec![NativeClipboardEntry {
            kind: "unknown".to_owned(),
            text: None,
            metadata: None,
            format: None,
            data: None,
            paths: None,
            url: None,
        }],
    };
    assert!(
        clipboard_item(unknown)
            .unwrap_err()
            .contains("unknown clipboard")
    );

    let missing_image = NativeClipboardItem {
        entries: vec![NativeClipboardEntry {
            kind: "image".to_owned(),
            text: None,
            metadata: None,
            format: Some("image/unknown".to_owned()),
            data: Some(Vec::new()),
            paths: None,
            url: None,
        }],
    };
    assert!(
        clipboard_item(missing_image)
            .unwrap_err()
            .contains("MIME type")
    );
}

#[test]
fn window_actions_parse_exact_values() {
    assert!(matches!(
        parse_window_action("set-fullscreen", Some("true".to_owned())),
        Ok(WindowAction::SetFullscreen(true))
    ));
    assert!(parse_window_action("set-fullscreen", Some("yes".to_owned())).is_err());
    assert!(parse_window_action("missing", None).is_err());

    assert!(matches!(
        parse_window_action("set-vibrancy", Some("under-page".to_owned())),
        Ok(WindowAction::SetMacOsVibrancy(Some(
            MacOsVibrancy::UnderPage
        )))
    ));
    assert!(matches!(
        parse_window_action("set-vibrancy", None),
        Ok(WindowAction::SetMacOsVibrancy(None))
    ));
    assert!(matches!(
        parse_window_action("set-visual-effect-state", Some("active".to_owned())),
        Ok(WindowAction::SetMacOsVisualEffectState(
            MacOsVisualEffectState::Active
        ))
    ));
    assert!(parse_window_action("set-vibrancy", Some("glass".to_owned())).is_err());
}

#[test]
fn shell_actions_preserve_urls_and_paths() {
    assert!(matches!(
        parse_shell_action("open-external", "https://quickgui.dev".to_owned()),
        Ok(ShellAction::OpenExternal(url)) if url == "https://quickgui.dev"
    ));
    assert!(matches!(
        parse_shell_action("trash-path", "/tmp/example".to_owned()),
        Ok(ShellAction::TrashPath(path)) if path.as_os_str() == "/tmp/example"
    ));
    assert!(parse_shell_action("missing", String::new()).is_err());
}
