use clap::Parser;
use coqui_stt::Model;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, StreamConfig};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_openai::{
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
    },
    Client,
};
use futures::stream::StreamExt;
use std::io::{self, Write};

use tokio::time::sleep;

/// Speech-to-Text application using Coqui STT
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Model directory containing the model files, *.tflite and *.scorer
    #[arg(short, long)]
    model_dir: String,

    /// The length in seconds to wait for audio recording to transcribe
    #[arg(long, default_value = "10")]
    chunk_seconds: u64,

    /// The input device used to record audio
    #[arg(long, default_value = "plughw:CARD=Device,DEV=0")]
    input_device: String,

    /// The number of channels of the recorded audio
    #[arg(long, default_value = "1")]
    num_channels: u16,

    /// The sample rate of the recorded audio
    #[arg(long, default_value = "16000")]
    sample_rate: u32,

    /// OpenAI API endpoint (default: https://api.openai.com/v1)
    #[arg(long, default_value = "https://api.openai.com/v1")]
    openai_endpoint: String,

    /// OpenAI model name (e.g., gpt-4, gpt-3.5-turbo)
    #[arg(long, default_value = "gpt-3.5-turbo")]
    openai_model: String,

    /// OpenAI API key (optional, can be set via OPENAI_API_KEY env var)
    #[arg(long)]
    openai_api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Configuration:");
    println!("  Model directory: {}", args.model_dir);
    println!("  Chunk seconds: {}", args.chunk_seconds);
    println!("  Input device: {}", args.input_device);
    println!("  Number of channels: {}", args.num_channels);
    println!("  Sample rate: {}", args.sample_rate);
    println!("  OpenAI endpoint: {}", args.openai_endpoint);
    println!("  OpenAI model: {}", args.openai_model);
    println!(
        "  OpenAI API key: {}",
        if args.openai_api_key.is_some() {
            "***"
        } else {
            "Not set (will use env var)"
        }
    );
    println!();

    // Find model files
    let dir_path = Path::new(&args.model_dir);
    let mut model_name: Option<Box<Path>> = None;
    let mut scorer_name: Option<Box<Path>> = None;

    for file in dir_path
        .read_dir()
        .expect("Specified model dir is not a dir")
    {
        if let Ok(f) = file {
            let file_path = f.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "pb" || ext == "pbmm" || ext == "tflite" {
                        model_name = Some(file_path.into_boxed_path());
                    } else if ext == "scorer" {
                        scorer_name = Some(file_path.into_boxed_path());
                    }
                }
            }
        }
    }

    let model_path = model_name.expect("No model file found (.pb, .pbmm, or .tflite)");

    // Initialize the STT model
    let mut model = Model::new(model_path.to_str().expect("invalid utf-8 found in path")).unwrap();

    // Enable external scorer if found
    if let Some(ref scorer) = scorer_name {
        let scorer_path = scorer.to_str().expect("invalid utf-8 found in path");
        println!("Using external scorer `{}`", scorer_path);
        model.enable_external_scorer(scorer_path).unwrap();
    }

    println!(
        "Model loaded. Recording for {} seconds...",
        args.chunk_seconds
    );

    // Record audio from microphone
    let audio_data = record_audio(
        Duration::from_secs(args.chunk_seconds),
        &args.input_device,
        args.num_channels,
        args.sample_rate,
    )?;

    println!("Recording complete. Transcribing...");

    // Transcribe
    let start = Instant::now();
    let result = model.speech_to_text(&audio_data)?;
    let duration = start.elapsed();

    println!("\nTranscription:");
    println!("{}", result);
    println!("\nTranscription time: {:?}", duration);

    // Send transcription to OpenAI and get response
    if !result.is_empty() {
        println!("\nSending transcription to AI server...");
        let ai_response = get_ai_response(&args, &result).await?;
        println!("\nAI Response:");
        println!("{}", ai_response);
    }

    Ok(())
}

async fn get_ai_response(
    args: &Args,
    transcription: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Set API key from args or environment variable
    let api_key = args.openai_api_key.clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .ok_or("OpenAI API key not provided via --openai-api-key or OPENAI_API_KEY environment variable")?;

    // Create client with custom endpoint
    let client = Client::with_config(
        async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(args.openai_endpoint.clone()),
    );

    // Create chat completion request with streaming enabled
    let request = CreateChatCompletionRequestArgs::default()
        .model(args.openai_model.clone())
        .messages(vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                name: None,
                content: ChatCompletionRequestSystemMessageContent::Text("You are an AI assistant running on an edge device as part of a speech-to-text (STT) and LLM pipeline. The input you receive is a transcription from an STT model, which may contain errors, missing words, or inaccuracies due to the limitations of speech recognition on edge hardware. Your task is to robustly interpret the user's intent from the transcription, even if some words are missing or incorrect. Respond to the user's queries concisely and helpfully, focusing on understanding the underlying meaning rather than getting caught up in transcription errors.".to_string()),
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                name: None,
                content: ChatCompletionRequestUserMessageContent::Text(transcription.to_string()),
            }),
        ])
        .stream(true)
        .build()?;

    // Send request and get streaming response
    let mut stream = client.chat().create_stream(request).await?;
    let mut full_response = String::new();

    // Display streaming response with TUI effect
    print!("\nAI Response: ");
    io::stdout().flush()?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if let Some(choice) = chunk.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        // Display each character with a small delay for typing effect
                        for ch in content.chars() {
                            print!("{}", ch);
                            io::stdout().flush()?;
                            // Small delay for natural typing effect (adjust as needed)
                            sleep(Duration::from_millis(10)).await;
                        }
                        full_response.push_str(content);
                    }
                }
            }
            Err(e) => {
                eprintln!("\nError in stream: {}", e);
                break;
            }
        }
    }

    println!(); // Add newline after response

    if full_response.is_empty() {
        Ok("No response content".to_string())
    } else {
        Ok(full_response)
    }
}

fn record_audio(
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
    let audio_data = match config.sample_format() {
        SampleFormat::I16 => record::<i16>(
            &device,
            &config.into(),
            target_sample_rate,
            target_channels,
            duration,
        )?,
        SampleFormat::U16 => record::<u16>(
            &device,
            &config.into(),
            target_sample_rate,
            target_channels,
            duration,
        )?,
        SampleFormat::F32 => record::<f32>(
            &device,
            &config.into(),
            target_sample_rate,
            target_channels,
            duration,
        )?,
        _ => return Err("Unsupported sample format".into()),
    };

    Ok(audio_data)
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
        .ok_or("No input device available".into())
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
