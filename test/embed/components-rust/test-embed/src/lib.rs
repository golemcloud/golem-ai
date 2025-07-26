#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::embed_exports::test_embed_api::*;
use crate::bindings::golem::embed::embed;
use crate::bindings::golem::embed::embed::{Config, ContentPart, EmbeddingResponse, Error};
use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Url};
use std::{fs, path::Path};

struct Component;

const IMAGE_URL: &'static str = "https://images.pexels.com/photos/33147349/pexels-photo-33147349.jpeg";

#[cfg(feature = "openai")]
const MODEL: &'static str = "text-embedding-3-small";
#[cfg(feature = "cohere")]
const MODEL: &'static str = "embed-v4.0";
#[cfg(feature = "hugging-face")]
const MODEL: &'static str = "sentence-transformers/all-MiniLM-L6-v2";
#[cfg(feature = "voyageai")]
const MODEL: &'static str = "voyage-3";

#[cfg(feature = "openai")]
const RERANKING_MODEL: &'static str = "";
#[cfg(feature = "cohere")]
const RERANKING_MODEL: &'static str = "rerank-v3.5";
#[cfg(feature = "hugging-face")]
const RERANKING_MODEL: &'static str = "cross-encoder/ms-marco-MiniLM-L-2-v2";
#[cfg(feature = "voyageai")]
const RERANKING_MODEL: &'static str = "rerank-1";

