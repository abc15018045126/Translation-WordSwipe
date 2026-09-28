use embed_manifest::{embed_manifest, new_manifest};
use embed_manifest::manifest::{DpiAwareness, SupportedOS::*};

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest("TranslationWordSwipe")
            .supported_os(Windows7..=Windows10)
            .dpi_awareness(DpiAwareness::PerMonitorV2);
        embed_manifest(manifest).expect("unable to embed manifest file");

        let mut res = winres::WindowsResource::new();
        res.set_icon("Translation WordSwipe.ico");
        res.set("ProductName", "Translation WordSwipe");
        res.set("FileDescription", "Translation WordSwipe");
        let _ = res.compile();
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Translation WordSwipe.ico");
}
