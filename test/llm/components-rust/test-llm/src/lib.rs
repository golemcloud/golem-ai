use golem_ai_llm::model::*;
use golem_ai_llm::LlmProvider;
use golem_rust::{agent_definition, agent_implementation, mark_atomic_operation};

#[agent_definition]
pub trait TestHelper {
    fn new(name: String) -> Self;
    fn inc_and_get(&mut self) -> u64;
}

struct TestHelperImpl {
    _name: String,
    total: u64,
}

#[agent_implementation]
impl TestHelper for TestHelperImpl {
    fn new(name: String) -> Self {
        Self {
            _name: name,
            total: 0,
        }
    }

    fn inc_and_get(&mut self) -> u64 {
        self.total += 1;
        self.total
    }
}

mod utils;

#[cfg(feature = "openai")]
type Provider = golem_ai_llm_openai::DurableOpenAI;
#[cfg(feature = "anthropic")]
type Provider = golem_ai_llm_anthropic::DurableAnthropic;
#[cfg(feature = "bedrock")]
type Provider = golem_ai_llm_bedrock::DurableBedrock;
#[cfg(feature = "grok")]
type Provider = golem_ai_llm_grok::DurableGrok;
#[cfg(feature = "openrouter")]
type Provider = golem_ai_llm_openrouter::DurableOpenRouter;
#[cfg(feature = "ollama")]
type Provider = golem_ai_llm_ollama::DurableOllama;

#[cfg(feature = "openai")]
const MODEL: &str = "gpt-3.5-turbo";
#[cfg(feature = "bedrock")]
const MODEL: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";
#[cfg(feature = "anthropic")]
const MODEL: &str = "claude-3-7-sonnet-20250219";
#[cfg(feature = "grok")]
const MODEL: &str = "grok-3-beta";
#[cfg(feature = "openrouter")]
const MODEL: &str = "openrouter/auto";
#[cfg(feature = "ollama")]
const MODEL: &str = "qwen3:1.7b";

#[cfg(feature = "openai")]
const IMAGE_MODEL: &str = "gpt-4o-mini";
#[cfg(feature = "bedrock")]
const IMAGE_MODEL: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";
#[cfg(feature = "anthropic")]
const IMAGE_MODEL: &str = "claude-3-7-sonnet-20250219";
#[cfg(feature = "grok")]
const IMAGE_MODEL: &str = "grok-2-vision-latest";
#[cfg(feature = "openrouter")]
const IMAGE_MODEL: &str = "openrouter/auto";
#[cfg(feature = "ollama")]
const IMAGE_MODEL: &str = "gemma3:4b";

#[cfg(feature = "openai")]
fn provider_passthrough_options() -> Vec<Kv> {
    vec![
        Kv {
            key: "max_output_tokens".to_string(),
            value: "64".to_string(),
        },
        Kv {
            key: "store".to_string(),
            value: "false".to_string(),
        },
    ]
}

#[cfg(feature = "anthropic")]
fn provider_passthrough_options() -> Vec<Kv> {
    vec![Kv {
        key: "metadata".to_string(),
        value: r#"{"user_id":"passthrough-test","session":"golem-ai"}"#.to_string(),
    }]
}

#[cfg(feature = "grok")]
fn provider_passthrough_options() -> Vec<Kv> {
    vec![Kv {
        key: "max_completion_tokens".to_string(),
        value: "64".to_string(),
    }]
}

#[cfg(feature = "openrouter")]
fn provider_passthrough_options() -> Vec<Kv> {
    vec![
        Kv {
            key: "max_tokens".to_string(),
            value: "64".to_string(),
        },
        Kv {
            key: "models".to_string(),
            value: "[\"openai/gpt-4o-mini\"]".to_string(),
        },
    ]
}

#[cfg(feature = "ollama")]
fn provider_passthrough_options() -> Vec<Kv> {
    vec![Kv {
        key: "options".to_string(),
        value: r#"{"use_mlock":true}"#.to_string(),
    }]
}

#[cfg(feature = "bedrock")]
fn provider_passthrough_options() -> Vec<Kv> {
    vec![Kv {
        key: "top_k".to_string(),
        value: "20".to_string(),
    }]
}

#[agent_definition]
pub trait LlmTest {
    fn new(name: String) -> Self;

    fn test1(&self) -> String;
    fn test2(&self) -> String;
    fn test3(&self) -> String;
    fn test4(&self) -> String;
    fn test5(&self) -> String;
    async fn test6(&self) -> String;
    fn test7(&self) -> String;
    async fn test8(&self) -> String;
    fn test9(&self) -> String;
}

struct LlmTestImpl {
    _name: String,
}

