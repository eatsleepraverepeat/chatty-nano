use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, StreamConfig};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Record audio from the given input device for the specified duration.
pub fn record_audio(
    duration: Duration,
    input_device_name: &str,
    target_channels: u16,
    target_sample_rate: u32,
) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
    // Get default audio host
    let host = cpal::default_host();

    // Find the specified input device
    let device = find_input_device(&host, input_device_name)?;

    println!("Using input device: {}", device.name()?);

    // Get default config
    let config = device.default_input_config()?;
    println!("Default input config: {:?}", config);

    // Record audio
    match config.sample_format() {
        SampleFormat::I16 => record::<i16>(
            &device,
            &config.into(),
            target_sample_rate,
            target_channels,
            duration,
        ),
        SampleFormat::U16 => record::<u16>(
            &device,
            &config.into(),
            target_sample_rate,
            target_channels,
            duration,
        ),
        SampleFormat::F32 => record::<f32>(
            &device,
            &config.into(),
            target_sample_rate,
            target_channels,
            duration,
        ),
        _ => Err("Unsupported sample format".into()),
    }
}

fn find_input_device(host: &Host, device_name: &str) -> Result<Device, Box<dyn std::error::Error>> {
    // Try to find device by name
    for device in host.input_devices()? {
        if device.name()?.contains(device_name) {
            return Ok(device);
        }
    }

    // If not found, try default device
    println!(
        "Device '{}' not found, using default input device",
        device_name
    );
    host.default_input_device()
        .ok_or_else(|| "No input device available".into())
}

fn record<T>(
    device: &Device,
    config: &StreamConfig,
    target_sample_rate: u32,
    target_channels: u16,
    duration: Duration,
) -> Result<Vec<i16>, Box<dyn std::error::Error>>
where
    T: cpal::Sample<Float = f32> + cpal::SizedSample + Send + 'static,
{
    let audio_data: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let audio_data_clone = audio_data.clone();

    let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

    // Build the input stream
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let mut buffer = audio_data_clone.lock().unwrap();
            for &sample in data {
                // Convert sample to i16 and resample if needed
                let i16_sample = sample_to_i16(sample);
                buffer.push(i16_sample);
            }
        },
        err_fn,
        None,
    )?;

    // Start recording
    stream.play()?;

    // Record for specified duration
    std::thread::sleep(duration);

    // Stop recording
    drop(stream);

    let mut audio_data = Arc::try_unwrap(audio_data).unwrap().into_inner()?;

    // Resample to target sample rate if needed
    if config.sample_rate.0 != target_sample_rate {
        audio_data = resample(&audio_data, config.sample_rate.0, target_sample_rate);
    }

    // Convert to target number of channels if needed
    if config.channels != target_channels {
        audio_data = convert_channels(
            &audio_data,
            config.channels as usize,
            target_channels as usize,
        );
    }

    Ok(audio_data)
}

fn sample_to_i16<T>(sample: T) -> i16
where
    T: cpal::Sample<Float = f32>,
{
    // Convert sample to f32 first, then to i16
    let f32_sample = sample.to_float_sample();
    (f32_sample * i16::MAX as f32) as i16
}

fn resample(audio_data: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    // Simple linear interpolation resampling
    if from_rate == to_rate {
        return audio_data.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (audio_data.len() as f64 / ratio) as usize;
    let mut resampled = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = (i as f64 * ratio) as usize;
        if src_pos < audio_data.len() - 1 {
            let frac = (i as f64 * ratio) - src_pos as f64;
            let sample =
                audio_data[src_pos] as f64 * (1.0 - frac) + audio_data[src_pos + 1] as f64 * frac;
            resampled.push(sample as i16);
        } else {
            resampled.push(audio_data[audio_data.len() - 1]);
        }
    }

    resampled
}

fn convert_channels(audio_data: &[i16], from_channels: usize, to_channels: usize) -> Vec<i16> {
    if from_channels == to_channels {
        return audio_data.to_vec();
    }

    // Convert from stereo to mono
    if from_channels == 2 && to_channels == 1 {
        return audio_data
            .chunks(2)
            .map(|chunk| {
                let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                (sum / 2) as i16
            })
            .collect();
    }

    // Convert from mono to stereo (duplicate samples)
    if from_channels == 1 && to_channels == 2 {
        let mut result = Vec::with_capacity(audio_data.len() * 2);
        for &sample in audio_data {
            result.push(sample);
            result.push(sample);
        }
        return result;
    }

    // For other conversions, just take the first channel
    audio_data
        .chunks(from_channels)
        .flat_map(|chunk| std::iter::repeat(chunk[0]).take(to_channels))
        .collect()
}
