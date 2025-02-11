use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Host,
};
use eframe::egui::{self};
use egui_plot::{Legend, Line, PlotPoints};
use lifx_core::HSBK;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};

use crate::color::DEFAULT_KELVIN;

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

impl Clone for AudioManager {
    fn clone(&self) -> Self {
        // host and stream can't clone, so create a new instance without stream
        let host = cpal::default_host();
        Self {
            host,
            current_device: self.current_device.clone(),
            configuration: self.configuration.clone(),
            stream: None,
            samples_buffer: Arc::clone(&self.samples_buffer),
        }
    }
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

    fn fft(samples: &[f32]) -> Vec<Complex<f32>> {
        let mut buffer = to_complex(samples);
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(buffer.len());
        fft.process(&mut buffer);
        buffer
    }

    pub fn fft_real(spectrum: &[f32]) -> Vec<f32> {
        let buffer = Self::fft(spectrum);
        to_real_f32(&buffer[0..buffer.len() / 2])
    }

    pub fn power_spectrum(samples: &[f32]) -> Vec<f32> {
        let buffer = Self::fft(samples);
        buffer.iter().map(|value| value.norm_sqr()).collect()
    }

    pub fn power(samples: &[f32]) -> u16 {
        let power_spectrum = Self::power_spectrum(samples);
        let avg_power = power_spectrum.iter().sum::<f32>() / power_spectrum.len() as f32;
        (avg_power.sqrt() * u16::MAX as f32) as u16
    }

    pub fn samples_to_hsbk(samples: Vec<f32>) -> HSBK {
        let value = Self::power(&samples);

        HSBK {
            hue: Self::freq_to_hue(&samples),
            saturation: u16::MAX,
            brightness: value,
            kelvin: DEFAULT_KELVIN,
        }
    }

