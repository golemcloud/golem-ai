Video Generation - Text to Video
Create Task
Protocol	Request URL	Request Method	Request Format	Response Format
https	/v1/videos/text2video	POST	application/json	application/json
Request Header
Field	Value	Description
Content-Type	application/json	Data Exchange Format
Authorization	Authentication information, refer to API authentication	Authentication information, refer to API authentication
Request Body
💡

Please note that in order to maintain naming consistency, the original model field has been changed to model_name, so in the future, please use this field to specify the version of the model that needs to be called.

    At the same time, we keep the behavior forward-compatible, if you continue to use the original model field, it will not have any impact on the interface call, there will not be any exception, which is equivalent to the default behavior when model_name is empty (i.e., call the V1 model).


Field	Type	Required Field	Default	Description
model_name	string	Optional	kling-v1	Model Name
Enum values：kling-v1, kling-v1-6, kling-v2-master, kling-v2-1-master
prompt	string	Required	None	Positive text prompt
Cannot exceed 2500 characters
negative_prompt	string	Optional	Null	Negative text prompt
Cannot exceed 2500 characters
cfg_scale	float	Optional	0.5	Flexibility in video generation; The higher the value, the lower the model’s degree of flexibility, and the stronger the relevance to the user’s prompt.
Value range: [0, 1]
mode	string	Optional	std	

Video generation mode

    Enum values: std, pro
    std: Standard Mode, which is cost-effective
    pro: Professional Mode, generates videos use longer duration but higher quality video output

camera_control	object	Optional	Null	

Terms of controlling camera movement ( If not specified, the model will intelligently match based on the input text/images)
The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"

camera_control

    type

	string	Optional	None	

Predefined camera movements type

    Enum values: “simple”, “down_back”, “forward_up”, “right_turn_forward”, “left_turn_forward”
    simple: Camera movement，Under this Type, you can choose one out of six options for camera movement in the “config”.
    down_back: Camera descends and moves backward ➡️ Pan down and zoom out, Under this Type, the config parameter must be set to “None”.
    forward_up: Camera moves forward and tilts up ➡️ Zoom in and pan up, the config parameter must be set to “None”.
    right_turn_forward: Rotate right and move forward ➡️ Rotate right and advance, the config parameter must be set to “None”.
    left_turn_forward: Rotate left and move forward ➡️ Rotate left and advance, the config parameter must be set to “None”.

camera_control

    config

	object	Optional	None	

Contains 6 Fields, used to specify the camera’s movement or change in different directions

    When the camera movement Type is set to simple, the Required Field must be filled out; when other Types are specified, it should be left blank.
    Choose one out of the following six parameters, meaning only one parameter should be non-zero, while the rest should be zero.

config

    horizontal

	float	Optional	None	Horizontal, controls the camera’s movement along the horizontal axis (translation along the x-axis).
Value range：[-10, 10], a negative Value indicates a translation to the left, while a positive Value indicates a translation to the right.

config

    vertical

	float	Optional	None	Vertical, controls the camera’s movement along the vertical axis (translation along the y-axis).
Value range：[-10, 10], a negative Value indicates a downward translation, while a positive Value indicates an upward translation.

config

    pan

	float	Optional	None	Pan, controls the camera’s rotation in the vertical plane (rotation around the x-axis).
Value range：[-10, 10]，a negative Value indicates a downward rotation around the x-axis, while a positive Value indicates an upward rotation around the x-axis.

config

    tilt

	float	Optional	None	Tilt, controls the camera’s rotation in the horizontal plane (rotation around the y-axis).
Value range：[-10, 10]，a negative Value indicates a rotation to the left around the y-axis, while a positive Value indicates a rotation to the right around the y-axis.

config

    roll

	float	Optional	None	Roll, controls the camera’s rolling amount (rotation around the z-axis).
Value range：[-10, 10]，a negative Value indicates a counterclockwise rotation around the z-axis, while a positive Value indicates a clockwise rotation around the z-axis.

config

    zoom

	float	Optional	None	Zoom, controls the change in the camera’s focal length, affecting the proximity of the field of view.
