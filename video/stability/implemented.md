# Stability AI Video Implementation

## ✅ Implemented Features

### Core Functionality
- **Image-to-Video Generation**: Full support for converting images to videos using Stability's Stable Video Diffusion model
- **UUID-based Job Management**: All jobs return UUIDs that map to either Stability task IDs or stored errors
- **Asynchronous Polling**: Proper polling implementation with status tracking (pending → running → succeeded/failed)
- **Error Handling**: Comprehensive error mapping from HTTP status codes to WIT video errors

### 🆕 Automatic Image Processing
- **Smart Resize & Center Crop**: Automatically processes input images to meet Stability's strict dimension requirements
- **Aspect Ratio Detection**: Uses WIT `aspect_ratio` configuration to determine target format:
  - `AspectRatio::Square` → 768x768 (1:1)
  - `AspectRatio::Portrait` → 576x1024 (9:16)  
  - `AspectRatio::Landscape` / `AspectRatio::Cinema` / `None` → 1024x576 (16:9)
- **Center Cropping Logic**: Intelligently crops images to preserve most important content
- **High-Quality Resizing**: Uses Lanczos3 filtering for optimal image quality
- **Universal Support**: Works with both URL downloads and base64/binary image data

### Image Input Support
- **Binary Data**: Direct support for `MediaData::Bytes` (raw image data) - now with automatic processing
- **URL Downloads**: Automatic download and processing of images from URLs
- **Format Support**: Handles JPEG and PNG input formats, outputs PNG for API compatibility

### Parameter Mapping
- **Built-in WIT Fields**:
  - `seed` → Stability API `seed` (0-4294967294)
  - `guidance_scale` → Stability API `cfg_scale` (0.0-10.0)
  - `aspect_ratio` → **NEW**: Automatic dimension mapping and image processing
- **Provider-Specific Options**:
  - `motion_bucket_id` (1-255) - Controls motion intensity in generated videos

### Technical Implementation
- **WASM-Compatible HTTP**: Custom multipart/form-data construction for Golem Cloud's reqwest
- **Image Processing**: Built-in `image` crate integration for resize/crop operations
- **Durability Integration**: Full integration with golem-video durability system
- **Parameter Validation**: Range validation according to Stability API specifications
- **Logging**: Enhanced logging with image processing details and configurable levels

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

### Fully Supported WIT Fields
- ✅ `aspect_ratio` - **NEW**: Automatically processed and mapped to correct Stability dimensions
- ✅ `seed` - Directly supported (0-4294967294)
- ✅ `guidance_scale` - Mapped to `cfg_scale` (0.0-10.0)

### Legacy Notes
- `resolution` - Now handled automatically by `aspect_ratio` mapping

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
  - `debug` level shows detailed image processing information

## ⚠️ Known Caveats

### Image Processing
- **Quality vs Size**: Uses Lanczos3 filtering for best quality but increases processing time
- **Center Crop Behavior**: Always crops from center - may not preserve subject focus in all cases
- **Memory Usage**: Temporarily loads full image into memory during processing
- **Format Limitation**: Outputs PNG regardless of input format (API requirement)

### Error Handling
- Invalid API key errors are handled through the job UUID system
- Network failures during generation create error UUIDs
- Rate limiting (429) maps to `quota-exceeded` error
- **NEW**: Image processing errors map to `invalid-input` errors

### Performance
- **NEW**: Image processing adds latency but ensures compatibility
- Image downloads from URLs are synchronous (blocking)
- Large images require more processing time for resize/crop operations
- Poll requests should not exceed once every 10 seconds (API rate limit)

### WASM Environment
- Uses custom multipart body construction (no external dependencies)
- Deterministic boundary generation (no random number generation)
- All HTTP operations use Golem Cloud's custom reqwest implementation
- **NEW**: WASM-compatible image processing with `image` crate

## 🧪 Validation

The implementation includes comprehensive validation:
- **NEW**: Image format validation and processing error handling
- Image data non-empty check
- Parameter range validation (seed, cfg_scale, motion_bucket_id)
- HTTP status code to WIT error mapping
- JSON error response parsing with fallback to raw text

## 🎯 Usage Examples

### Automatic Aspect Ratio Processing

```rust
// Square video (768x768)
GenerationConfig {
    aspect_ratio: Some(AspectRatio::Square),
    // ... other config
}

// Portrait video (576x1024) 
GenerationConfig {
    aspect_ratio: Some(AspectRatio::Portrait),
    // ... other config
}

// Landscape video (1024x576) - default
GenerationConfig {
    aspect_ratio: Some(AspectRatio::Landscape), // or None
    // ... other config
}
```

Any input image will be automatically:
1. **Center cropped** to match the target aspect ratio
2. **Resized** to exact Stability dimensions 
3. **Encoded** as PNG for API compatibility
