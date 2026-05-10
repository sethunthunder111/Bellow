//! Device enumeration and capability querying.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
/// Device capability info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub host_api: String,
    pub is_input: bool,
    pub is_output: bool,
    pub channel_count: u16,
    pub channel_layout: Vec<String>,
    pub supported_sample_rates: Vec<u32>,
    pub supported_buffer_sizes: BufferSizeRange,
    pub supported_formats: Vec<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferSizeRange {
    pub min: u32,
    pub max: u32,
    pub preferred: Vec<u32>,
}

/// List all audio devices with capabilities.
pub fn list_devices() -> Result<DeviceList, cpal::DevicesError> {
    let host = cpal::default_host();
    let host_name = host.id().name().to_string();

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    let default_in = host.default_input_device();
    let default_out = host.default_output_device();

    for device in host.devices()? {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let mut info = DeviceInfo {
            id: format!("{}:{}", host_name, name),
            name: name.clone(),
            host_api: host_name.clone(),
            is_input: false,
            is_output: false,
            channel_count: 2,
            channel_layout: vec!["L".into(), "R".into()],
            supported_sample_rates: vec![44100, 48000, 88200, 96000, 192000],
            supported_buffer_sizes: BufferSizeRange {
                min: 16,
                max: 4096,
                preferred: vec![64, 128, 256, 512, 1024],
            },
            supported_formats: vec!["f32".into(), "i16".into(), "i32".into()],
            is_default: false,
        };

        if let Ok(conf) = device.default_input_config() {
            info.is_input = true;
            info.channel_count = conf.channels();
            info.supported_sample_rates = collect_sample_rates(&device, true);
        }
        if let Ok(conf) = device.default_output_config() {
            info.is_output = true;
            info.channel_count = conf.channels();
            info.supported_sample_rates = collect_sample_rates(&device, false);
        }

        let is_default_in =
            default_in.as_ref().map(|d| d.name().ok()).flatten() == Some(name.clone());
        let is_default_out =
            default_out.as_ref().map(|d| d.name().ok()).flatten() == Some(name.clone());

        if info.is_input {
            let mut in_info = info.clone();
            in_info.is_default = is_default_in;
            inputs.push(in_info);
        }
        if info.is_output {
            let mut out_info = info;
            out_info.is_default = is_default_out;
            outputs.push(out_info);
        }
    }

    Ok(DeviceList {
        inputs,
        outputs,
        host_apis: vec![host_name],
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceList {
    pub inputs: Vec<DeviceInfo>,
    pub outputs: Vec<DeviceInfo>,
    pub host_apis: Vec<String>,
}

fn collect_sample_rates(device: &cpal::Device, input: bool) -> Vec<u32> {
    if input {
        match device.supported_input_configs() {
            Ok(mut it) => {
                let mut rates = Vec::new();
                while let Some(c) = it.next() {
                    rates.push(c.min_sample_rate().0);
                    rates.push(c.max_sample_rate().0);
                }
                rates.sort_unstable();
                rates.dedup();
                rates
            }
            Err(_) => vec![44100, 48000, 96000],
        }
    } else {
        match device.supported_output_configs() {
            Ok(mut it) => {
                let mut rates = Vec::new();
                while let Some(c) = it.next() {
                    rates.push(c.min_sample_rate().0);
                    rates.push(c.max_sample_rate().0);
                }
                rates.sort_unstable();
                rates.dedup();
                rates
            }
            Err(_) => vec![44100, 48000, 96000],
        }
    }
}
