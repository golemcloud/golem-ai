Veo on Vertex AI API

Veo is the name of the model that supports video generation. Veo generates a video from a text prompt or an image prompt that you provide.

To explore this model in the console, see the Video Generation model card in the Model Garden.

Try Veo on Vertex AI (Vertex AI Studio)

Try Veo in a Colab

Request access: Advanced features & Veo waitlist
Supported Models

Veo API supports the following models:

    veo-2.0-generate-001
    veo-3.0-generate-preview (Preview)

HTTP request

curl -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
https://LOCATION

-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/LOCATION

/publishers/google/models/MODEL_ID

:predictLongRunning \

-d '{
  "instances": [
    {
      "prompt": string,
      "image": {
        // Union field can be only one of the following:
        "bytesBase64Encoded": string,
        "gcsUri": string,
        // End of list of possible types for union field.
        "mimeType": string
      },
      "lastFrame": {
        // Union field can be only one of the following:
        "bytesBase64Encoded": string,
        "gcsUri": string,
        // End of list of possible types for union field.
        "mimeType": string
      },
      "video": {
        // Union field can be only one of the following:
        "bytesBase64Encoded": string,
        "gcsUri": string,
        // End of list of possible types for union field.
        "mimeType": string
      }
    }
  ],
  "parameters": {
    "aspectRatio": string,
    "durationSeconds": integer,
    "enhancePrompt": boolean,
    "generateAudio": boolean,
    "negativePrompt": string,
    "personGeneration": string,
    "sampleCount": integer,
    "seed": uint32,
    "storageUri": string
  }
}'

Instances

prompt
	

string

Required for text-to-video.
Optional if an input image prompt is provided (image-to-video).

A text string to guide the first eight seconds in the video. For example:

    A fast-tracking shot through a bustling dystopian sprawl with bright neon signs, flying cars and mist, night, lens flare, volumetric lighting
    A neon hologram of a car driving at top speed, speed of light, cinematic, incredible details, volumetric lighting
    Many spotted jellyfish pulsating under water. Their bodies are transparent and glowing in deep ocean
    extreme close-up with a shallow depth of field of a puddle in a street. reflecting a busy futuristic Tokyo city with bright neon signs, night, lens flare
    Timelapse of the northern lights dancing across the Arctic sky, stars twinkling, snow-covered landscape
    A lone cowboy rides his horse across an open plain at beautiful sunset, soft light, warm colors

Union field image.

Optional. An image to guide video generation, which can be either a bytesBase64Encoded string that encodes an image or a gcsUri string URI to a Cloud Storage bucket location.

Union field lastFrame.

Optional. An image of the first frame of a video to fill the space between. lastFrame can be either a bytesBase64Encoded string that encodes an image or a gcsUri string URI to a Cloud Storage bucket location.

lastFrame is supported by veo-2.0-generate-001 only.

Union field video.

Optional. A Veo generated video to extend in length, which can be either a bytesBase64Encoded string that encodes a video or a gcsUri string URI to a Cloud Storage bucket location.

video is supported by veo-2.0-generate-001 only.
bytesBase64Encoded 	

string

A bytes Base64-encoded string of an image or video file.
gcsUri 	

string

A string URI to a Cloud Storage bucket location.
mimeType 	

string

Required for the following objects:

    image
    video
    lastFrame

Specifies the mime type of a video or image.

For images, the following mime types are accepted:

    image/jpeg
    image/png

For videos, the following mime types are accepted:

    video/mp4

Parameters
aspectRatio 	

string

Optional. Specifies the aspect ratio of generated videos. The following are accepted values:

    16:9 (default value)
    9:16

durationSeconds 	

integer

Required. The length of video files that you want to generate.

The following are the accepted values for each model:

    veo-2.0-generate-001: 5-8. The default is 8.
    veo-3.0-generate-preview: 8.

enhancePrompt 	

boolean

Optional. Use Gemini to enhance your prompts. Accepted values are true or false. The default value is true.
generateAudio 	

boolean

Required for veo-3.0-generate-preview. Generate audio for the video. Accepted values are true or false.

generateAudio isn't supported by veo-2.0-generate-001.
negativePrompt 	

string

Optional. A text string that describes anything you want to discourage the model from generating. For example:

    overhead lighting, bright colors
    people, animals
    multiple cars, wind

personGeneration 	

string

Optional. The safety setting that controls whether people or face generation is allowed. One of the following:

    allow_adult (default value): allow generation of adults only
    dont_allow: disallows inclusion of people/faces in images

