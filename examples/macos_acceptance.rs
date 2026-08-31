#[cfg(target_os = "macos")]
mod app {
    include!("macos_acceptance/application.rs");
    include!("macos_acceptance/view.rs");
    include!("macos_acceptance/probe.rs");
    include!("macos_acceptance/tests.rs");
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The macos_acceptance example requires macOS.");
}
