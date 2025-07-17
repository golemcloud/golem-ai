Take the [test application](test/video/components-rust/test-video/src/lib.rs) as an example of using `golem-video` from Rust. The implemented test functions are demonstrating the following:

| Function Name | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `test1`       | Text to video generation                                                                   | 
| `test2`       | Iamge to video generation, using raw bytes, also durability polling test                   |
| `test3`       | Image to video generation, using a URL, with Image role last                               |
| `test4`       | Video to video generation, Available only in veo, uses google bucket for storage           |
| `test5`       | Video upscale, Available only in runway                                                    |

### Running the examples

To run the examples first you need a running Golem instance. This can be Golem Cloud or the single-executable `golem`
binary
started with `golem server run`.

Then build and deploy the _test application_. Select one of the following profiles to choose which provider to use:
| Profile Name | Description |
|--------------|-----------------------------------------------------------------------------------------------|
| `veo-debug` | Uses the VEO video implementation and compiles the code in debug profile |
| `veo-release` | Uses the VEO video implementation and compiles the code in release profile |
| `runway-debug` | Uses the Runway video implementation and compiles the code in debug profile |
| `runway-release` | Uses the Runway video implementation and compiles the code in release profile |
| `stability-debug` | Uses the Stability video implementation and compiles the code in debug profile |
| `stability-release` | Uses the Stability video implementation and compiles the code in release profile |
| `kling-debug` | Uses the Kling video implementation and compiles the code in debug profile |
| `kling-release` | Uses the Kling video implementation and compiles the code in release profile |

```bash
cd test
golem app build -b veo-debug
golem app deploy -b veo-debug
```

Depending on the provider selected, an environment variable has to be set for the worker to be started, containing the API key for the given provider:

```bash
golem worker new test:video/debug --env VEO_PROJECT_ID=xxx --env VEO_CLIENT_EMAIL=xxx --env VEO_PRIVATE_KEY=xxx --env GOLEM_VIDEO_LOG=trace
```

Then you can invoke the test functions on this worker:

```bash
golem worker invoke test:video/debug test1 --stream 
```