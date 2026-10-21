fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/mic/rustcast-mic.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
