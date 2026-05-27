fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "DOM Wallet");
        res.set("FileDescription", "DOM Wallet — Deterministic Monetary Network");
        res.set("CompanyName", "DOM Protocol");
        res.set("LegalCopyright", "DOM Protocol");
        res.set("OriginalFilename", "dom-wallet-app.exe");
        // Subsystem: windows (hides the console). We toggle it via no_console feature in main.
        if std::path::Path::new("assets/dom.ico").exists() {
            res.set_icon("assets/dom.ico");
        }
        let _ = res.compile();
    }
}