Value range：[-10, 10], A negative Value indicates an increase in focal length, resulting in a narrower field of view, while a positive Value indicates a decrease in focal length, resulting in a wider field of view.
aspect_ratio	string	Optional	16:9	The aspect ratio of the generated video frame (width:height)
Enum values：16:9, 9:16, 1:1
duration	string	Optional	5	Video Length, unit: s (seconds)
Enum values: 5，10
callback_url	string	Optional	None	The callback notification address for the result of this task. If configured, the server will actively notify when the task status changes
The specific message schema of the notification can be found in “Callback Protocol”
external_task_id	string	Optional	None	

Customized Task ID

    Users can provide a customized task ID, which will not overwrite the system-generated task ID but can be used for task queries.
    Please note that the customized task ID must be unique within a single user account.

response body

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

Query Task (Single)
Protocol	Request URL	Request Method	Request Format	Response Format
https	/v1/videos/text2video/{id}	GET	application/json	application/json
Request Header
Field	Value	Description
Content-Type	application/json	Data Exchange Format
Authorization	Authentication information, refer to API authentication	Authentication information, refer to API authentication
Request Path Parameters
Field	Type	Required Field	Default	Description
task_id	string	Optional	None	Task ID for Text to Video
Request Path Parameters，directly fill the Value in the request path
When creating a task, you can choose to query by external_task_id or task_id.
external_task_id	string	Optional	None	Customized Task ID for Text-to-Video
Request Path Parameters，directly fill the Value in the request path
When creating a task, you can choose to query by external_task_id or task_id.
Request Body

None
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
15
16
17
18
19
20
21
22
23
24
{
  "code": 0, //Error codes；Specific definitions can be found in "Error Code"
  "message": "string", //Error information
  "request_id": "string", //Request ID, generated by thTask ID, generated by the system is used to track requests and troubleshoot problems
  "data":{
  	"task_id": "string", //Task ID, generated by the system
    "task_status": "string", //Task status, Enum values：submitted、processing、succeed、failed
    "task_status_msg": "string", //Task status information, displaying the failure reason when the task fails (such as triggering the content risk control of the platform, etc.)
    "task_info":{ //Task creation parameters
    	"external_task_id": "string" //Customer-defined task ID
    },
    "created_at": 1722769557708, //Task creation time, Unix timestamp, unit: ms
    "updated_at": 1722769557708, //Task update time, Unix timestamp, unit: ms
    "task_result":{
      "videos":[
        {
        	"id": "string", //Generated video ID; globally unique
      		"url": "string", //URL for generating videos，such as https://p1.a.kwimgs.com/bs2/upload-ylab-stunt/special-effect/output/HB1_PROD_ai_web_46554461/-2878350957757294165/output.mp4(To ensure information security, generated images/videos will be cleared after 30 days. Please make sure to save them promptly.)
      		"duration": "string" //Total video duration, unit: s (seconds)
        }
      ]
    }
  }
}

Video Generation - Image to Video
Create Task
Protocol	Request URL	Request Method	Request Format	Response Format
https	/v1/videos/image2video	POST	application/json	application/json
Request Header
Field	Value	Description
Content-Type	application/json	Data Exchange Format
Authorization	Authentication information, refer to API authentication	Authentication information, refer to API authentication
Request Body
💡

Please note that in order to maintain naming consistency, the original model field has been changed to model_name, so in the future, please use this field to specify the version of the model that needs to be called.

    At the same time, we keep the behavior forward-compatible, if you continue to use the original model field, it will not have any impact on the interface call, there will not be any exception, which is equivalent to the default behavior when model_name is empty (i.e., call the V1 model).