sampleCount 	

int

Optional. The number of output videos requested. Accepted values are 1-4.
seed 	

uint32

Optional. A number to request to make generated videos deterministic. Adding a seed number with your request without changing other parameters will cause the model to produce the same videos.

The accepted range is 0-4,294,967,295.
storageUri 	

string

Optional. A Cloud Storage bucket URI to store the output video, in the format gs://BUCKET_NAME/SUBDIRECTORY. If a Cloud Storage bucket isn't provided, base64-encoded video bytes are returned in the response.
Sample request

Use the following requests to send a text-to-video request or an image-to-video request:
Text-to-video generation request
REST

To test a text prompt by using the Vertex AI Veo API, send a POST request to the publisher model endpoint.

Before using any of the request data, make the following replacements:

    PROJECT_ID: Your Google Cloud project ID.
    MODEL_ID: The model ID to use. Available values:
        veo-2.0-generate-001 (GA)
        veo-3.0-generate-preview (Preview)
    TEXT_PROMPT: The text prompt used to guide video generation.
    OUTPUT_STORAGE_URI: Optional: The Cloud Storage bucket to store the output videos. If not provided, video bytes are returned in the response. For example: gs://video-bucket/output/.
    RESPONSE_COUNT: The number of video files you want to generate. Accepted integer values: 1-4.
    DURATION: The length of video files that you want to generate. Accepted integer values are 5-8.

    Additional optional parameters

HTTP method and URL:

POST https://us-central1-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

:predictLongRunning

Request JSON body:

{
  "instances": [
    {
      "prompt": "TEXT_PROMPT

"
    }
  ],
  "parameters": {
    "storageUri": "OUTPUT_STORAGE_URI

",
    "sampleCount": "RESPONSE_COUNT

"
  }
}

To send your request, choose one of these options:
curl
PowerShell
Note: The following command assumes that you have logged in to the gcloud CLI with your user account by running gcloud init or gcloud auth login , or by using Cloud Shell, which automatically logs you into the gcloud CLI . You can check the currently active account by running gcloud auth list.

Save the request body in a file named request.json, and execute the following command:

curl -X POST \
     -H "Authorization: Bearer $(gcloud auth print-access-token)" \
     -H "Content-Type: application/json; charset=utf-8" \
     -d @request.json \
     "https://us-central1-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

:predictLongRunning"

This request returns a full operation name with a unique operation ID. Use this full operation name to poll that status of the video generation request.

{
  "name": "projects/PROJECT_ID/locations/us-central1/publishers/google/models/MODEL_ID/operations/a1b07c8e-7b5a-4aba-bb34-3e1ccb8afcc8"
}

Image-to-video generation request
REST

To test a text prompt by using the Vertex AI Veo API, send a POST request to the publisher model endpoint.

Before using any of the request data, make the following replacements:

    PROJECT_ID: Your Google Cloud project ID.
    MODEL_ID: The model ID to use. Available values:
        veo-2.0-generate-001 (GA)
        veo-3.0-generate-preview (Preview)
    TEXT_PROMPT: The text prompt used to guide video generation.
    INPUT_IMAGE: Base64-encoded bytes string representing the input image. To ensure quality, the input image should be 720p or higher (1280 x 720 pixels) and have a 16:9 or 9:16 aspect ratio. Images of other aspect ratios or sizes may be resized or centrally cropped during the upload process.
    MIME_TYPE: The MIME type of the input image. Only the images of the following MIME types are supported: image/jpeg or image/png.
    OUTPUT_STORAGE_URI: Optional: The Cloud Storage bucket to store the output videos. If not provided, video bytes are returned in the response. For example: gs://video-bucket/output/.
    RESPONSE_COUNT: The number of video files you want to generate. Accepted integer values: 1-4.
    DURATION: The length of video files that you want to generate. Accepted integer values are 5-8.

    Additional optional parameters

HTTP method and URL:

POST https://us-central1-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

:predictLongRunning

Request JSON body:

{
  "instances": [
    {
      "prompt": "TEXT_PROMPT

",
      "image": {
        "bytesBase64Encoded": "INPUT_IMAGE

",
        "mimeType": "MIME_TYPE

"
      }
    }
  ],
  "parameters": {
    "storageUri": "OUTPUT_STORAGE_URI

",
    "sampleCount": RESPONSE_COUNT


  }
}

