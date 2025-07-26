use crate::bindings::golem::llm::llm;

pub fn consume_next_event(stream: &llm::ChatStream) -> Option<String> {
    let events = stream.blocking_get_next();

    if events.is_empty() {
        return None;
    }

    let mut result = String::new();

    for event in events {
        println!("Received {event:?}");

        match event {
            llm::StreamEvent::Delta(delta) => {
                for content in delta.content.unwrap_or_default() {
                    match content {
                        llm::ContentPart::Text(txt) => {
                            result.push_str(&txt);
                        }
                        llm::ContentPart::Image(image_ref) => match image_ref {
                            llm::ImageReference::Url(url_data) => {
                                result.push_str(&format!(
                                    "IMAGE URL: {} ({:?})\n",
                                    url_data.url, url_data.detail
                                ));
                            }
                            llm::ImageReference::Inline(inline_data) => {
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
            llm::StreamEvent::Finish(..) => {}
            llm::StreamEvent::Error(error) => {
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
