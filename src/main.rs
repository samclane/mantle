#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]

use mantle::app::MantleApp;
use mantle::ui::setup_eframe_options;
use mantle::utils::init_logging;

fn main() -> eframe::Result {
    #[cfg(debug_assertions)]
    start_puffin_server(); // Optional, keep if you're using Puffin for profiling

    init_logging();

    let options = setup_eframe_options();

    eframe::run_native(
        "Mantle",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MantleApp::new(cc)))
        }),
    )
}

#[cfg(debug_assertions)]
fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            eprintln!("Failed to start puffin server: {err}");
        }
    };
}