To send your request, choose one of these options:
curl
PowerShell
Note: The following command assumes that you have logged in to the gcloud CLI with your user account by running gcloud init or gcloud auth login , or by using Cloud Shell, which automatically logs you into the gcloud CLI . You can check the currently active account by running gcloud auth list.

Save the request body in a file named request.json, and execute the following command:

curl -X POST \
     -H "Authorization: Bearer $(gcloud auth print-access-token)" \
     -H "Content-Type: application/json; charset=utf-8" \
     -d @request.json \
     "https://us-central1-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

:predictLongRunning"

This request returns a full operation name with a unique operation ID. Use this full operation name to poll that status of the video generation request.

{
  "name": "projects/PROJECT_ID/locations/us-central1/publishers/google/models/MODEL_ID/operations/a1b07c8e-7b5a-4aba-bb34-3e1ccb8afcc8"
}

Poll the status of the video generation long-running operation

Check the status of the video generation long-running operation.
REST

Before using any of the request data, make the following replacements:

    PROJECT_ID: Your Google Cloud project ID.
    MODEL_ID: The model ID to use. Available values:
        veo-2.0-generate-001 (GA)
        veo-3.0-generate-preview (Preview)
    OPERATION_ID: The unique operation ID returned in the original generate video request.

HTTP method and URL:

POST https://us-central1-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

:fetchPredictOperation

Request JSON body:

{
  "operationName": "projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

/operations/OPERATION_ID

"
}

To send your request, choose one of these options:
curl
PowerShell
Note: The following command assumes that you have logged in to the gcloud CLI with your user account by running gcloud init or gcloud auth login , or by using Cloud Shell, which automatically logs you into the gcloud CLI . You can check the currently active account by running gcloud auth list.

Save the request body in a file named request.json, and execute the following command:

curl -X POST \
     -H "Authorization: Bearer $(gcloud auth print-access-token)" \
     -H "Content-Type: application/json; charset=utf-8" \
     -d @request.json \
     "https://us-central1-aiplatform.googleapis.com/v1/projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

:fetchPredictOperation"

This request returns information about the operation, including if the operation is still running or is done.
Response

Response body (generate video request)

Sending a text-to-video or image-to-video request returns the following response:

{
  "name": string
}

Response element 	Description
name 	The full operation name of the long-running operation that begins after a video generation request is sent.
Sample response (generate video request)

{
  "name": "projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

/operations/OPERATION_ID

"
}

Response body (poll long-running operation)

Polling the status of the original video generation long-running operation returns the following response:

{
   "name": string,
   "done": boolean,
   "response":{
      "@type":"type.googleapis.com/cloud.ai.large_models.vision.GenerateVideoResponse",
      "videos":[
         {
           "gcsUri": string,
           "mimeType": string
         },
         {
           "gcsUri": string,
           "mimeType": string
         },
         {
           "gcsUri": string,
           "mimeType": string
         },
         {
           "gcsUri": string,
           "mimeType": string
         },
      ]
   }
}

Response element 	Description
name 	The full operation name of the long-running operation that begins after a video generation request is sent.
done 	A boolean value that indicates whether the operation is complete.
response 	The response body of the long-running operation.
generatedSamples 	An array of the generated video sample objects.
video 	The generated video.
uri 	The Cloud Storage URI of the generated video.
encoding 	The video encoding type.
Sample response (poll long-running operation)

{
   "name": "projects/PROJECT_ID

/locations/us-central1/publishers/google/models/MODEL_ID

/operations/OPERATION_ID

",
   "done":true,
   "response":{
      "@type":"type.googleapis.com/cloud.ai.large_models.vision.GenerateVideoResponse",
      "videos":[
        {
          "gcsUri":"gs://STORAGE_BUCKET

/TIMESTAMPED_SUBDIRECTORY

/sample_0.mp4",
          "mimeType":"video/mp4"
        },
        {
          "gcsUri":"gs://STORAGE_BUCKET

/TIMESTAMPED_SUBDIRECTORY

/sample_1.mp4",
          "mimeType":"video/mp4"
        },
        {
          "gcsUri":"gs://STORAGE_BUCKET

/TIMESTAMPED_SUBDIRECTORY

/sample_2.mp4",
          "mimeType":"video/mp4"
        },
        {
          "gcsUri":"gs://STORAGE_BUCKET

/TIMESTAMPED_SUBDIRECTORY

/sample_3.mp4",
          "mimeType":"video/mp4"
        }
      ]
   }
}