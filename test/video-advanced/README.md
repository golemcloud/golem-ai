Take the [test application](test/video-advanced/components-rust/test-video-advanced/src/lib.rs) as an example of using `golem-video` from Rust. The
implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Image to video generation with first frame and last frame included (both inline images)    | 
| `test2`       | Image to video generation with advanced camera control enum                                |
| `test3`       | Image to video generation with static and dynamic mask (URL input)                         |
| `test4`       | List voice IDs and their information                                                       |
| `test5`       | Lip-sync video generation using voice-id                                                   |
| `test6`       | Lip-sync video generation using audio file (inline raw bytes audio input)                  |
| `test7`       | Video effects with single input image (inline raw bytes) and single image effect           |
| `test8`       | Video effects with two input images (URLs) and dual image effect "hug"                     |
| `test9`       | Extend video using generation-id from completed text-to-video                              |
| `testx`       | Multi-image generation (2 URLs + 1 inline raw bytes)                                       |
| `testy`       | Text to video, then extend video, and then lip sync with voice-id  (using generation-id)   |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary
started with `golem server run`.

Then build and deploy the _test application_. Select one of the following profiles to choose which provider to use:
| Profile Name | Description |
|--------------|-----------------------------------------------------------------------------------------------|
| `kling-debug` | Uses the Kling video implementation and compiles the code in debug profile |
| `kling-release` | Uses the Kling video implementation and compiles the code in release profile |

```bash
cd test
golem app build -b kling-debug
golem app deploy -b kling-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the API key for the given provider:

```bash
golem worker new test:video-advanced/debug --env KLING_ACCESS_KEY=xxx --env KLING_SECRET_KEY=xxx --env GOLEM_VIDEO_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:video-advanced/debug test1 --stream 
```