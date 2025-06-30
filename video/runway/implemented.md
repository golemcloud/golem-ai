# Runway Video Implementation

## ✅ Implemented Features

### Core Functionality
- **Image-to-Video Generation**: Full support for converting images to videos using Runway's gen3a_turbo and gen4_turbo models
- **UUID-based Job Management**: All jobs return UUIDs that correspond to Runway task IDs
- **Asynchronous Polling**: Proper polling implementation with status tracking (pending → running → succeeded/failed)
- **Error Handling**: Comprehensive error mapping from HTTP status codes to WIT video errors
- **Task Cancellation**: Full support for canceling/deleting tasks via Runway's DELETE endpoint

### Image Input Support
- **URL Input**: Direct support for `MediaData::Url` (image URLs passed directly to API)
- **Binary Data**: Automatic conversion of `MediaData::Bytes` to base64 data URIs
- **Text Prompts**: Support for image prompts via the `ReferenceImage.prompt` field

### Parameter Mapping
- **Built-in WIT Fields**:
  - `seed` → Runway API `seed` (0-4294967295)
  - `model` → Runway API `model` (gen3a_turbo, gen4_turbo)
  - `duration_seconds` → Runway API `duration` (5-10 seconds)
  - `aspect_ratio` → Runway API `ratio` (model-specific resolutions)
- **Provider-Specific Options**:
  - `publicFigureThreshold` → Content moderation setting ("auto" or "low")

### Model Support
- **gen3a_turbo**: 
  - Landscape: 1280:768
  - Portrait: 768:1280
  - Square/Cinema: Falls back to landscape with warning
- **gen4_turbo**:
  - Landscape: 1280:720
  - Portrait: 720:1280
  - Square: 960:960
  - Cinema: 1584:672

### Technical Implementation
- **JSON API**: Uses JSON requests with proper headers (Authorization, X-Runway-Version)
- **Durability Integration**: Full integration with golem-video durability system
- **Parameter Validation**: Range validation according to Runway API specifications
- **Logging**: Proper logging integration with configurable levels

## ✅ Key Differences from Stability

### Additional Features
- **Task Cancellation**: Unlike Stability, Runway supports task cancellation via DELETE endpoint
- **Text Prompts**: Supports text prompts alongside images (via `promptText` field)
- **Duration Control**: Supports video duration between 5-10 seconds
- **Content Moderation**: Configurable public figure detection threshold

### API Design
- **JSON vs Multipart**: Uses JSON requests instead of multipart/form-data
- **URL Support**: Direct URL support without needing to download images first
- **Model Selection**: Explicit model selection with different resolution support per model

## ❌ Limitations & Unsupported Features

### API Limitations
- **No Text-to-Video**: Returns `unsupported-feature` error - only supports image-to-video
- **First Frame Only**: As requested, bypasses "first"/"last" frame positioning - all images treated as first frame

### Unsupported WIT Fields
The following WIT fields are not supported by Runway API and will log warnings:
- `negative_prompt` - Not supported
- `scheduler` - Not supported  
- `guidance_scale` - Not supported
- `enable_audio` - Not supported
- `enhance_prompt` - Not supported

### Partially Supported WIT Fields
- `resolution` - Supported via aspect_ratio mapping to model-specific ratios
- `aspect_ratio` - Mapped to appropriate ratio strings per model

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
1. **Generate**: Returns UUID immediately (Runway task ID)
2. **Poll**: 
   - `JobStatus::Running` while processing (PENDING/RUNNING status)
   - `JobStatus::Succeeded` with video data when complete
   - `JobStatus::Failed` with error message on failure
3. **Cancel**: Supports task cancellation via DELETE endpoint

## 🔧 Configuration

### Required Environment Variables
- `RUNWAY_API_KEY`: Your Runway API key

### Optional Logging
- `GOLEM_VIDEO_LOG`: Set logging level (debug, info, warn, error)

### Provider Options
- `model`: Override default model selection ("gen3a_turbo" or "gen4_turbo")
- `publicFigureThreshold`: Set content moderation level ("auto" or "low")

## ⚠️ Known Caveats

### Error Handling
- Invalid API key errors are handled through standard HTTP error responses
- Network failures during generation return appropriate VideoError types
- Rate limiting maps to `quota-exceeded` error

### Performance
- Video downloads from Runway URLs are synchronous (blocking)
- Large images converted to base64 may increase request size
- Poll requests should respect reasonable intervals to avoid rate limiting

### WASM Environment
- Uses standard JSON serialization (no custom multipart handling needed)
- Base64 encoding for binary image data
- All HTTP operations use Golem Cloud's custom reqwest implementation

## 🧪 Validation

The implementation includes comprehensive validation:
- Model validation (gen3a_turbo, gen4_turbo only)
- Duration range validation (5-10 seconds)
- Seed range validation (0-4294967295)
- HTTP status code to WIT error mapping
- JSON error response parsing with fallback
- Aspect ratio to resolution mapping per model

## 🔄 Comparison with Stability

| Feature | Runway | Stability |
|---------|--------|-----------|
| **Request Format** | JSON | Multipart/form-data |
| **Image Input** | URL + Base64 | Binary only |
| **Text Prompts** | ✅ Supported | ❌ Not supported |
| **Task Cancellation** | ✅ Supported | ❌ Not supported |
| **Duration Control** | ✅ 5-10 seconds | ❌ Fixed duration |
| **Model Selection** | ✅ 2 models | ❌ Single model |
| **Content Moderation** | ✅ Configurable | ❌ Not available |
| **Resolution Options** | Model-dependent | Fixed resolutions | 