    pub fn freq_to_hue(samples: &[f32]) -> u16 {
        let spectrum = Self::fft(samples);
        let sample_rate = AUDIO_BUFFER_DEFAULT as f32;
        let dominant_freq_hz = spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.norm_sqr().partial_cmp(&b.norm_sqr()).unwrap())
            .map(|(index, _)| index as f32 * sample_rate / spectrum.len() as f32)
            .unwrap_or_default();
        let max_freq = sample_rate / 2.0;
        ((dominant_freq_hz / max_freq) * u16::MAX as f32) as u16
    }

    pub fn freq_centroid(samples: &[f32]) -> HSBK {
        let power_spectrum = AudioManager::power_spectrum(samples);

        let total_power: f32 = power_spectrum.iter().sum();
        let brightness = (total_power.sqrt().min(u16::MAX as f32)) as u16;

        let sample_rate = AUDIO_BUFFER_DEFAULT;
        let fft_size = power_spectrum.len() * 2;
        let bin_size_hz: f32 = (sample_rate / fft_size) as f32;

        let mut weighted_sum = 0.0;
        let mut mag_sum = 0.0;
        for (i, mag) in power_spectrum.iter().enumerate() {
            let freq = i as f32 * bin_size_hz;
            weighted_sum += freq * mag;
            mag_sum += mag;
        }
        let centroid_freq = if mag_sum > 0.0 {
            weighted_sum / mag_sum
        } else {
            0.0
        };
        let max_freq = sample_rate as f32 / 2.0;
        let hue = ((centroid_freq / max_freq) as u16) * u16::MAX;

        HSBK {
            hue,
            saturation: u16::MAX,
            brightness,
            kelvin: DEFAULT_KELVIN,
        }
    }

    pub fn devices(&self) -> Vec<cpal::Device> {
        self.host
            .output_devices()
            .map_or(Vec::new(), |devices| devices.collect())
    }

    pub fn get_samples_data(&self) -> Result<Vec<f32>, String> {
        match self.samples_buffer.lock() {
            Ok(buffer) => Ok(buffer.clone()),
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn clone_samples_buffer(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.samples_buffer)
    }

    pub fn ui(&self, ui: &mut eframe::egui::Ui) {
        let audio_data = self.get_samples_data();

        if let Ok(ref data) = audio_data {
            let spectrum = Self::fft_real(data);
            let color = Self::power(data);
            egui::ScrollArea::vertical().show(ui, |ui| {
                // show current color
                ui.label(format!("Current color: {:?}", color));
                egui_plot::Plot::new("Audio Samples")
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .legend(Legend::default())
                    .show(ui, |plot_ui| {
                        let lines = PlotPoints::from_ys_f32(&subsample(data, 10));
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

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::BuildStreamError;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_to_complex_empty() {
        let buffer: Vec<f32> = vec![];
        let result = to_complex(&buffer);
        assert!(result.is_empty());
    }

    #[test]
    fn test_to_complex_and_to_real_roundtrip() {
        let buffer = vec![0.0, 1.5, -3.2, 42.0];
        let complex = to_complex(&buffer);
        let real = to_real_f32(&complex);
        assert_eq!(buffer, real);
    }

    #[test]
    fn test_subsample_edge_cases() {
        // When factor is 1, we expect the same output.
        let buffer = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(subsample(&buffer, 1), buffer);
        // When the factor exceeds length, we expect only the first element.
        assert_eq!(subsample(&buffer, 10), vec![1.0]);
    }

    // Tests for FFT-related functions

    #[test]
    fn test_fft_real_length() {
        // Use a small sample so that we can verify length invariance.
        let samples = vec![1.0, 0.0, -1.0, 0.0];
        let fft_re = AudioManager::fft_real(&samples);
        assert_eq!(fft_re.len(), samples.len() / 2);
    }

    #[test]
    fn test_power_spectrum_nonnegative() {
        let samples = vec![0.5; 64];
        let spectrum = AudioManager::power_spectrum(&samples);
        // All values in a power spectrum should be nonnegative.
        assert!(spectrum.iter().all(|&val| val >= 0.0));
    }

    #[test]
    fn test_power_zero_input() {
        let samples = vec![0.0; 64];
        // For zero input, the power should be zero.
        assert_eq!(AudioManager::power(&samples), 0);
    }

    #[test]
    fn test_samples_to_hsbk_structure() {
        let samples = vec![1.0, 0.5, 0.25, 0.125];
        let hsbk = AudioManager::samples_to_hsbk(samples.clone());
        // We expect saturation to be u16::MAX and kelvin equal to DEFAULT_KELVIN.
        assert_eq!(hsbk.saturation, u16::MAX);
        assert_eq!(hsbk.kelvin, DEFAULT_KELVIN);
        // Brightness is computed from power; ensure it is less than or equal to u16::MAX.
        assert!(hsbk.brightness <= u16::MAX);
        // hue is computed via FFT-based dominant frequency; for nonzero input, it should be in range.
        assert!(hsbk.hue <= u16::MAX);
    }

    #[test]
    fn test_freq_to_hue_on_constant_signal() {
        // If the input is constant, the FFT should have a dominant spike at index 0,
        // yielding hue 0.
        let samples = vec![1.0; 64];
        let hue = AudioManager::freq_to_hue(&samples);
        assert_eq!(hue, 0);
    }

    #[test]
    fn test_freq_centroid_edge() {
        // When the power spectrum is zero everywhere, the centroid should be zero.
        let samples = vec![0.0; 64];
        let hsbk = AudioManager::freq_centroid(&samples);
        // hue computed from a zero centroid will be 0.
        assert_eq!(hsbk.hue, 0);
        // brightness should be zero as well.
        assert_eq!(hsbk.brightness, 0);
    }

    // Tests for audio stream construction

    #[test]
    fn test_build_output_stream_no_device() {
        // Create an AudioManager with no device.
        let mut manager = AudioManager {
            host: cpal::default_host(),
            current_device: None,
            configuration: None,
            stream: None,
            samples_buffer: Arc::new(Mutex::new(Vec::new())),
        };
        // Expect an error when trying to build the stream.
        let max_buffer_size = AUDIO_BUFFER_DEFAULT;
        let result = manager.build_output_stream(&max_buffer_size);
        match result {
            Err(BuildStreamError::DeviceNotAvailable) => {}
            Err(err) => panic!("Unexpected error variant: {:?}", err),
            Ok(_) => panic!("Expected error when device is None"),
        }
    }

    #[test]
    fn test_build_input_stream_no_config() {
        // Construct a manager with a dummy device (if available) but no configuration.
        // We simulate this by creating a manager via default and then overriding configuration to None.
        let mut manager = AudioManager::default();
        manager.configuration = None;
        let max_buffer_size = AUDIO_BUFFER_DEFAULT;
        let result = manager.build_input_stream(&max_buffer_size);
        match result {
            Err(_err) => {} // We expect an error here.
            Ok(_) => panic!("Expected error when configuration is None"),
        }
    }

    // Test for samples buffer retrieval

    #[test]
    fn test_get_samples_data() {
        let manager = AudioManager::default();
        // Manually populate the samples_buffer
        {
            let mut buffer = manager.samples_buffer.lock().unwrap();
            buffer.extend_from_slice(&[0.1, 0.2, 0.3]);
        }
        let data = manager.get_samples_data().unwrap();
        assert_eq!(data, vec![0.1, 0.2, 0.3]);
    }

    // Minimal UI callback test
    //
    // Although testing GUI code is challenging, we can at least call the `ui` function with a dummy context.
    // Note: This test requires an egui::Context. In real projects, you might use egui's test utilities.
    #[test]
    fn test_ui_no_audio_data() {
        use eframe::egui;
        let manager = AudioManager::default();
        // Create a dummy egui context and run it in a frame
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                manager.ui(ui);
            });
        });
    }
}
