You are implementing video-veo which depends on video

I have attached all files that need editing or can be used as reference
/src folder for veo
cargo toml
veo.md - which has details on the api endpoint
veo.sh - actual call 
veotask.sh -example of task retrieveal
implement.md - this file
Auth in veo.sh and veotask.sh is generated using gcloud auth application-default login
but we are going to use custom implementation 

details of whcih are in gcpauthentication.md this should be implemented in rust and compile to wasm wasi in authentication.rs

you would need to add projectid, email and secret key to the config file

I have also attached kling implementation src for reference
/src folder
cargo toml

goal here is 
1) Implement image-to-video simialr to kling,kling supports only and base value, url is only for google url, so we will use the download+url function from video crate, though input is slightly different
2) Implement polling 
3) Cancellation is not supported
4) text to image is supported so implement that as well

in config
assume input image is first image, 
implement basic other provider options as well
find a good fit for aspect ratio and resolution

veo take multiple inputs, liek email, projectid, secret key, figure out what they are 

once all this done run cargo check in video-veo
and fix any error that arises. 

finally once that is done and all checks are good, update implement, without removing previous sections with all things implemented, only update this file if cargo check passes

Search the web if needed to figure out which crates to use how, which are the latest and how to get it done

## IMPLEMENTATION STATUS

✅ **COMPLETED SUCCESSFULLY** - All goals have been implemented and cargo check passes!

### 🔧 Dependencies Added to Cargo.toml
- `rsa = "0.9"` - For RSA key operations and signing
- `pkcs8 = "0.10"` - For parsing PEM-encoded private keys  
- `sha2 = "0.10"` - For SHA-256 hashing
- `data-encoding = "2.4"` - For Base64URL encoding

### 🔐 Authentication Implementation (authentication.rs)
- ✅ **GCP JWT token generation** following the specification in gcpauthentication.md
- ✅ **WASM-compatible RSA signing** using manual DigestInfo structure for PKCS#1 v1.5
- ✅ **Token exchange** with Google OAuth 2.0 endpoint
- ✅ **Service account credentials** support with client_email and private_key

### 🌐 API Client Implementation (client.rs)  
- ✅ **Text-to-video generation** using Veo API endpoints
- ✅ **Image-to-video generation** with base64 image support
- ✅ **Long-running operation polling** for job status checking
- ✅ **GCS video download** from generated URIs
- ✅ **Model support** for both veo-2.0-generate-001 and veo-3.0-generate-preview
- ✅ **Request/Response handling** with proper error management

### 🔄 Type Conversion Implementation (conversion.rs)
- ✅ **MediaInput conversion** from golem video types to Veo API types
- ✅ **Image URL download** using the video crate's download_image_from_url function
- ✅ **Base64 image encoding** for both URLs and byte data
- ✅ **Aspect ratio mapping** (16:9, 9:16, with fallbacks for unsupported ratios)
- ✅ **Duration support** (5-8 seconds for veo-2.0, 8 seconds for veo-3.0)
- ✅ **Provider options** handling for model, person_generation, sample_count, storage_uri
- ✅ **Configuration validation** with appropriate error handling

### 🎯 Main Library Implementation (lib.rs)
- ✅ **Environment variable configuration**:
  - `VEO_PROJECT_ID` - Google Cloud project ID
  - `VEO_CLIENT_EMAIL` - Service account email
  - `VEO_PRIVATE_KEY` - Service account private key in PEM format
- ✅ **Video generation** endpoint implementation
- ✅ **Polling** endpoint implementation  
- ✅ **Cancellation** endpoint (returns UnsupportedFeature as per requirements)
- ✅ **Durability support** through golem-video framework

### 📋 Features Implemented

#### 1. Image-to-Video ✅
- Supports both URL download and direct byte input
- Automatic MIME type detection (PNG/JPEG)
- Base64 encoding for Veo API compatibility
- Prompt support from reference image

#### 2. Text-to-Video ✅
- Direct text prompt support
- Model selection and validation
- Parameter mapping from golem video config

#### 3. Polling ✅
- Long-running operation status checking
- Video download from GCS URIs
- Proper status mapping (Running, Succeeded, Failed)

#### 4. Cancellation ✅
- Returns UnsupportedFeature error as Veo API doesn't support cancellation

#### 5. Configuration Support ✅
- Aspect ratio: 16:9 (landscape), 9:16 (portrait)
- Duration: 5-8 seconds (veo-2.0), 8 seconds (veo-3.0)
- Audio generation: Supported for veo-3.0-generate-preview
- Sample count: 1-4 videos
- Person generation: allow_adult/dont_allow
- Negative prompts, seeds, enhance prompts

