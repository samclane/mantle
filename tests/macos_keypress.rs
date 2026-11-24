//! Regression test for #19: on macOS 26 a key press reaching the background
//! input listener crashed the app (`dispatch_assert_queue_fail` in HIToolbox).
//!
//! Event taps need Accessibility permission, which CI runners don't grant, so
//! this is opt-in on a Mac (allow your terminal under Privacy & Security ->
//! Accessibility first):
//!
//!     MANTLE_INPUT_TESTS=1 cargo test --test macos_keypress
//!
//! `harness = false` so `main` owns the real main thread and can service the
//! main dispatch queue, standing in for eframe's run loop.

#[cfg(target_os = "macos")]
fn main() {
    use mantle::listener::input_listener::InputListener;
    use rdev::{simulate, EventType, Key};
    use std::{process::exit, thread, time::Duration};

    extern "C" {
        fn dispatch_main() -> !;
    }

    if std::env::var_os("MANTLE_INPUT_TESTS").is_none() {
        println!(
            "macos_keypress skipped: set MANTLE_INPUT_TESTS=1 (needs Accessibility permission)"
        );
        return;
    }

    let listener = InputListener::new();
    listener.start();

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1)); // let the event tap come up
        simulate(&EventType::KeyPress(Key::KeyS)).expect("simulate key press");
        thread::sleep(Duration::from_millis(500));
        let seen = listener.is_key_pressed(Key::KeyS);
        let _ = simulate(&EventType::KeyRelease(Key::KeyS));
        if seen {
            println!("macos_keypress ok");
            exit(0);
        }
        eprintln!("listener never saw the key press (Accessibility permission granted?)");
        exit(1);
    });

    // SAFETY: plain libdispatch call; never returns, the thread above exits the process.
    unsafe { dispatch_main() }
}

#[cfg(not(target_os = "macos"))]
fn main() {}
