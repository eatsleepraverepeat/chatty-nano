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
use std::time::Duration;
use tokio::time::sleep;

/// Send the transcription to the LLM and stream the response to stdout.
pub async fn get_ai_response(
    api_key: Option<&str>,
    endpoint: &str,
    model_name: &str,
    transcription: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Set API key from args or environment variable
    let api_key = api_key
        .map(str::to_owned)
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .ok_or(
            "OpenAI API key not provided via --openai-api-key or OPENAI_API_KEY environment variable",
        )?;

    // Create client with custom endpoint
    let client = Client::with_config(
        async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(endpoint.to_string()),
    );

    // Create chat completion request with streaming enabled
    let request = CreateChatCompletionRequestArgs::default()
        .model(model_name)
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

    // Display streaming response with typing effect
    print!("AI Response: ");
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

    Ok(())
}
