# Image Resize & Center Crop for Stability AI

## Overview

The Stability AI video generation API has strict image dimension requirements:
- **Landscape (16:9)**: 1024x576 pixels
- **Portrait (9:16)**: 576x1024 pixels  
- **Square (1:1)**: 768x768 pixels

This module automatically processes input images to meet these requirements using intelligent resize and center crop algorithms.

## How It Works

### 1. Aspect Ratio Detection
The system uses the WIT `aspect_ratio` configuration to determine target format:

```rust
AspectRatio::Square    → 768x768   (1:1)
AspectRatio::Portrait  → 576x1024  (9:16)
AspectRatio::Landscape → 1024x576  (16:9)
AspectRatio::Cinema    → 1024x576  (16:9, mapped)
None                   → 1024x576  (16:9, default)
```

### 2. Center Cropping Algorithm
When input image aspect ratio doesn't match target:

- **Wide images** (wider than target): Crop width from center
- **Tall images** (taller than target): Crop height from center

```
Original: 1920x1080 → Target: 768x768
1. Calculate crop: 1080x1080 (center crop to square)
2. Resize: 768x768 (final dimensions)
```

### 3. High-Quality Resizing
- Uses **Lanczos3 filtering** for optimal quality
- Preserves image details during scaling
- Outputs PNG format for API compatibility

## Processing Pipeline

```
Input Image (any size/format)
         ↓
   Load & Validate
         ↓
   Calculate Crop Dimensions
         ↓
   Center Crop to Target Aspect
         ↓
   Resize to Exact Dimensions
         ↓
   Encode as PNG
         ↓
   Send to Stability API
```

## Supported Input Formats

- **JPEG** (.jpg, .jpeg)
- **PNG** (.png)
- **URL downloads** (automatic detection)
- **Base64/Binary data** (MediaData::Bytes)

## Examples

### Square Video Generation
```rust
GenerationConfig {
    aspect_ratio: Some(AspectRatio::Square),
    // Input: Any sized image
    // Output: Processed to 768x768
}
```

### Portrait Video Generation  
```rust
GenerationConfig {
    aspect_ratio: Some(AspectRatio::Portrait),
    // Input: Any sized image  
    // Output: Processed to 576x1024
}
```

### Landscape Video Generation (Default)
```rust
GenerationConfig {
    aspect_ratio: None, // or AspectRatio::Landscape
    // Input: Any sized image
    // Output: Processed to 1024x576
}
```

## Processing Examples

### Example 1: Wide Image → Square
```
Input:  1920x1080 (16:9)
Target: 768x768   (1:1)
Crop:   1080x1080 (center crop)
Final:  768x768   (resize)
```

### Example 2: Tall Image → Landscape  
```
Input:  1080x1920 (9:16)
Target: 1024x576  (16:9)
Crop:   1080x608  (center crop)
Final:  1024x576  (resize)
```

### Example 3: Perfect Match
```
Input:  1024x576  (16:9)
Target: 1024x576  (16:9)
Crop:   1024x576  (no crop needed)
Final:  1024x576  (no resize needed)
```

## Technical Details

### Memory Usage
- Temporarily loads full image into memory
- Processing happens in-memory for speed
- Automatic cleanup after processing

### Quality Settings
- **Filter**: Lanczos3 (highest quality)
- **Format**: PNG output (lossless)
- **Bit Depth**: Preserves original where possible

### Error Handling
```rust
// Invalid image data
invalid_input("Failed to decode image: ...")

// Processing failure  
internal_error("Failed to encode processed image: ...")
```

### Logging
Enable debug logging to see processing details:
```bash
export GOLEM_VIDEO_LOG=debug
```

Output includes:
- Original dimensions
- Target dimensions  
- Crop coordinates
- Final processed size

## Performance Considerations

- **Small images** (< 1MP): Near-instantaneous
- **Medium images** (1-4MP): < 100ms typical
- **Large images** (> 10MP): 200-500ms typical
- **URL downloads**: Additional network latency

## Limitations

- **Center crop only**: Cannot preserve specific subjects
- **Memory intensive**: Large images use more RAM
- **PNG output**: May be larger than input JPEG
- **Synchronous**: Blocks during processing

## Future Enhancements

Potential improvements:
- Smart crop (face/object detection)
- Batch processing optimization
- Background processing
- Format preservation options
