# golem-video

WebAssembly Components providing a unified API for various video generation providers.

## Versions

Each video provider has two versions: **Default** (with Golem-specific durability features) and **Portable** (no Golem dependencies).

There are 8 published WASM files for each release:

| Name                                 | Description                                                                                |
|--------------------------------------|--------------------------------------------------------------------------------------------|
| `golem-video-veo.wasm`              | Video implementation for Google Veo, using custom Golem specific durability features |
| `golem-video-stability.wasm`        | Video implementation for Stability AI, using custom Golem specific durability features |
| `golem-video-kling.wasm`            | Video implementation for Kling, using custom Golem specific durability features |
| `golem-video-runway.wasm`           | Video implementation for Runway ML, using custom Golem specific durability features |
| `golem-video-veo-portable.wasm`     | Video implementation for Google Veo, with no Golem specific dependencies |
| `golem-video-stability-portable.wasm` | Video implementation for Stability AI, with no Golem specific dependencies |
| `golem-video-kling-portable.wasm`   | Video implementation for Kling, with no Golem specific dependencies |
| `golem-video-runway-portable.wasm`  | Video implementation for Runway ML, with no Golem specific dependencies |

Every component **exports** the same `golem:video` interface, [defined here](wit/golem-video.wit).

## Usage

For general usage information, integration examples, and getting started guides, see the [main README](../README.md).

### Environment Variables

Each provider has to be configured with an ENVIRONMENT variables passed as an environment variable:

| Provider  | Environment Variable |
|-----------|---------------------|
| Veo       | `VEO_PROJECT_ID`, `VEO_CLIENT_EMAIL`, `VEO_PRIVATE_KEY` |
| Stability | `STABILITY_API_KEY` |
| Kling     | `KLING_ACCESS_KEY`, `KLING_SECRET_KEY` |
| Runway    | `RUNWAY_API_KEY`    |

Additionally, setting the `GOLEM_VIDEO_LOG=trace` environment variable enables trace logging for all the communication
with the underlying video provider.

## Examples

Golem-video has two test applications:

- Basic video tests
- Advanced video tests (Kling only)

### Basic Video Tests

Take the [basic video test application](../test/video/components-rust/test-video/src/lib.rs) as an example of using `golem-video` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Simple text-to-video generation with a simple prompt                                       | 
| `test2`       | Image-to-video generation with inline image with polling durability testing                |
| `test3`       | Image-to-video generation with 'last' role and URL image                                   |
| `test4`       | Video-to-video generation (VEO only)                                                       |
| `test5`       | Video upscale (Runway only)                                                                |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary started with `golem server run`.

Then build and deploy the _test application_. The following profiles are available for testing:

| Profile Name         | Description                                                                           |
|----------------------|---------------------------------------------------------------------------------------|
| `veo-debug`          | Uses the Google Veo implementation and compiles the code in debug profile             |
| `veo-release`        | Uses the Google Veo implementation and compiles the code in release profile           |
| `stability-debug`    | Uses the Stability AI implementation and compiles the code in debug profile           |
| `stability-release`  | Uses the Stability AI implementation and compiles the code in release profile         |
| `kling-debug`        | Uses the Kling implementation and compiles the code in debug profile                  |
| `kling-release`      | Uses the Kling implementation and compiles the code in release profile                |
| `runway-debug`       | Uses the Runway ML implementation and compiles the code in debug profile              |
| `runway-release`     | Uses the Runway ML implementation and compiles the code in release profile            |

```bash
cd ../test/video
golem app build -b veo-debug
golem app deploy -b veo-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the ENVIRONMENT variable (eg.API key) for the given provider:

```bash
golem worker new test:video/debug --env VEO_PROJECT_ID=xxx --env VEO_CLIENT_EMAIL=xxx --env VEO_PRIVATE_KEY=xxx --env GOLEM_VIDEO_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:video/debug test1 --stream 
```

### Advanced Video Tests

For advanced video generation features, there's a separate test application with additional capabilities.

Take the [advanced video test application](../test/video/components-rust/test-video-advanced/src/lib.rs) as an example of using `golem-video` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Image-to-video with first and last frame, inline with polling durability testing           |
| `test2`       | Image-to-video with advanced camera control enum                                           |
| `test3`       | Image-to-video with static and dynamic mask                                                |
| `test4`       | List voice IDs and their information                                                       |
| `test5`       | Lip-sync video generation using voice-id                                                   |
| `test6`       | Lip-sync video generation using audio file                                                 |
| `test7`       | Video effects with single input image and effect "fuzzyfuzzy"                              |
| `test8`       | Video effects with two input images and effect "hug"                                       |
| `test9`       | Extend video using generation-id from a completed text-to-video generation                 |
| `testx`       | Multi-image generation (2 URLs + 1 inline raw bytes)                                       |
| `testy`       | Text to video, extend the video, and then lip sync workflow                                |


### Running the advanced examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary started with `golem server run`.

Then build and deploy the _test application_. The advanced examples are only supported by the Kling provider. 
The following profiles are available for testing:

| Profile Name         | Description                                                                           |
|----------------------|---------------------------------------------------------------------------------------|
| `kling-debug`        | Uses the Kling implementation and compiles the code in debug profile                  |
| `kling-release`      | Uses the Kling implementation and compiles the code in release profile                |

```bash
cd ../test/video-advanced
golem app build -b kling-debug
golem app deploy -b kling-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the ENVIRONMENT variable (eg.API key) for the given provider:

```bash
golem worker new test:video-advanced/debug --env KLING_ACCESS_KEY=xxx --env KLING_SECRET_KEY=xxx --env GOLEM_VIDEO_LOG=trace
```

```bash
golem worker invoke test:video-advanced/debug test1 --stream 
```

