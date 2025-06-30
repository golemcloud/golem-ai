# Implementation Plan: WIT Updates for Image Roles and LastFrame Support

## Overview
Update all video providers to support the new WIT structure with:
- `image-role` enum (first/last) in the `reference` record  
- `lastframe: option<input-image>` in `generation-config`
- Updated `reference` structure with `data: input-image`

## Provider Capabilities Analysis

### ✅ Kling
- **Supports**: Image-to-video via `/v1/videos/image2video` endpoint
- **Frame positioning**: Based on implement.md, supports both first and last frames
- **LastFrame**: Should support if role=last or lastframe config is provided
- **Special handling**: If both role=last AND lastframe provided → warn and use lastframe only

### ✅ Runway  
- **Supports**: Image-to-video with explicit position field ("first"/"last")
- **Frame positioning**: Native support via `position` in promptImage array
- **LastFrame**: Can use lastframe for last frame positioning

### ✅ Veo
- **Supports**: Image-to-video + lastFrame for veo-2.0-generate-001 only
- **Frame positioning**: Supports both first frame (image field) and last frame (lastFrame field)
- **LastFrame**: Direct mapping to API's lastFrame field

### ❌ Stability  
- **Supports**: Basic image-to-video only
- **Frame positioning**: Not supported - ignore role and lastframe with warnings
- **LastFrame**: Not supported

## Step-by-Step Implementation Plan

### Phase 1: Core Infrastructure Updates
1. **Update video/video WIT dependencies**
   - Copy updated golem-video.wit to all provider wit/deps folders
   - Ensure all providers have the new WIT structure

2. **Update video/video module bindings** 
   - Regenerate bindings to include new types (InputImage, ImageRole)
   - Update exports in video crate

### Phase 2: Kling Implementation  
3. **Update Kling client.rs**
   - Add support for lastframe parameter in ImageToVideoRequest
   - Research if Kling API supports last frame positioning

4. **Update Kling conversion.rs** 
   - Handle `reference.role` (first/last) 
   - Handle `config.lastframe`
   - Implement warning logic: if role=last AND lastframe present → warn, use lastframe
   - Update MediaInput::Image handling for new input-image structure

### Phase 3: Runway Implementation
5. **Update Runway client.rs**
   - Modify promptImage to support position field properly
   - Handle multiple images (first frame + last frame)

6. **Update Runway conversion.rs**
   - Map `reference.role` to Runway's position field  
   - Handle `config.lastframe` for last frame positioning
   - Support both single image with position and first+last image combinations

### Phase 4: Veo Implementation  
7. **Update Veo client.rs**
   - Add lastFrame field support to ImageToVideoRequest
   - Ensure veo-2.0-generate-001 model validation for lastFrame usage

8. **Update Veo conversion.rs**
   - Map `reference.role` to appropriate API fields
   - Handle `config.lastframe` → map to API's lastFrame field
   - Add model-specific validation (lastFrame only for veo-2.0)

### Phase 5: Stability Implementation
9. **Update Stability conversion.rs**
   - Add warning logs for unsupported `reference.role` 
   - Add warning logs for unsupported `config.lastframe`
   - Ensure backward compatibility (ignore new fields gracefully)

### Phase 6: Build and Validation
10. **Copy WIT files**
    - Run `cd video && cargo make wit` to copy updated WIT to all providers

11. **Test core video module**
    - `cd video && cargo check` must pass
    - Verify new types are properly exported

12. **Test all providers individually**
    - `cd video/kling && cargo check` 
    - `cd video/runway && cargo check`
    - `cd video/veo && cargo check` 
    - `cd video/stability && cargo check`

13. **Test component builds**
    - `cd video/kling && cargo component build`
    - `cd video/runway && cargo component build` 
    - `cd video/veo && cargo component build`
    - `cd video/stability && cargo component build`

## Implementation Details

### Input-Image Structure Changes
```rust
// OLD: reference.data was MediaData directly  
// NEW: reference.data is InputImage { data: MediaData }

// Update all MediaInput::Image handling:
match input {
    MediaInput::Image(reference) => {
        let media_data = reference.data.data; // Note: .data.data now
        let role = reference.role; // NEW
        let prompt = reference.prompt;
        // ...
    }
}
```

### LastFrame Handling Logic
```rust
// In config processing:
if let Some(lastframe) = config.lastframe {
    // Handle lastframe based on provider capabilities
    match provider {
        Kling => { /* Use as last frame if supported */ }
        Runway => { /* Add to promptImage array with position: "last" */ }  
        Veo => { /* Map to API lastFrame field */ }
        Stability => { /* Log warning and ignore */ }
    }
}
```

### Role + LastFrame Conflict Resolution (Kling only)
```rust
if reference.role == Some(ImageRole::Last) && config.lastframe.is_some() {
    log::warn!("Both image role=last and lastframe provided. Using lastframe only as specified.");
    // Use config.lastframe, ignore reference for last frame
    // Use reference as first frame instead
}
```

## Success Criteria
- ✅ All 4 providers compile with `cargo check`  
- ✅ All 4 providers build with `cargo component build`
- ✅ New WIT types properly supported across all providers
- ✅ Frame positioning works for Kling, Runway, Veo
- ✅ Stability gracefully ignores unsupported features with warnings
- ✅ Backward compatibility maintained for existing functionality