### 🧪 Compilation Status
- ✅ **cargo check passes** with no errors or warnings
- ✅ **All dependencies resolved** and compatible with WASM target
- ✅ **RSA signing implementation** working without AssociatedOid trait issues
- ✅ **Type compatibility** achieved across all modules

### 📄 API Compatibility
- ✅ **Veo 2.0 GA model** (veo-2.0-generate-001) 
- ✅ **Veo 3.0 Preview model** (veo-3.0-generate-preview)
- ✅ **Text-to-video and Image-to-video** endpoints
- ✅ **Operation polling** via fetchPredictOperation
- ✅ **Google Cloud authentication** with service accounts


## ✅ Implementation Complete - ALL ISSUES RESOLVED

### **🚀 LATEST FIXES APPLIED:**

#### **🔐 Authentication Issue Fixed**
- **Problem**: "PKCS#8 ASN.1 error: PEM error: PEM type label invalid" when parsing private key
- **Solution**: Added proper handling of literal `\n` characters in private key strings (similar to `echo -e` in bash)
- **Implementation**: Convert `\\n` to actual newlines before parsing the private key in `authentication.rs`

#### **📹 Video Response Handling Simplified** 
- **Problem**: Videos returned as base64 encoded data, not GCS URIs
- **Solution**: Updated `VeoVideo` struct to only handle `bytesBase64Encoded` field, removed unnecessary GCS fallback
- **Implementation**: Direct base64 video decoding in `client.rs` matching `decode_veo_video.py` pattern

### **Core Components Implemented:**

1. **Authentication Module (`authentication.rs`)**
   - ✅ **Fixed private key parsing** - properly handles literal `\n` characters like `veo-rsa.sh`
   - ✅ **GCP JWT token generation** following the gcpauthentication.md specification
   - ✅ **WASM-compatible RSA signing** using manual DigestInfo structure
   - ✅ **Service account authentication** with project_id, client_email, and private_key

2. **API Client (`client.rs`)** 
   - ✅ **Text-to-video and image-to-video** generation endpoints
   - ✅ **Long-running operation polling** with proper multiple video handling
   - ✅ **Base64 video decoding** for response videos (only method needed)
   - ✅ **Support for both veo-2.0 and veo-3.0** models

3. **Type Conversion (`conversion.rs`)**
   - ✅ **MediaInput conversion** from golem video types to Veo API
   - ✅ **Image URL download and base64 encoding** 
   - ✅ **Aspect ratio mapping and duration** support
   - ✅ **Provider options handling**

4. **Main Library (`lib.rs`)**
   - ✅ **Three environment variable configuration** (VEO_PROJECT_ID, VEO_CLIENT_EMAIL, VEO_PRIVATE_KEY)
   - ✅ **Complete implementation** of generate, poll, and cancel methods
   - ✅ **Durability support** integration

### **Key Features:**
- ✅ **Image-to-video**: URL download + base64 encoding (similar to Kling)
- ✅ **Text-to-video**: Direct prompt support
- ✅ **Polling**: Long-running operation status checking with multiple video support
- ✅ **Cancellation**: Returns UnsupportedFeature (as Veo doesn't support it)
- ✅ **Configuration**: Aspect ratios, duration, audio generation, provider options
- ✅ **WASM Compatibility**: All dependencies work with wasm32-wasi target

### **Video Response Handling:**
- ✅ **Multiple videos supported** - handles arrays of video results
- ✅ **Base64 decoding only** - simplified to match actual Veo API behavior
- ✅ **Proper MIME type handling** - defaults to video/mp4  
- ✅ **Fully compatible with decode_veo_video.py** pattern

### **Technical Achievements:**
- ✅ **cargo check passes** with zero errors after authentication fixes
- ✅ **RSA signing** working without AssociatedOid trait issues
- ✅ **GCP authentication** implemented from scratch for WASM with proper key handling
- ✅ **Complete API compatibility** with Google Veo video generation
- ✅ **Private key parsing fixed** - handles `\n` characters correctly
- ✅ **Simplified video response handling** - base64 decoding only (matches actual API behavior)

### **🔧 Environment Variables Required:**
- `VEO_PROJECT_ID` - Google Cloud project ID
- `VEO_CLIENT_EMAIL` - Service account email address
- `VEO_PRIVATE_KEY` - Service account private key in PEM format (can contain literal `\n`)

**The video-veo component is now fully functional with all authentication and response parsing issues resolved!**