impl Guest for Component {
    /// test1 demonstrates text embedding generation.
    fn test1() -> String {
        let config = Config {
            model: Some(MODEL.to_string()),
            task_type: Some(embed::TaskType::RetrievalDocument),
            dimensions: Some(1024),
            truncation: None,
            output_format: Some(embed::OutputFormat::FloatArray),
            output_dtype: Some(embed::OutputDtype::FloatArray),
            user: Some("RutikThakre".to_string()),
            provider_options: vec![],
        };
        println!("Sending text for embedding generation...");
        let response: Result<EmbeddingResponse, Error> = embed::generate(
            &[ContentPart::Text(
                "Carson City is the capital city of the American state of Nevada.".to_string(),
            )],
            &config,
        );

        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    /// test7 demonstrates reranking functionality.
    fn test2() -> String {
        let config = Config {
            model: Some(RERANKING_MODEL.to_string()),
            task_type: None,
            dimensions: None,
            truncation: None,
            output_format: None,
            output_dtype: None,
            user: None,
            provider_options: vec![],
        };
        let query = "What is machine learning?";
        let documents = vec![
            "Machine learning is a subset of artificial intelligence.".to_string(),
            "The weather today is sunny and warm.".to_string(),
            "AI and ML are transforming various industries.".to_string(),
        ];

        println!("Sending reranking request...");
        let response = embed::rerank(query, &documents, &config);
        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    /// test3 demonstrates image embedding generation.
    fn test3() -> String {
        let config = Config {
            model: Some(MODEL.to_string()),
            task_type: None,
            dimensions: None,
            truncation: None,
            output_format: None,
            output_dtype: None,
            user: Some("RutikThakre".to_string()),
            provider_options: vec![],
        };
        let data = vec![ContentPart::Image(embed::ImageUrl {
            url: IMAGE_URL.to_string(),
        })];

        println!("Sending image for embedding generation...");
        let response: Result<EmbeddingResponse, Error> = embed::generate(&data, &config);

        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    /// test4 demonstrates text + image embedding generation.
    fn test4() -> String {
        let (config, data) = {
            #[cfg(not(feature = "cohere"))]
            {
                let config = Config {
                    model: Some(MODEL.to_string()),
                    task_type: Some(embed::TaskType::RetrievalDocument),
                    dimensions: Some(1024),
                    truncation: Some(true),
                    output_format: Some(embed::OutputFormat::FloatArray),
                    output_dtype: Some(embed::OutputDtype::FloatArray),
                    user: Some("RutikThakre".to_string()),
                    provider_options: vec![],
                };
                let data = vec![
                    ContentPart::Text("A serene mountain landscape at sunrise.".to_string()),
                    ContentPart::Image(embed::ImageUrl {
                        url: IMAGE_URL.to_string(),
                    }),
                ];
                (config, data)
            }
            #[cfg(feature = "cohere")]
            {
                let provider_options = get_cohere_inputs_param().unwrap_or_else(|_| vec![]);
                println!("provider_options: {:?}", provider_options);
                let config = Config {
                    model: Some(MODEL.to_string()),
                    task_type: Some(embed::TaskType::RetrievalDocument),
                    dimensions: Some(1024),
                    truncation: Some(true),
                    output_format: Some(embed::OutputFormat::FloatArray),
                    output_dtype: Some(embed::OutputDtype::FloatArray),
                    user: Some("RutikThakre".to_string()),
                    provider_options,
                };
                let data = vec![];
                (config, data)
            }
        };

        println!("Sending text + image for embedding generation...");
        let response: Result<EmbeddingResponse, Error> = embed::generate(&data, &config);

        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    /// test5 demonstrates embedding generation with default parameters.
    fn test5() -> String {
        let config = Config {
            model: Some(MODEL.to_string()),
            task_type: None,
            dimensions: None,
            truncation: None,
            output_format: None,
            output_dtype: None,
            user: None,
            provider_options: vec![],
        };
        println!("Sending text for embedding generation with default params...");
        let response: Result<EmbeddingResponse, Error> = embed::generate(
            &[ContentPart::Text(
                "Carson City is the capital city of the American state of Nevada.".to_string(),
            )],
            &config,
        );

        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    /// test6 demonstrates embedding generation with provider-specific parameters.
    fn test6() -> String {
        let config = Config {
            model: Some(MODEL.to_string()),
            task_type: Some(embed::TaskType::RetrievalDocument),
            dimensions: Some(1024),
            truncation: None,
            output_format: Some(embed::OutputFormat::FloatArray),
            output_dtype: Some(embed::OutputDtype::FloatArray),
            user: None,
            provider_options: get_embed_provider_options(),
        };
        println!("Sending text for embedding generation with provider-specific params...");
        let response: Result<EmbeddingResponse, Error> = embed::generate(
            &[
                ContentPart::Text(
                    "Machine learning is a subset of artificial intelligence.".to_string(),
                ),
                ContentPart::Text("The weather today is sunny and warm.".to_string()),
                ContentPart::Text("AI and ML are transforming various industries.".to_string()),
            ],
            &config,
        );

        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }

    /// test7 demonstrates reranking with provider-specific parameters.
    fn test7() -> String {
        let config = Config {
            model: Some(RERANKING_MODEL.to_string()),
            task_type: None,
            dimensions: None,
            truncation: None,
            output_format: None,
            output_dtype: None,
            user: None,
            provider_options: get_rerank_provider_options(),
        };
        println!("Sending text for embedding generation with provider-specific params...");
        let query = "What is machine learning?";
        let documents = vec![
            "Machine learning is a subset of artificial intelligence.".to_string(),
            "The weather today is sunny and warm.".to_string(),
            "AI and ML are transforming various industries.".to_string(),
        ];
        let response = embed::rerank(query, &documents, &config);
        match response {
            Ok(response) => {
                format!("Response: {:?}", response)
            }
            Err(error) => {
                format!(
                    "Error: {:?} {} {}",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                )
            }
        }
    }
}

pub fn image_to_base64(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = if Url::parse(source).is_ok() {
        let client = Client::new();
        let response = client.get(source).send()?;
        response.bytes()?.to_vec()
    } else {
        let path = Path::new(source);
        fs::read(path)?
    };

    let kind = infer::get(&bytes).ok_or("Could not determine MIME type")?;
    let mime_type = kind.mime_type();
    println!("mime_type {mime_type}");
    let base64_data = general_purpose::STANDARD.encode(&bytes);
    let data_uri = format!("data:{};base64,{}", mime_type, base64_data);

    Ok(data_uri)
}

fn get_cohere_inputs_param() -> Result<Vec<embed::Kv>, Box<dyn std::error::Error>> {
    let image_base = image_to_base64(IMAGE_URL)?;
    Ok(vec![embed::Kv {
        key: "inputs".to_string(),
        value: format!(
            r#"[ 
                {{
                    "content": [
                        {{"type": "text", "text": "A serene mountain landscape at sunrise."}},
                        {{"type": "image_url", "image_url": {{"url": "{image_base}"}}}}
                    ]
                }},
                {{
                    "content": [
                        {{"type": "text", "text": "A serene mountain landscape at sunrise."}}
                    ]
                }},
                {{
                    "content": [
                        {{"type": "image_url", "image_url": {{"url": "{image_base}"}}}}
                    ]
                }}
            ]"#
        ),
    }])
}

fn get_embed_provider_options() -> Vec<embed::Kv> {
    #[cfg(feature = "openai")]
    {
        return vec![embed::Kv {
            key: "user".to_string(),
            value: "RutikThakre".to_string(),
        }];
    }
    #[cfg(feature = "cohere")]
    {
        return vec![
            embed::Kv {
                key: "truncate".to_string(),
                value: "END".to_string(),
            },
            embed::Kv {
                key: "max_tokens".to_string(),
                value: "100".to_string(),
            },
        ];
    }
    #[cfg(feature = "hugging-face")]
    {
        return vec![
            embed::Kv {
                key: "normalize".to_string(),
                value: "true".to_string(),
            },
            embed::Kv {
                key: "prompt_name".to_string(),
                value: "test".to_string(),
            },
        ];
    }
    #[cfg(feature = "voyageai")]
    {
        return vec![embed::Kv {
            key: "truncation".to_string(),
            value: "true".to_string(),
        }];
    }
}

fn get_rerank_provider_options() -> Vec<embed::Kv> {
    // OpenAI does not support reranking.
    #[cfg(feature = "openai")]
    {
        return vec![];
    }
    #[cfg(feature = "cohere")]
    {
        return vec![embed::Kv {
            key: "top_n".to_string(),
            value: "2".to_string(),
        }];
    }
    // HuggingFace does have rerank api.
    #[cfg(feature = "hugging-face")]
    {
        return vec![embed::Kv {
            key: "hugging_face_api_key".to_string(),
            value: "your_hugging_face_api_key".to_string(),
        }];
    }
    #[cfg(feature = "voyageai")]
    {
        return vec![
            embed::Kv {
                key: "return_documents".to_string(),
                value: "true".to_string(),
            },
            embed::Kv {
                key: "top_k".to_string(),
                value: "2".to_string(),
            },
        ];
    }
}

bindings::export!(Component with_types_in bindings);
