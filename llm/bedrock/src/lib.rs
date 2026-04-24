use async_utils::get_async_runtime;
use client::Bedrock as BedrockClient;
use golem_ai_llm::durability::{DurableLLM, ExtendedLlmProvider};
use golem_ai_llm::model::{
    ChatStream, Config, ContentPart, Error, Event, Message, Response, Role, StreamDelta,
};
use golem_ai_llm::LlmProvider;
use golem_rust::bindings::wasi::clocks::monotonic_clock;
use indoc::indoc;
use stream::BedrockChatStream;

mod async_utils;
mod client;
mod conversions;
mod stream;
mod wasi_client;

pub struct Bedrock;

impl LlmProvider for Bedrock {
    type ChatStream = BedrockChatStream;

    fn send(events: Vec<Event>, config: Config) -> Result<Response, Error> {
        let runtime = get_async_runtime();

        runtime.block_on(async {
            let client = get_bedrock_client().await?;
            client.converse(events, config).await
        })
    }

    fn stream(events: Vec<Event>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(events, config))
    }
}

impl ExtendedLlmProvider for Bedrock {
    fn unwrapped_stream(messages: Vec<Event>, config: Config) -> Self::ChatStream {
        let runtime = get_async_runtime();

        runtime.block_on(async {
            let bedrock = get_bedrock_client().await;

            match bedrock {
                Ok(client) => client.converse_stream(messages, config).await,
                Err(err) => BedrockChatStream::failed(err),
            }
        })
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

    fn subscribe(_stream: &Self::ChatStream) -> golem_rust::golem_wasm::Pollable {
        // this function will never get called in bedrock implementation because of `golem-llm/nopoll` feature flag
        monotonic_clock::subscribe_duration(0)
    }
}

async fn get_bedrock_client() -> Result<BedrockClient, Error> {
    BedrockClient::new().await
}

pub type DurableBedrock = DurableLLM<Bedrock>;
