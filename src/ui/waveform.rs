use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::{ColorChannelEntry, MantleApp, WaveformMode, WaveformTracker, AUDIO_ICON},
    audio::AudioManager,
    color::DeltaColor,
    device_info::DeviceInfo,
    screencap::{RegionCaptureTarget, ScreencapManager},
};

use super::widgets::create_highlighted_button;

use eframe::egui::{self, Ui};
use lifx_core::HSBK;

pub fn handle_audio(app: &mut MantleApp, ui: &mut Ui, device: &DeviceInfo) -> Option<DeltaColor> {
    let mut color: Option<HSBK> = None;
    let update_interval_ms = app.settings.update_interval_ms;

    ensure_waveform_channel(app, device.id());
    app.waveform_map
        .entry(device.id())
        .or_insert(WaveformTracker {
            active: false,
            last_update: Instant::now(),
            mode: WaveformMode::Audio,
            region: RegionCaptureTarget::All,
            stop_tx: None,
        });

    if let Some(color_channel) = app.waveform_channel.get(&device.id()) {
        if let Some(tracker) = app.waveform_map.get_mut(&device.id()) {
            if tracker.active
                && tracker.mode == WaveformMode::Audio
                && has_time_elapsed(update_interval_ms, tracker)
            {
                update_color_from_channel(&mut color, tracker, color_channel);
            }
        }
    }

    let is_active = app
        .waveform_map
        .get(&device.id())
        .is_some_and(|w| w.active && w.mode == WaveformMode::Audio);
    if create_highlighted_button(ui, "audio", AUDIO_ICON, is_active)
        .on_hover_text("Toggle audio-reactive color")
        .clicked()
    {
        initialize_waveform_tracker(
            app,
            device,
            update_interval_ms,
            WaveformMode::Audio,
            ui.ctx().clone(),
        );
    }

    color.map(|color| DeltaColor {
        next: color,
        duration: Some((update_interval_ms / 2) as u32),
    })
}

pub(crate) fn ensure_waveform_channel(app: &mut MantleApp, device_id: u64) {
    app.waveform_channel.entry(device_id).or_insert_with(|| {
        let (tx, rx) = mpsc::channel();
        ColorChannelEntry {
            tx,
            rx,
            handle: None,
        }
    });
}

fn stop_active_waveform(app: &mut MantleApp, device_id: u64) {
    if let Some(tracker) = app.waveform_map.get_mut(&device_id) {
        if let Some(stop_tx) = tracker.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        tracker.active = false;
    }
    if let Some(channel) = app.waveform_channel.get_mut(&device_id) {
        if let Some(handle) = channel.handle.take() {
            let _ = handle.join();
        }
    }
}

pub(crate) fn initialize_waveform_tracker(
    app: &mut MantleApp,
    device: &DeviceInfo,
    update_interval_ms: u64,
    mode: WaveformMode,
    ctx: egui::Context,
) {
    let device_id = device.id();

    let is_toggle_off = app
        .waveform_map
        .get(&device_id)
        .map(|w| w.active && w.mode == mode)
        .unwrap_or(false);

    let existing_region = app
        .waveform_map
        .get(&device_id)
        .map(|w| w.region.clone())
        .unwrap_or(RegionCaptureTarget::All);

    stop_active_waveform(app, device_id);

    if is_toggle_off {
        return;
    }

    ensure_waveform_channel(app, device_id);

    app.waveform_map.insert(
        device_id,
        WaveformTracker {
            active: true,
            last_update: Instant::now(),
            mode: mode.clone(),
            region: existing_region.clone(),
            stop_tx: None,
        },
    );

    let tx = match app.waveform_channel.get(&device_id) {
        Some(channel) => channel.tx.clone(),
        None => return,
    };
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let handle = match mode {
        WaveformMode::Screencap => {
            let region = existing_region;
            let shared_subregion = if matches!(region, RegionCaptureTarget::Subregion(_)) {
                app.subregion_points.get(&device_id).cloned()
            } else {
                None
            };
            let ctx = ctx.clone();
            thread::spawn(move || {
                let screen_manager = match ScreencapManager::new() {
                    Ok(sm) => sm,
                    Err(e) => {
                        log::error!("Failed to create screen manager in capture thread: {}", e);
                        return;
                    }
                };
                loop {
                    let capture_region = match &shared_subregion {
                        Some(sub_lock) => {
                            let sub = sub_lock.lock().expect("Failed to lock subregion");
                            RegionCaptureTarget::Subregion(vec![sub.clone()])
                        }
                        None => region.clone(),
                    };
                    match screen_manager.calculate_average_color(capture_region) {
                        Ok(color) => {
                            if tx.send(color).is_err() {
                                break;
                            }
                            ctx.request_repaint();
                        }
                        Err(e) => log::error!("Screen capture error: {}", e),
                    }
                    thread::sleep(Duration::from_millis(update_interval_ms));
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                }
            })
        }
        WaveformMode::Audio => {
            let buffer_clone = app.audio_manager.clone_samples_buffer();
            thread::spawn(move || loop {
                let samples = match buffer_clone.lock() {
                    Ok(buf) => buf.clone(),
                    Err(_) => break,
                };
                let audio_color = AudioManager::samples_to_hsbk(samples);
                if tx.send(audio_color).is_err() {
                    break;
                }
                ctx.request_repaint();
                thread::sleep(Duration::from_millis(update_interval_ms));
                if stop_rx.try_recv().is_ok() {
                    break;
                }
            })
        }
    };

    if let Some(channel) = app.waveform_channel.get_mut(&device_id) {
        channel.handle = Some(handle);
    }
    if let Some(tracker) = app.waveform_map.get_mut(&device_id) {
        tracker.stop_tx = Some(stop_tx);
    }
}

pub(crate) fn has_time_elapsed(
    update_interval_ms: u64,
    waveform_tracker: &mut WaveformTracker,
) -> bool {
    Instant::now() - waveform_tracker.last_update > Duration::from_millis(update_interval_ms)
}

pub(crate) fn update_color_from_channel(
    color: &mut Option<HSBK>,
    tracker: &mut WaveformTracker,
    color_channel: &ColorChannelEntry,
) {
    let mut latest = None;
    while let Ok(computed_color) = color_channel.rx.try_recv() {
        latest = Some(computed_color);
    }
    if let Some(latest_color) = latest {
        *color = Some(latest_color);
        tracker.last_update = Instant::now();
    }
}
