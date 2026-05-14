mod async_utils;
mod client;
pub mod config;
mod conversions;
mod stream;
mod wasi_client;

use client::Bedrock as BedrockClient;
use golem_ai_llm::durability::{DurableLLM, ExtendedLlmProvider};
use golem_ai_llm::model::{
    ChatStream, Config, ContentPart, Error, Event, Message, Response, Role, StreamDelta,
};
use golem_ai_llm::wasi_compat::Pollable;
use golem_ai_llm::LlmProvider;
use indoc::indoc;
use stream::BedrockChatStream;

pub use config::BedrockConfig;
#[cfg(feature = "golem")]
pub use config::BedrockHostConfig;

pub struct Bedrock;

impl LlmProvider for Bedrock {
    type ChatStream = BedrockChatStream;
    type ProviderConfig = BedrockConfig;

    async fn send(
        provider_config: Self::ProviderConfig,
        events: Vec<Event>,
        config: Config,
    ) -> Result<Response, Error> {
        let client = BedrockClient::new(&provider_config).await?;
        client.converse(events, config).await
    }

    async fn stream(
        provider_config: Self::ProviderConfig,
        events: Vec<Event>,
        config: Config,
    ) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(provider_config, events, config).await)
    }
}

impl ExtendedLlmProvider for Bedrock {
    async fn unwrapped_stream(
        provider_config: Self::ProviderConfig,
        messages: Vec<Event>,
        config: Config,
    ) -> Self::ChatStream {
        match BedrockClient::new(&provider_config).await {
            Ok(client) => client.converse_stream(messages, config).await,
            Err(err) => BedrockChatStream::failed(err),
        }
    }

    fn retry_prompt(
        original_events: &[Result<Event, Error>],
        partial_result: &[StreamDelta],
    ) -> Vec<Event> {
        let mut extended_events = Vec::new();
        extended_events.push(Event::Message(Message {
            role: Role::System,
            name: None,
            content: vec![ContentPart::Text(indoc! {"
                You were asked the same question previously, but the response was interrupted before completion.
                Please continue your response from where you left off.
                Do not include the part of the response that was already seen.
                If the response starts with a new word and no punctuation then add a space to the beginning."
            }.to_string())],
        }));
        extended_events.push(Event::Message(Message {
            role: Role::User,
            name: None,
            content: vec![ContentPart::Text(
                "Here is the original question:".to_string(),
            )],
        }));
        extended_events.extend(
            original_events
                .iter()
                .filter_map(|e| e.as_ref().ok().cloned()),
        );

        let mut partial_result_as_content = Vec::new();
        for delta in partial_result {
            if let Some(contents) = &delta.content {
                partial_result_as_content.extend_from_slice(contents);
            }
            if let Some(tool_calls) = &delta.tool_calls {
                for tool_call in tool_calls {
                    partial_result_as_content.push(ContentPart::Text(format!(
                        "<tool-call id=\"{}\" name=\"{}\" arguments=\"{}\"/>",
                        tool_call.id, tool_call.name, tool_call.arguments_json,
                    )));
                }
            }
        }

        extended_events.push(Event::Message(Message {
            role: Role::User,
            name: None,
            content: vec![ContentPart::Text(
                "Here is the partial response that was successfully received:".to_string(),
            )]
            .into_iter()
            .chain(partial_result_as_content)
            .collect(),
        }));
        extended_events
    }

    fn subscribe(_stream: &Self::ChatStream) -> Pollable {
        // this function will never get called in the bedrock implementation because of the
        // `golem-ai-llm/nopoll` feature flag.
        golem_ai_llm::wasi_compat::subscribe_zero()
    }
}

pub type DurableBedrock = DurableLLM<Bedrock>;
