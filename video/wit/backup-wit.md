package golem:video-generation

interface types {
  variant video-error {
    invalid-input(string),
    unsupported-feature(string),
    quota-exceeded,
    generation-failed(string),
    cancelled,
    internal-error(string),
  }

  variant media-input {
    text(string),
    image(reference),
  }

// Added prompt
  record reference {
    data: input-image,
    prompt: option<string>,
    role: option<image-role>,
  }

// Changed to first and last
  enum image-role {
    first,
    last,
  }

  record input-image {
   data: media-data,
  }
  record base-video {
    data: media-data,
  }

  record narration {
    data: media-data,
  }

  variant media-data {
    url(string),
    bytes(list<u8>),
  }

  record generation-config {
    negative-prompt: option<string>,
    seed: option<u64>,
    scheduler: option<string>,
    guidance-scale: option<f32>,
    aspect-ratio: option<aspect-ratio>,
    duration-seconds: option<f32>,
    resolution: option<resolution>,
    enable-audio: option<bool>,
    enhance-prompt: option<bool>,
    provider-options: list<kv>,
    ///Added model and lastframe (Kling Only)
    model: option<string>,
    lastframe: option<input-image: media-data>,
  }

  enum aspect-ratio {
    square,
    portrait,
    landscape,
    cinema,
  }

  enum resolution {
    sd,
    hd,
    fhd,
    uhd,
  }

  record kv {
    key: string,
    value: string,
  }

    record video {
    uri: option<string>,
    base64-bytes: option<list<u8>>,
    mime-type: string,
    width: option<u32>,
    height: option<u32>,
    fps: option<f32>,
    duration-seconds: option<f32>,
  }

  variant job-status {
    pending,
    running,
    succeeded,
    failed(string),
  }

  record video-result {
    status: job-status,
    videos: option<list<video>>,
    metadata: option<list<kv>>,
  }
}

interface video-generation {
  use types.{media-input, generation-config, video-result, video-error};
  
  // changed output from string to result<string, video-error>
  // easier to pass input-invalid, generation error
  // for all generate func
  generate: func(input: media-input, config: generation-config) -> result<string, video-error>;
  poll: func(job-id: string) -> result<video-result, video-error>;
  cancel: func(job-id: string) -> result<string, video-error>;
}

interface lip-sync {
  use types.{video-error, media-data};

// Define the two possible audio source, using voice-id or input audio
  variant audio-source {
    from-text(text: string, voice-id: option<string>, speed: u32),
    from-audio(narration-audio: media-data),
  }

  generate: func(
    input: (base-video: media-data),
    audio: audio-source,
  ) -> result<string, video-error>;

  record voice-info {
    voice-id: string,
    name: string,
    language: string,
    gender: option<string>,
    preview-url: option<string>,
  }

  list-voices: func(language: option<string>) -> result<list<voice-info>, video-error>;
}

interface advanced {
    use types.{video-error, kv};

    // Supported in Kling and veo
    extend-video: func(
        input: base-video,
        prompt: option<string>,
        duration: option<f32>,
    ) -> result<string, video-error>;

    // Supported in runway
    upscale-video: func(
        input: base-video,
    ) -> result<string, video-error>;

    // Supported in kling only
    video-effects: func(
        input: input-image,
        second-image: input-image,
        effect: string,
    ) -> result<string, video-error>;
    
    // Multi image generation, kling Only
    multi-image-generation: func(
        input: input-image,
        other-images: list<input-image>, //Upto max 3 more
        config: generation-config,
    ) -> result<string, video-error>;
}

// I have left this as is, I would like a clarification for this
// I also dont get why no introspection
interface templates {
  use types.{video-error, kv};
  generate-from-template: func(
    template-id: string,
    variables: list<kv>
  ) -> string;
}

world video-generation {
  import types;
  import video-generation;
  import lip-sync;
  import advanced;
  import templates;

  export api: video-generation;
  export lip-sync;
  export template-videos: templates;
  export video-effects: effects;
}
