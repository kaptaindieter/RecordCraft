fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/app-icon/recordcraft.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/app-icon/recordcraft.ico")
        .set("ProductName", "RecordCraft")
        .set("FileDescription", "RecordCraft Gameplay & Screen Recorder")
        .set("OriginalFilename", "recordcraft.exe")
        .set("InternalName", "recordcraft");
    let _ = res.compile();
}
