Video Effects
Create Task
Protocol	Request URL	Request Method	Request Format	Response Format
https	/v1/videos/effects	POST	application/json	application/json
Request Header
Field	Value	Description
Content-Type	application/json	Data Exchange Format
Authorization	Authentication information, refer to API authentication	Authentication information, refer to API authentication
General Request Body

You can achieve different special effects based on the effect_scene. Currently supported are:

    Single Image Effects: 5 types available, bloombloom, dizzydizzy, fuzzyfuzzy, squish, expansion
    Dual-character Effects: 3 types available, hug, kiss, heart_gesture

Field	Type	Required Field	Default	Description
effect_scene	string	Required	None	

Scene Name

    Enum Values：bloombloom, dizzydizzy, fuzzyfuzzy, squish, expansion, hug, kiss, heart_gesture

input	object	Required	None	

Supports different task input structures.

    Depending on the scene, the fields passed in the structure vary, as detailed in the “Scene Request Body”.

callback_url	string	Optional	None	

The callback notification address for the result of this task. If configured, the server will actively notify when the task status changes

    The specific message schema of the notification can be found in “Callback Protocol”

external_task_id	string	Optional	None	

Customized Task ID

    Users can provide a customized task ID, which will not overwrite the system-generated task ID but can be used for task queries.
    Please note that the customized task ID must be unique within a single user account.

Scene Request Body

Single Image Effect: 5 types available, bloombloom, dizzydizzy, fuzzyfuzzy, squish, expansion
Field	Type	Required Field	Default	Description
model_name	string	Required	Null	

Model Name

    Enum Values：kling-v1-6

image	string	Required	Null	

Reference Image

    Support inputting image Base64 encoding or image URL (ensure accessibility)

Please note, if you use the Base64 method, make sure all image data parameters you pass are in Base64 encoding format. When submitting data, do not add any prefixes to the Base64-encoded string, such as data:image/png;base64. The correct parameter format should be the Base64-encoded string itself.
Example: Correct Base64 encoded parameter:

1
iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==

Incorrect Base64 encoded parameter (includes the data: prefix):

1
data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==

Please provide only the Base64-encoded string portion so that the system can correctly process and parse your data.

    Supported image formats include.jpg / .jpeg / .png
    The image file size cannot exceed 10MB, and the width and height dimensions of the image shall not be less than 300px, and the aspect ratio of the image should be between 1:2.5 ~ 2.5:1

duration	string	Required	Null	

Video Length, unit: s (seconds)

    Enum values：5

Dual-character Effects: 3 types available, hug, kiss, heart_gesture
Field	Type	Required Field	Default	Description
model_name	string	Optional	kling-v1	

Model Name

    Enum Values：kling-v1, kling-v1-5, kling-v1-6

mode	string	Optional	std	

Video generation mode

    Enum values: std, prox
    std: Standard Mode, which is cost-effective
    pro: Professional Mode, generates videos use longer duration but higher quality video output

image	Array[string]	Required	Null	

Reference Image Group
The length of the array must be 2. The first image uploaded will be positioned on the left side of the composite photo, and the second image uploaded will be positioned on the right side of the composite photo.
"https://p2-kling.klingai.com/bs2/upload-ylab-stunt/c54e463c95816d959602f1f2541c62b2.png?x-kcdn-pid=112452",
"https://p2-kling.klingai.com/bs2/upload-ylab-stunt/5eef15e03a70e1fa80732808a2f50f3f.png?x-kcdn-pid=112452"
The resulting effect of the composite photo is as follows:

    Support inputting image Base64 encoding or image URL (ensure accessibility)

Please note, if you use the Base64 method, make sure all image data parameters you pass are in Base64 encoding format. When submitting data, do not add any prefixes to the Base64-encoded string, such as data:image/png;base64. The correct parameter format should be the Base64-encoded string itself.
Example: Correct Base64 encoded parameter:

1
iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==

Incorrect Base64 encoded parameter (includes the data: prefix):

1
data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==

Please provide only the Base64-encoded string portion so that the system can correctly process and parse your data.

    Supported image formats include.jpg / .jpeg / .png
    The image file size cannot exceed 10MB, and the width and height dimensions of the image shall not be less than 300px

The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"
duration	string	Required	Null	

Video Length, unit: s (seconds)

    Enum values：5，10

Request Example
JSON
Copy
Collapse

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
{
  "effect_scene": "hug",
  "input":{
  	"model_name": "kling-v1-6",
    "mode": "std",
    "images":[
    	"https://p2-kling.klingai.com/bs2/upload-ylab-stunt/c54e463c95816d959602f1f2541c62b2.png?x-kcdn-pid=112452",
      "https://p2-kling.klingai.com/bs2/upload-ylab-stunt/5eef15e03a70e1fa80732808a2f50f3f.png?x-kcdn-pid=112452"
    ],
    "duration": "5"
  }
}

Response Body
JSON
Copy
Collapse

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
{
  "code": 0, //Error codes；Specific definitions can be found in "Error Code"
  "message": "string", //Error information
  "request_id": "string", //Request ID, generated by the system
  "data":{
  	"task_id": "string", //Task ID, generated by the system
    "task_status": "string", //Task status, Enum values：submitted、processing、succeed、failed
    "task_info":{ //Task creation parameters
    	"external_task_id": "string" //Customer-defined task ID
    },
    "created_at": 1722769557708, //Task creation time, Unix timestamp, unit ms
    "updated_at": 1722769557708 //Task update time, Unix timestamp, unit ms
  }
}
