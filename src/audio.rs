use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Host,
};
use eframe::egui;
use egui_plot::{Legend, Line, PlotPoints};
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};

pub const AUDIO_BUFFER_DEFAULT: usize = 48000;

fn to_complex(buffer: &[f32]) -> Vec<Complex<f32>> {
    buffer
        .iter()
        .map(|&value| Complex::new(value, 0.0))
        .collect()
}

fn to_real_f32(buffer: &[Complex<f32>]) -> Vec<f32> {
    buffer.iter().map(|value| value.re).collect()
}

fn subsample(buffer: &[f32], factor: usize) -> Vec<f32> {
    buffer
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if index % factor == 0 {
                Some(*value)
            } else {
                None
            }
        })
        .collect()
}

pub struct AudioManager {
    host: Host,
    current_device: Option<cpal::Device>,
    configuration: Option<cpal::StreamConfig>,
    stream: Option<cpal::Stream>,
    samples_buffer: Arc<Mutex<Vec<f32>>>,
}

impl Default for AudioManager {
    fn default() -> Self {
        let host = cpal::default_host();
        let current_device = host.default_output_device();
        let configuration = current_device
            .as_ref()
            .and_then(|device| device.supported_output_configs().ok())
            .and_then(|mut configs| configs.next())
            .map(|config| config.with_max_sample_rate().config());

        Self {
            host,
            current_device,
            configuration,
            stream: None,
            samples_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl AudioManager {
    pub fn build_output_stream(
        &mut self,
        max_buffer_size: &usize,
    ) -> Result<(), cpal::BuildStreamError> {
        let device = self
            .current_device
            .as_ref()
            .ok_or(cpal::BuildStreamError::DeviceNotAvailable)?;

        let config = self
            .configuration
            .as_ref()
            .ok_or(cpal::BuildStreamError::InvalidArgument)?;

        let buffer_clone = Arc::clone(&self.samples_buffer);
        let max_size = *max_buffer_size;

        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buffer = buffer_clone.lock().unwrap();
                buffer.extend_from_slice(data);
                if buffer.len() > max_size {
                    let excess = buffer.len() - max_size;
                    buffer.drain(0..excess);
                }
            },
            move |err| {
                log::error!("an error occurred on the output audio stream: {}", err);
            },
            None,
        )?;

        let _ = stream.play();
        self.stream = Some(stream);
        Ok(())
    }

    pub fn build_input_stream(
        &mut self,
        max_buffer_size: &usize,
    ) -> Result<(), cpal::BuildStreamError> {
        let device = self
            .current_device
            .as_ref()
            .ok_or(cpal::BuildStreamError::DeviceNotAvailable)?;

        let config = self
            .configuration
            .as_ref()
            .ok_or(cpal::BuildStreamError::InvalidArgument)?;

        let buffer_clone = Arc::clone(&self.samples_buffer);
        let max_size = *max_buffer_size;

        let stream = device.build_input_stream(
            config,
            move |data: &[f32], _| {
                let mut buffer = buffer_clone.lock().unwrap();
                buffer.extend_from_slice(data);
                if buffer.len() > max_size {
                    let excess = buffer.len() - max_size;
                    buffer.drain(0..excess);
                }
            },
            move |err| {
                log::error!("an error occurred on the input audio stream: {}", err);
            },
            None,
        )?;

        let _ = stream.play();
        self.stream = Some(stream);
        Ok(())
    }

    fn fft(&self) -> Vec<Complex<f32>> {
        let mut buffer = to_complex(&self.samples_buffer.lock().unwrap().clone());
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(buffer.len());
        fft.process(&mut buffer);
        buffer
    }

    pub fn real_fft(&self) -> Vec<f32> {
        let buffer = self.fft();
        // Return the first half of the buffer
        to_real_f32(&buffer[0..buffer.len() / 2])
    }

    pub fn devices(&self) -> Vec<cpal::Device> {
        self.host
            .output_devices()
            .map_or(Vec::new(), |devices| devices.collect())
    }

    pub fn get_samples_data(&self) -> Result<Vec<f32>, String> {
        self.samples_buffer
            .lock()
            .map_err(|err| err.to_string())
            .map(|buffer| buffer.clone())
    }

    pub fn ui(&self, ui: &mut eframe::egui::Ui) {
        let audio_data = self.get_samples_data();
        let spectrum = self.real_fft();

        if let Ok(data) = audio_data {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui_plot::Plot::new("Audio Samples")
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .legend(Legend::default())
                    .show(ui, |plot_ui| {
                        let lines = PlotPoints::from_ys_f32(&subsample(&data, 10));
                        plot_ui.line(Line::new(lines));
                    });
                egui_plot::Plot::new("FFT")
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .legend(Legend::default())
                    .show(ui, |plot_ui| {
                        let lines = PlotPoints::from_ys_f32(&subsample(&spectrum, 10));
                        plot_ui.line(Line::new(lines));
                    });
            });
        } else {
            ui.label("No audio data available");
        }
    }
}
