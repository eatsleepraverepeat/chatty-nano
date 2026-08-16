mod ai;
mod audio;

use clap::Parser;
use coqui_stt::Model;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

    /// Skip sending the transcription to the LLM (transcription-only mode)
    #[arg(long)]
    transcribe_only: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Locate the model and optional scorer files
    let (model_path, scorer_path) = find_model_files(&args.model_dir);

    // Initialize the STT model
    let mut model = Model::new(model_path.to_str().expect("invalid utf-8 found in path"))?;

    // Enable external scorer if found
    if let Some(scorer) = &scorer_path {
        println!("Using external scorer `{}`", scorer.display());
        model.enable_external_scorer(scorer.to_str().expect("invalid utf-8 found in path"))?;
    }

    // Record audio from microphone
    let audio_data = audio::record_audio(
        Duration::from_secs(args.chunk_seconds),
        &args.input_device,
        args.num_channels,
        args.sample_rate,
    )?;

    // Transcribe
    let start = Instant::now();
    let transcription = model.speech_to_text(&audio_data)?;
    let duration = start.elapsed();
    println!("\nTranscription (took {:?}):\n{}", duration, transcription);

    // Send transcription to the LLM and stream the response
    if !args.transcribe_only && !transcription.is_empty() {
        ai::get_ai_response(
            args.openai_api_key.as_deref(),
            &args.openai_endpoint,
            &args.openai_model,
            &transcription,
        )
        .await?;
    }

    Ok(())
}

/// Locate the model (.pb, .pbmm, or .tflite) and optional scorer (.scorer) files.
fn find_model_files(dir: &str) -> (PathBuf, Option<PathBuf>) {
    let mut model = None;
    let mut scorer = None;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some("pb") | Some("pbmm") | Some("tflite") => model = Some(path),
                Some("scorer") => scorer = Some(path),
                _ => {}
            }
        }
    }

    let model = model.expect("No model file found (.pb, .pbmm, or .tflite)");
    (model, scorer)
}
