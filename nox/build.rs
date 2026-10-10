use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=NOX_INSTALL_CHANNEL");
    let channel = match env::var("NOX_INSTALL_CHANNEL").as_deref() {
        Ok("stable") => "stable",
        Ok("dev") => "dev",
        _ => "unknown",
    };
    println!("cargo:rustc-env=NOX_INSTALL_CHANNEL={channel}");
}
