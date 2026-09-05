fn main() {
    napi_build::setup();
    // The SwiftUI bridge in the core links the back-deployed parts of the Swift runtime through
    // `@rpath`. The crate's unit-test binary needs the system runtime directory on its search path;
    // the addon itself resolves the same libraries by absolute path, so the extra entry is inert.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