#[agent_implementation]
impl LlmTest for LlmTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    fn test1(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.2),
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: None,
        };

        println!("Sending request to LLM...");
        let response = Provider::send(
            vec![Event::Message(Message {
                role: Role::User,
                name: Some("vigoo".to_string()),
                content: vec![ContentPart::Text(
                    "What is the usual weather on the Vršič pass in the beginning of May?"
                        .to_string(),
                )],
            })],
            config,
        );
        println!("Response: {:?}", response);

        match response {
            Ok(response) => {
                format!(
                    "{}, {:?}",
                    response
                        .content
                        .into_iter()
                        .map(|content| match content {
                            ContentPart::Text(txt) => txt,
                            ContentPart::Image(image_ref) => match image_ref {
                                ImageReference::Url(url_data) =>
                                    format!("[IMAGE URL: {}]", url_data.url),
                                ImageReference::Inline(inline_data) => format!(
                                    "[INLINE IMAGE: {} bytes, mime: {}]",
                                    inline_data.data.len(),
                                    inline_data.mime_type
                                ),
                            },
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    response.tool_calls
                )
            }
            Err(error) => {
                format!(
                    "ERROR: {:?} {} ({})",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    fn test2(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.2),
            max_tokens: None,
            stop_sequences: None,
            tools: Some(vec![ToolDefinition {
                name: "test-tool".to_string(),
                description: Some("Test tool for generating test values".to_string()),
                parameters_schema: r#"{
                        "type": "object",
                        "properties": {
                            "maximum": {
                                "type": "number",
                                "description": "Upper bound for the test value"
                            }
                        },
                        "required": [
                            "maximum"
                        ],
                        "additionalProperties": false
                    }"#
                .to_string(),
            }]),
            tool_choice: Some("auto".to_string()),
            provider_options: None,
        };

        let input = vec![
            ContentPart::Text("Generate a random number between 1 and 10".to_string()),
            ContentPart::Text(
                "then translate this number to German and output it as a text message.".to_string(),
            ),
        ];

        let mut events = vec![];
        events.push(Event::Message(Message {
            role: Role::User,
            name: Some("vigoo".to_string()),
            content: input.clone(),
        }));

        println!("Sending request to LLM...");
        let response1 = Provider::send(events.clone(), config.clone());
        let tool_request = match response1 {
            Ok(response) => {
                events.push(Event::Response(response.clone()));
                response.tool_calls
            }
            Err(error) => {
                println!(
                    "ERROR: (1) {:?} {} ({})",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                );
                vec![]
            }
        };

        if !tool_request.is_empty() {
            for call in tool_request {
                events.push(Event::ToolResults(vec![ToolResult::Success(
                    ToolSuccess {
                        id: call.id,
                        name: call.name,
                        result_json: r#"{ "value": 6 }"#.to_string(),
                        execution_time_ms: None,
                    },
                )]));
            }

            let response2 = Provider::send(events, config);

            match response2 {
                Ok(response) => {
                    format!("Response 2: {:?}", response)
                }
                Err(error) => {
                    format!(
                        "ERROR: (2) {:?} {} ({})",
                        error.code,
                        error.message,
                        error.provider_error_json.unwrap_or_default()
                    )
                }
            }
        } else {
            "No tool request".to_string()
        }
    }

    fn test3(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.2),
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: None,
        };

        println!("Starting streaming request to LLM...");
        let stream = Provider::stream(
            vec![Event::Message(Message {
                role: Role::User,
                name: Some("vigoo".to_string()),
                content: vec![ContentPart::Text(
                    "What is the usual weather on the Vršič pass in the beginning of May?"
                        .to_string(),
                )],
            })],
            config,
        );

        let mut result = String::new();

        loop {
            let events = stream.get_next();
            if events.is_empty() {
                break;
            }

            for event in events {
                println!("Received {event:?}");

                match event {
                    Ok(StreamEvent::Delta(delta)) => {
                        result.push_str(&format!("DELTA: {:?}\n", delta));
                    }
                    Ok(StreamEvent::Finish(finish)) => {
                        result.push_str(&format!("FINISH: {:?}\n", finish));
                    }
                    Err(error) => {
                        result.push_str(&format!(
                            "ERROR: {:?} {} ({})\n",
                            error.code,
                            error.message,
                            error.provider_error_json.unwrap_or_default()
                        ));
                    }
                }
            }
        }

        result
    }

    fn test4(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.2),
            max_tokens: None,
            stop_sequences: None,
            tools: Some(vec![ToolDefinition {
                name: "test-tool".to_string(),
                description: Some("Test tool for generating test values".to_string()),
                parameters_schema: r#"{
                        "type": "object",
                        "properties": {
                            "maximum": {
                                "type": "number",
                                "description": "Upper bound for the test value"
                            }
                        },
                        "required": [
                            "maximum"
                        ],
                        "additionalProperties": false
                    }"#
                .to_string(),
            }]),
            tool_choice: Some("auto".to_string()),
            provider_options: None,
        };

        let input = vec![
            ContentPart::Text("Generate a random number between 1 and 10".to_string()),
            ContentPart::Text(
                "then translate this number to German and output it as a text message.".to_string(),
            ),
        ];

        println!("Starting streaming request to LLM...");
        let stream = Provider::stream(
            vec![Event::Message(Message {
                role: Role::User,
                name: Some("vigoo".to_string()),
                content: input,
            })],
            config,
        );

        let mut result = String::new();

        loop {
            let events = stream.get_next();

            if events.is_empty() {
                break;
            }

            for event in events {
                println!("Received {event:?}");

                match event {
                    Ok(StreamEvent::Delta(delta)) => {
                        result.push_str(&format!("DELTA: {:?}\n", delta));
                    }
                    Ok(StreamEvent::Finish(finish)) => {
                        result.push_str(&format!("FINISH: {:?}\n", finish));
                    }
                    Err(error) => {
                        result.push_str(&format!(
                            "ERROR: {:?} {} ({})\n",
                            error.code,
                            error.message,
                            error.provider_error_json.unwrap_or_default()
                        ));
                        break;
                    }
                }
            }
        }

        result
    }

    fn test5(&self) -> String {
        let config = Config {
            model: IMAGE_MODEL.to_string(),
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: None,
        };

        println!("Sending request to LLM...");
        let response = Provider::send(
            vec![
                Event::Message(Message {
                    role: Role::User,
                    name: None,
                    content: vec![
                        ContentPart::Text("What is on this image?".to_string()),
                        ContentPart::Image(ImageReference::Url(ImageUrl {
                            url: "https://blog.vigoo.dev/images/blog-zio-kafka-debugging-3.png"
                                .to_string(),
                            detail: Some(ImageDetail::High),
                        })),
                    ],
                }),
                Event::Message(Message {
                    role: Role::System,
                    name: None,
                    content: vec![ContentPart::Text(
                        "Produce the output in both English and Hungarian".to_string(),
                    )],
                }),
            ],
            config,
        );
        println!("Response: {:?}", response);

        match response {
            Ok(response) => {
                format!(
                    "{}, {:?}",
                    response
                        .content
                        .into_iter()
                        .map(|content| match content {
                            ContentPart::Text(txt) => txt,
                            ContentPart::Image(image_ref) => match image_ref {
                                ImageReference::Url(url_data) =>
                                    format!("[IMAGE URL: {}]", url_data.url),
                                ImageReference::Inline(inline_data) => format!(
                                    "[INLINE IMAGE: {} bytes, mime: {}]",
                                    inline_data.data.len(),
                                    inline_data.mime_type
                                ),
                            },
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    response.tool_calls
                )
            }
            Err(error) => {
                format!(
                    "ERROR: {:?} {} ({})",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    async fn test6(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.2),
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: None,
        };

        println!("Starting streaming request to LLM...");
        let stream = Provider::stream(
            vec![Event::Message(Message {
                role: Role::User,
                name: Some("vigoo".to_string()),
                content: vec![ContentPart::Text(
                    "What is the usual weather on the Vršič pass in the beginning of May?"
                        .to_string(),
                )],
            })],
            config,
        );

        let mut result = String::new();

        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let mut round = 0;

        loop {
            let events = stream.get_next();

            if events.is_empty() {
                break;
            }

            for event in events {
                println!("Received {event:?}");

                match event {
                    Ok(StreamEvent::Delta(delta)) => {
                        for content in delta.content.unwrap_or_default() {
                            match content {
                                ContentPart::Text(txt) => {
                                    result.push_str(&txt);
                                }
                                ContentPart::Image(image_ref) => match image_ref {
                                    ImageReference::Url(url_data) => {
                                        result.push_str(&format!(
                                            "IMAGE URL: {} ({:?})\n",
                                            url_data.url, url_data.detail
                                        ));
                                    }
                                    ImageReference::Inline(inline_data) => {
                                        result.push_str(&format!(
                                            "INLINE IMAGE: {} bytes, mime: {}, detail: {:?}\n",
                                            inline_data.data.len(),
                                            inline_data.mime_type,
                                            inline_data.detail
                                        ));
                                    }
                                },
                            }
                        }
                    }
                    Ok(StreamEvent::Finish(finish)) => {
                        result.push_str(&format!("\nFINISH: {:?}\n", finish));
                    }
                    Err(error) => {
                        result.push_str(&format!(
                            "\nERROR: {:?} {} ({})\n",
                            error.code,
                            error.message,
                            error.provider_error_json.unwrap_or_default()
                        ));
                        break;
                    }
                }
            }

            if round == 2 {
                let _guard = mark_atomic_operation();
                let mut client = TestHelperClient::get(name.clone());
                let answer = client.inc_and_get().await;
                if answer == 1 {
                    panic!("Simulating crash")
                }
            }

            round += 1;
        }

        result
    }

    fn test7(&self) -> String {
        use std::fs::File;
        use std::io::Read;

        let config = Config {
            model: IMAGE_MODEL.to_string(),
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: None,
        };

        println!("Reading image from Initial File System...");
        let mut file = match File::open("/data/cat.png") {
            Ok(file) => file,
            Err(err) => return format!("ERROR: Failed to open cat.png: {}", err),
        };

        let mut buffer = Vec::new();
        match file.read_to_end(&mut buffer) {
            Ok(_) => println!("Successfully read {} bytes from cat.png", buffer.len()),
            Err(err) => return format!("ERROR: Failed to read cat.png: {}", err),
        }

        println!("Sending request to LLM with inline image...");
        let response = Provider::send(
            vec![Event::Message(Message {
                role: Role::User,
                name: None,
                content: vec![
                    ContentPart::Text(
                        "Please describe this cat image in detail. What breed might it be?"
                            .to_string(),
                    ),
                    ContentPart::Image(ImageReference::Inline(ImageSource {
                        data: buffer,
                        mime_type: "image/png".to_string(),
                        detail: None,
                    })),
                ],
            })],
            config,
        );
        println!("Response: {:?}", response);

        match response {
            Ok(response) => {
                format!(
                    "{}, {:?}",
                    response
                        .content
                        .into_iter()
                        .map(|content| match content {
                            ContentPart::Text(txt) => txt,
                            ContentPart::Image(image_ref) => match image_ref {
                                ImageReference::Url(url_data) =>
                                    format!("[IMAGE URL: {}]", url_data.url),
                                ImageReference::Inline(inline_data) => format!(
                                    "[INLINE IMAGE: {} bytes, mime: {}]",
                                    inline_data.data.len(),
                                    inline_data.mime_type
                                ),
                            },
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    response.tool_calls
                )
            }
            Err(error) => {
                format!(
                    "ERROR: {:?} {} ({})",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    async fn test8(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.2),
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: None,
        };

        let mut events = vec![Event::Message(Message {
            role: Role::User,
            name: Some("vigoo".to_string()),
            content: vec![ContentPart::Text(
                "Do you know what a haiku is?".to_string(),
            )],
        })];

        let stream = Provider::stream(events.clone(), config.clone());

        let mut result = String::new();

        loop {
            match utils::consume_next_event(&stream) {
                Some(delta) => {
                    result.push_str(&delta);
                }
                None => break,
            }
        }

        events.push(Event::Message(Message {
            role: Role::Assistant,
            name: Some("assistant".to_string()),
            content: vec![ContentPart::Text(result)],
        }));

        events.push(Event::Message(Message {
            role: Role::User,
            name: Some("vigoo".to_string()),
            content: vec![ContentPart::Text(
                "Can you write one for me?".to_string(),
            )],
        }));

        println!("Message: {events:?}");

        let stream = Provider::stream(events, config);

        let mut result = String::new();

        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let mut round = 0;

        loop {
            match utils::consume_next_event(&stream) {
                Some(delta) => {
                    result.push_str(&delta);
                }
                None => break,
            }

            if round == 2 {
                let _guard = mark_atomic_operation();
                let mut client = TestHelperClient::get(name.clone());
                let answer = client.inc_and_get().await;
                if answer == 1 {
                    panic!("Simulating crash")
                }
            }

            round += 1;
        }

        result
    }

    fn test9(&self) -> String {
        let config = Config {
            model: MODEL.to_string(),
            temperature: Some(0.1),
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options: Some(provider_passthrough_options()),
        };

        println!(
            "Sending request with passthrough options: {:?}",
            config.provider_options
        );

        let response = Provider::send(
            vec![Event::Message(Message {
                role: Role::User,
                name: None,
                content: vec![ContentPart::Text(
                    "Reply with the text: passthrough-ok".to_string(),
                )],
            })],
            config,
        );

        match response {
            Ok(response) => {
                let text = response
                    .content
                    .into_iter()
                    .filter_map(|part| match part {
                        ContentPart::Text(value) => Some(value),
                        ContentPart::Image(_) => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                format!(
                    "content={text}; metadata={:?}; tool_calls={:?}",
                    response.metadata, response.tool_calls
                )
            }
            Err(error) => format!(
                "ERROR: {:?} {} ({})",
                error.code,
                error.message,
                error.provider_error_json.unwrap_or_default()
            ),
        }
    }
}
