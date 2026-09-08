#[tauri::command]
fn workload_ready(window: tauri::WebviewWindow, width: u32, height: u32) -> Result<bool, String> {
    if width != 1100 || height != 720 {
        // Some macOS title bar configurations reduce the WebView viewport.
        // Correct the measured difference so every fixture renders 1100 × 720.
        let scale = window.scale_factor().map_err(|error| error.to_string())?;
        let size = window
            .inner_size()
            .map_err(|error| error.to_string())?
            .to_logical::<f64>(scale);
        window
            .set_size(tauri::LogicalSize::new(
                size.width + 1100.0 - f64::from(width),
                size.height + 720.0 - f64::from(height),
            ))
            .map_err(|error| error.to_string())?;
        return Ok(false);
    }
    window
        .set_title("Issue tracker — ready")
        .map_err(|error| error.to_string())?;
    Ok(true)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![workload_ready])
        .run(tauri::generate_context!())
        .expect("Could not run the benchmark application");
}