Bash
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
15
16
17
18
19
20
curl --location --request POST 'https://api.klingai.com/v1/videos/image2video' \
--header 'Authorization: Bearer xxx' \
--header 'Content-Type: application/json' \
--data-raw '{
    "model_name": "kling-v1",
    "mode": "pro",
    "duration": "5",
    "image": "https://h2.inkwai.com/bs2/upload-ylab-stunt/se/ai_portal_queue_mmu_image_upscale_aiweb/3214b798-e1b4-4b00-b7af-72b5b0417420_raw_image_0.jpg",
    "prompt": "The astronaut stood up and walked away",
    "cfg_scale": 0.5,
    "static_mask": "https://h2.inkwai.com/bs2/upload-ylab-stunt/ai_portal/1732888177/cOLNrShrSO/static_mask.png",
    "dynamic_masks": [
      {
        "mask": "https://h2.inkwai.com/bs2/upload-ylab-stunt/ai_portal/1732888130/WU8spl23dA/dynamic_mask_1.png",
        "trajectories": [
          {"x":279,"y":219},{"x":417,"y":65}
        ]
      }
    ]
}'

Field	Type	Required Field	Default	Description
model_name	string	Optional	kling-v1	

Model Name

    Enum values：kling-v1, kling-v1-5, kling-v1-6, kling-v2-master, kling-v2-1, kling-v2-1-master

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
    At least one parameter should be filled in between parameter image and parameter image_tail; cannot both be empty at the same time
    image+image_tail, dynamic_masks/static_mask, and camera_control. These three parameters cannot be used at the same time

The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"
image_tail	string	Optional	Null	

Reference Image - End frame control

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
    At least one parameter should be filled in between parameter image and parameter image_tail; cannot both be empty at the same time
    image+image_tail, dynamic_masks/static_mask, and camera_control. These three parameters cannot be used at the same time

The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"
prompt	string	Optional	None	

Positive text prompt

    Cannot exceed 2500 characters

negative_prompt	string	Optional	Null	

Negative text prompt

    Cannot exceed 2500 characters

cfg_scale	float	Optional	0.5	

Flexibility in video generation; The higher the value, the lower the model’s degree of flexibility, and the stronger the relevance to the user’s prompt.

    Value range: [0, 1]

mode	string	Optional	std	

Video generation mode

    Enum values: std, pro
    std: Standard Mode, which is cost-effective
    pro: Professional Mode, generates videos use longer duration but higher quality video output

The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"
static_mask	string	Optional	null	

Static Brush Application Area (Mask image created by users using the motion brush)
The "Motion Brush" feature includes two types of brushes: Dynamic Brush (dynamic_masks) and Static Brush (static_mask).

    Support inputting image Base64 encoding or image URL (ensure the URL is accessible and follows the same format requirements as the image field).
    Supported image formats include.jpg / .jpeg / .png
    The aspect ratio of the mask image must match the input image (image field); otherwise, the task will fail (failed).
    The resolutions of the static_mask image and the dynamic_masks.mask image must be identical; otherwise, the task will fail (failed).

The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"
Please refer to the code block above for a specific example.
dynamic_masks	array	Optional	null	

Dynamic Brush Configuration List
Multiple configurations can be set up (up to 6 groups). Each group includes a "mask area" (mask) and a sequence of "motion trajectories" (trajectories).\nUntil 20241129: The Dynamic Brush feature only supports the kling-v1 model in Standard Mode (std 5s) and Professional Mode (pro 5s).\nPlease refer to the code block above for a specific example.

dynamic_masks

    mask

	string	Optional	null	

Dynamic Brush Application Area (Mask image created by users using the motion brush)

    Support inputting image Base64 encoding or image URL (ensure the URL is accessible and follows the same format requirements as the image field).
    Supported image formats include.jpg / .jpeg / .png
    The aspect ratio of the mask image must match the input image (image field); otherwise, the task will fail (failed).
    The resolutions of the static_mask image and the dynamic_masks.mask image must be identical; otherwise, the task will fail (failed).

dynamic_masks

    trajectories

	array	Optional	null	

Motion Trajectory Coordinate Sequence

    To generate a 5-second video, the trajectory length must not exceed 77 coordinates, with the number of coordinates ranging from [2, 77].
    The coordinate system is based on the bottom-left corner of the image as the origin point.

