# Stability AI Video Implementation

## ✅ Implemented Features

### Core Functionality
- **Image-to-Video Generation**: Full support for converting images to videos using Stability's Stable Video Diffusion model
- **UUID-based Job Management**: All jobs return UUIDs that map to either Stability task IDs or stored errors
- **Asynchronous Polling**: Proper polling implementation with status tracking (pending → running → succeeded/failed)
- **Error Handling**: Comprehensive error mapping from HTTP status codes to WIT video errors

### Image Input Support
- **Binary Data**: Direct support for `MediaData::Bytes` (raw image data)
- **URL Downloads**: Automatic download of images from URLs (similar to Ollama implementation)
- **Format Detection**: Automatic content-type handling for multipart requests

### Parameter Mapping
- **Built-in WIT Fields**:
  - `seed` → Stability API `seed` (0-4294967294)
  - `guidance_scale` → Stability API `cfg_scale` (0.0-10.0)
- **Provider-Specific Options**:
  - `motion_bucket_id` (1-255) - Controls motion intensity in generated videos

### Technical Implementation
- **WASM-Compatible HTTP**: Custom multipart/form-data construction for Golem Cloud's reqwest
- **Durability Integration**: Full integration with golem-video durability system
- **Parameter Validation**: Range validation according to Stability API specifications
- **Logging**: Proper logging integration with configurable levels

## ❌ Limitations & Unsupported Features

### API Limitations
- **No Text-to-Video**: Returns `unsupported-feature` error - Stability API only supports image-to-video
- **No Cancellation**: Returns `unsupported-feature` error - Stability API doesn't support job cancellation
- **No Prompt with Image**: Image prompts are ignored (Stability API limitation)

### Unsupported WIT Fields
The following WIT fields are not supported by Stability API and will log warnings:
- `negative_prompt` - Not supported
- `scheduler` - Not supported  
- `duration_seconds` - Not supported
- `enable_audio` - Not supported
- `enhance_prompt` - Not supported

### Partially Supported WIT Fields
- `aspect_ratio` - Supported via input image dimensions (1:1, 16:9, 9:16)
- `resolution` - Supported via input image dimensions (1024x576, 576x1024, 768x768)

## 📊 Output Format

### Video Response
Videos are returned in the WIT format as:
```wit
record video {
  uri: option<string>,           // Always None
  base64-bytes: option<list<u8>>, // Raw video bytes (MP4 format)
  mime-type: string,             // "video/mp4"  
  width: option<u32>,            // None (not provided by API)
  height: option<u32>,           // None (not provided by API)
  fps: option<f32>,              // None (not provided by API)
  duration-seconds: option<f32>, // None (not provided by API)
}
```

### Job Status Flow
1. **Generate**: Returns UUID immediately
2. **Poll**: 
   - `JobStatus::Running` while processing
   - `JobStatus::Succeeded` with video data when complete
   - `JobStatus::Failed` with error message on failure

## 🔧 Configuration

### Required Environment Variables
- `STABILITY_API_KEY`: Your Stability AI API key

### Optional Logging
- `GOLEM_VIDEO_LOG`: Set logging level (debug, info, warn, error)

## ⚠️ Known Caveats

### Error Handling
- Invalid API key errors are handled through the job UUID system
- Network failures during generation create error UUIDs
- Rate limiting (429) maps to `quota-exceeded` error

### Performance
- Image downloads from URLs are synchronous (blocking)
- Large images may take longer to upload due to multipart encoding
- Poll requests should not exceed once every 10 seconds (API rate limit)

### WASM Environment
- Uses custom multipart body construction (no external dependencies)
- Deterministic boundary generation (no random number generation)
- All HTTP operations use Golem Cloud's custom reqwest implementation

## 🧪 Validation

The implementation includes comprehensive validation:
- Image data non-empty check
- Parameter range validation (seed, cfg_scale, motion_bucket_id)
- HTTP status code to WIT error mapping
- JSON error response parsing with fallback to raw text
