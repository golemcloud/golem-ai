All advanced tests are for kling only.

whenever a file or link is needed, use generic file name or example.com/file as url, I will add actual file or link later.

Test 1
generate(image)
This is a image to video test. A simple test which should be able to generate a video from a image, the image should specific first in role, and should also include a lastimage as kling supports it and the prompt should be a simple text. again this output will be saved with test1 appended to the file name. both the image should be a data, inline raw bytes image format. Each output file should be saved as a separate file, with test1 appended to the file name.

Test 2
generate(image)
This is a image to video test. A test which should be able to generate a video from a image, the image should specific none in role, and the prompt should be a simple text. It should also include camera conifg enum, you can find details in wit file. Each output file should be saved as a separate file, with test2 appended to the file name.

Test 3
generate(image)
This is a image to video test. A test which should be able to generate a video from a image, the image should specific none in role, and the prompt should be a simple text. it should include static and dynamic mask, with link to url as image input, and position values, you can find details in wit file. Each output file should be saved as a separate file, with test3 appended to the file name. remember to save the job-id so it can be used in test 9.

Test 4
list voice-id
This is a simple test which test the list voice-id function, output the voice-id and the name and langugae as text.

Test 5
lip-sync(voice-id)
This is a test for lipsync it will take a input video and a voice-id, and it will generate a video with the lipsync. it should output the video file with test5 appended to the file name. Use inline raw bytes video format for input video.

Test 6
lip-sync(audio-file)
This is a test for lipsync it will take a input video and audio-file, and it will generate a video with the lipsync. it should output the video file with test6 appended to the file name.Use in line raw bytes audio format for audio-file.

Test 7
Effects(single)
This is a test for effects, it will take one single input image, and a enum effect, and it will generate a video with the effects. it should output the video file with test7 appended to the file name. The image will be a data, inline raw bytes image format.

Test 8
Effects(double)
This is a test for effects, it will take two input images, and a enum effect, and it will generate a video with the effects. it should output the video file with test8 appended to the file name. The images will be urls.

Test 9 
extend(video)
extend video takes a job-id, this in this case will be job-id from test 3, remember to use the job-id from test 3. and pass it with paramters to extend the video.

Test 10
multiplegeneration(list(image))
This is a test for multi-image generation, it will take a list of images, and a prompt, and it will generate a video with the images. it should output the video file with test10 appended to the file name. The images will be urls. 3 images in the list, make 2 url and 1 inline raw bytes image format.
