Test video

whenever a file or link is needed, use generic file name or example.com/file as url, I will add actual file or link later.

Test 1
generate(text)
Is a text to video test. A simple test which should be able to generate a video from a text prompt. and should save the video file, with the test as part of the file name. job_id is passed as a parameter, but not used, replace that it with test1, for each test it will be appended with test number.

Test 2
generate(image)
It is a image to video test. A simple test which should be able to generate a video from a image, the image should specific none in role, and the prompt should be a simple text. again this output will be saved with test2 appended to the file name. the image should be a data, inline raw bytes image format.

you will have to do a durability test as part of this, in the polling, lib.rs file from llm has a test with such features, you can use that as a reference. That is for streaming with failure, here you have to do polling with failure, let it poll and then have the worker fail, and it will auto pass.

Test 3
generate(image)
It is a image to video test. A simple test which should be able to generate a video from a image, the image should specific last in role, and the prompt should be a simple text. again this output will be saved with test3 appended to the file name. the image should be a url

Test 4
//veo only
generate(video)
This is a video to video test. A simple test which should be able to generate a video from a video, this will use data, inline raw bytes video format. and it will use the output of test 1 as input. it should ouput file with test4 appended to the file name.

Test 5
//runway only
upscale(video)
This is a video to video test, for upscaling a video. it will use the output of test 1 as input. it should ouput file with test5 appended to the file name.