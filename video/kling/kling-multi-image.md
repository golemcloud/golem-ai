Video Generation - Multi-Image to Video
Create Task
Protocol	Request URL	Request Method	Request Format	Response Format
https	/v1/videos/multi-image2video	POST	application/json	application/json
Request Header
Field	Value	Description
Content-Type	application/json	Data Exchange Format
Authorization	Authentication information, refer to API authentication	Authentication information, refer to API authentication
Request Body
Field	Field	Field	Field	Field
model_name	string	Optional	kling-v1-6	

Model Name

    Enum values：kling-v1-6

image_list	array	Required	Null	

Reference Image List

    Support up to 4 images, load with key:value, details as follows:

1
2
3
4
5
6
7
8
9
10
11
12
13
14
"image_list":[
	{
  	"image":"image_url"
  },
	{
  	"image":"image_url"
  },
	{
  	"image":"image_url"
  },
	{
  	"image":"image_url"
  }
]

    Please directly upload the image with selected subject since there is no cropping logic on the API side.
    Support inputting image Base64 encoding or image URL (ensure accessibility)

Please note, if you use the Base64 method, make sure all image data parameters you pass are in Base64 encoding format. When submitting data, do not add any prefixes to the Base64-encoded string, such as data:image/png;base64. The correct parameter format should be the Base64-encoded string itself.
Example:
Correct Base64 encoded parameter:

1
iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==

Incorrect Base64 encoded parameter (includes the data: prefix):

1
data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==

Please provide only the Base64-encoded string portion so that the system can correctly process and parse your data.

    Supported image formats include.jpg / .jpeg / .png
    The image file size cannot exceed 10MB, and the image resolution should not be less than 300*300px, and the aspect ratio of the image should be between 1:2.5 ~ 2.5:1

prompt	string	Optional	None	

Positive text prompt

    Cannot exceed 2500 characters

negative_prompt	string	Optional	Null	

Negative text prompt

    Cannot exceed 2500 characters

mode	string	Optional	std	

Video generation mode

    Enum values: std, pro
    std: Standard Mode, which is cost-effective
    pro: Professional Mode, generates videos use longer duration but higher quality video output

The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"
duration	string	Optional	5	

Video Length, unit: s (seconds)

    Enum values：5，10

Please note, requests that include the end frame (image_tail) and motion brush (dynamic_masks & static_mask) currently only support the generation of videos up to 5 seconds long.
aspect_ratio	string	Optional	16:9	

The aspect ratio of the generated video frame (width:height)

    Enum values：16:9, 9:16, 1:1

callback_url	string	Optional	null	

The callback notification address for the result of this task. If configured, the server will actively notify when the task status changes

    The specific message schema of the notification can be found in “Callback Protocol”

external_task_id	string	Optional	None	

Customized Task ID

    Users can provide a customized task ID, which will not overwrite the system-generated task ID but can be used for task queries.
    Please note that the customized task ID must be unique within a single user account.