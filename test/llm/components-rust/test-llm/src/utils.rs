use golem_ai_llm::model::*;

pub async fn consume_next_event(stream: &ChatStream) -> Option<String> {
    let events = stream.get_next().await;

    if events.is_empty() {
        return None;
    }

    let mut result = String::new();

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
            Ok(StreamEvent::Finish(..)) => {}
            Err(error) => {
                result.push_str(&format!(
                    "\nERROR: {:?} {} ({})\n",
                    error.code,
                    error.message,
                    error.provider_error_json.unwrap_or_default()
                ));
            }
        }
    }

    Some(result)
}