Note-1: The more coordinates provided, the more precise the trajectory will be. For example, if only two trajectory points are provided, the motion will form a straight line connecting these two points.
Note-2: The trajectory direction follows the input order. The first coordinate serves as the starting point, and subsequent coordinates are connected sequentially to form the motion trajectory.

dynamic_masks

    trajectories
        x

	int	Optional	null	The horizontal coordinate (X-coordinate) of each trajectory point is defined within a 2D pixel coordinate system, where the bottom-left corner of the input image (image) serves as the origin point (0, 0).

dynamic_masks

    trajectories
        y

	int	Optional	null	The vertical coordinate (Y-coordinate) of each trajectory point is defined within a 2D pixel coordinate system, where the bottom-left corner of the input image (image) serves as the origin point (0, 0).
camera_control	object	Optional	Null	

Terms of controlling camera movement ( If not specified, the model will intelligently match based on the input text/images)
The support range for different model versions and video modes varies. For more details, please refer to the current document's "3-0 Capability Map"

camera_control

    type

	string	Optional	None	

Predefined camera movements type

    Enum values: “simple”, “down_back”, “forward_up”, “right_turn_forward”, “left_turn_forward”
    simple: Camera movement，Under this Type, you can choose one out of six options for camera movement in the “config”.
    down_back: Camera descends and moves backward ➡️ Pan down and zoom out, Under this Type, the config parameter must be set to “None”.
    forward_up: Camera moves forward and tilts up ➡️ Zoom in and pan up, the config parameter must be set to “None”.
    right_turn_forward: Rotate right and move forward ➡️ Rotate right and advance, the config parameter must be set to “None”.
    left_turn_forward: Rotate left and move forward ➡️ Rotate left and advance, the config parameter must be set to “None”.

camera_control

    config

	object	Optional	None	

Contains 6 Fields, used to specify the camera’s movement or change in different directions

    When the camera movement Type is set to simple, the Required Field must be filled out; when other Types are specified, it should be left blank.
    Choose one out of the following six parameters, meaning only one parameter should be non-zero, while the rest should be zero.

config

    horizontal

	float	Optional	None	

Horizontal, controls the camera’s movement along the horizontal axis (translation along the x-axis).

    Value range：[-10, 10], a negative Value indicates a translation to the left, while a positive Value indicates a translation to the right.

config

    vertical

	float	Optional	None	

Vertical, controls the camera’s movement along the vertical axis (translation along the y-axis).

    Value range：[-10, 10], a negative Value indicates a downward translation, while a positive Value indicates an upward translation.

config

    pan

	float	Optional	None	

Pan, controls the camera’s rotation in the vertical plane (rotation around the x-axis).

    Value range：[-10, 10]，a negative Value indicates a downward rotation around the x-axis, while a positive Value indicates an upward rotation around the x-axis.

config

    tilt

	float	Optional	None	

Tilt, controls the camera’s rotation in the horizontal plane (rotation around the y-axis).

    Value range：[-10, 10]，a negative Value indicates a rotation to the left around the y-axis, while a positive Value indicates a rotation to the right around the y-axis.

config

    roll

	float	Optional	None	

Roll, controls the camera’s rolling amount (rotation around the z-axis).

    Value range：[-10, 10]，a negative Value indicates a counterclockwise rotation around the z-axis, while a positive Value indicates a clockwise rotation around the z-axis.

config

    zoom

	float	Optional	None	

Zoom, controls the change in the camera’s focal length, affecting the proximity of the field of view.

    Value range：[-10, 10], A negative Value indicates an increase in focal length, resulting in a narrower field of view, while a positive Value indicates a decrease in focal length, resulting in a wider field of view.

duration	string	Optional	5	

Video Length, unit: s (seconds)

    Enum values：5，10

callback_url	string	Optional	None	

The callback notification address for the result of this task. If configured, the server will actively notify when the task status changes

    The specific message schema of the notification can be found in "Callback Protocol

external_task_id	string	Optional	None	

Customized Task ID

    Users can provide a customized task ID, which will not overwrite the system-generated task ID but can be used for task queries.
    Please note that the customized task ID must be unique within a single user account.
