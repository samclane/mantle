use cpal::{
    traits::{DeviceTrait, HostTrait},
    Host,
};
use std::sync::{Arc, Mutex};

pub const AUDIO_BUFFER_DEFAULT: usize = 48000;

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
    pub fn build_stream(&mut self, max_buffer_size: &usize) -> Result<(), cpal::BuildStreamError> {
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

        self.stream = Some(stream);
        Ok(())
    }

    pub fn devices(&self) -> Vec<cpal::Device> {
        self.host
            .output_devices()
            .map_or(Vec::new(), |devices| devices.collect())
    }
}
