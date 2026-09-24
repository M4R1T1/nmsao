#[cfg(windows)]
fn main() {
    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon("assets/classmini.a.ico");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {}