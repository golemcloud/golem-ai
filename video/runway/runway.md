Start generating

These endpoints all kick off tasks to create generations.
Generate a video from an image
POST
/v1/image_to_video

This endpoint will start a new task to generate a video from an image prompt.
Authentication

Authorization

    Use the HTTP Authorization header with the Bearer scheme along with an API key.

Headers

X-Runway-Version
Requiredstring

    This field must be set to the exact value 2024-11-06.

Request body

promptImage
Requiredstring or Array of PromptImages (objects)
    string <uri>

    A HTTPS URL or data URI containing an encoded image to be used as the first frame of the generated video. See our docs on image inputs for more information.
    Array of objects

    uri
    Requiredstring <uri>

        A HTTPS URL or data URI containing an encoded image. See our docs on image inputs for more information.
    position
    Requiredstring
        Accepted values:"first""last"

        The position of the image in the output video. "first" will use the image as the first frame of the video, "last" will use the image as the last frame of the video.

        "last" is currently supported for gen3a_turbo only.

model
Requiredstring
    Accepted values:"gen3a_turbo""gen4_turbo"

    The model variant to use.
ratio
Requiredstring
    Accepted values:"1280:720""720:1280""1104:832""832:1104""960:960""1584:672""1280:768""768:1280"

    The resolution of the output video.

    gen4_turbo supports the following values:

        1280:720
        720:1280
        1104:832
        832:1104
        960:960
        1584:672

    gen3a_turbo supports the following values:

        1280:768
        768:1280

seed
integer[ 0 .. 4294967295 ]

    If unspecified, a random number is chosen. Varying the seed integer is a way to get different results for the same other request parameters. Using the same seed integer for an identical request will produce similar results.
promptText
string<= 1000 characters

    A non-empty string up to 1000 characters (measured in UTF-16 code units). This should describe in detail what should appear in the output.
duration
integer
    Default:10
    Accepted values:510

    The number of seconds of duration for the output video.
contentModeration
object

    Settings that affect the behavior of the content moderation system.

    publicFigureThreshold
    string
        Default:"auto"
        Accepted values:"auto""low"

        When set to low, the content moderation system will be less strict about preventing generations that include recognizable public figures.

Responses
Response Schema: application/json

200 
id
Requiredstring <uuid>

    The ID of the newly created task. Use this ID to query the task status and retrieve the generated video.


Get task detail
GET
/v1/tasks/{id}

Return details about a task. Consumers of this API should not expect updates more frequent than once every five seconds for a given task.
Authentication

Authorization

    Use the HTTP Authorization header with the Bearer scheme along with an API key.

Path parameters

id
Requiredstring <uuid>

    The ID of a previously-submitted task that has not been canceled or deleted.

Headers

X-Runway-Version
Requiredstring

    This field must be set to the exact value 2024-11-06.

Responses

curl https://api.dev.runwayml.com/v1/tasks/{id} \
  -H "Authorization: Bearer {{ YOUR API KEY }}" \
  -H "X-Runway-Version: 2024-11-06"

    200

An example of a pending task
{

    "id": "17f20503-6c24-4c16-946b-35dbbce2af2f",
    "status": "PENDING",
    "createdAt": "2024-06-27T19:49:32.334Z"

}
Cancel or delete a task
DELETE
/v1/tasks/{id}

Tasks that are running, pending, or throttled can be canceled by invoking this method. Invoking this method for other tasks will delete them.

The output data associated with a deleted task will be deleted from persistent storage in accordance with our data retention policy. Aborted and deleted tasks will not be able to be fetched again in the future.
Authentication

Authorization

    Use the HTTP Authorization header with the Bearer scheme along with an API key.

Path parameters

id
Requiredstring <uuid>

    The ID of a previously-submitted task that has not been canceled or deleted.

Headers

X-Runway-Version
Requiredstring

    This field must be set to the exact value 2024-11-06.

Responses


example curl call image to video 

{
    "model": "gen3a_turbo",
    "promptText": "Butterflies flutter and and whole image moves in psychedlic pattern",
    "promptImage": [
        {
            "uri": "https://images.nightcafe.studio/jobs/Uw0oOLek3mY81AOS9Zys/Uw0oOLek3mY81AOS9Zys--1--0vmq9_6x.jpg",
            "position": "last"
        }
    ],
    "ratio": "1280:768"
}

for task retrieval 
https://api.dev.runwayml.com/v1/tasks/:id

example output 

{
  "id": "8900bf98-7913-4603-ad9d-257a23c2e624",
  "status": "SUCCEEDED",
  "createdAt": "2025-06-25T11:10:39.688Z",
  "output": [
    "https://dnznrvs05pmza.cloudfront.net/6f0a59be-1133-4394-ab4a-cfab4601243b.mp4?_jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJrZXlIYXNoIjoiYjljYzVlODA1MWViNmM1ZCIsImJ1Y2tldCI6InJ1bndheS10YXNrLWFydGlmYWN0cyIsInN0YWdlIjoicHJvZCIsImV4cCI6MTc1MDk4MjQwMH0.1epmckr5LdHMOhf7QtLayUjbYbcbqZhtJYJo6BIoYV4"
  ]
}


https://api.dev.runwayml.com endpoint 

does not support text to image 

delete task

curl -X DELETE https://api.dev.runwayml.com/v1/tasks/{id} \
  -H "Authorization: Bearer {{ YOUR API KEY }}" \
  -H "X-Runway-Version: 2024-11-06"


  output for success image-to-video call 

  Response Schema: application/json

id
Requiredstring <uuid>

    The ID of the newly created task. Use this ID to query the task status and retrieve the generated video.
