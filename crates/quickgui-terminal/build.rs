fn main() {
    println!("cargo:rustc-check-cfg=cfg(quickgui_terminal_extension)");
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"terminal\"))");
    println!("cargo:rustc-cfg=quickgui_terminal_extension");
}
