Upscale a video
POST
/v1/video_upscale

This endpoint will start a new task to upscale a video. Videos will be upscaled by a factor of 4X, capped at a maximum of 4096px along each side.
Authentication

Authorization

    Use the HTTP Authorization header with the Bearer scheme along with an API key.

Headers

X-Runway-Version
Requiredstring

    This field must be set to the exact value 2024-11-06.

Request body

videoUri
Requiredstring <uri>

    A HTTPS URL pointing to a video or a data URI containing a video. The video must be less than 4096px on each side. The video duration may not exceed 40 seconds. See our docs on video inputs for more information.
model
Requiredstring

    The model variant to use.

    This field must be set to the exact value upscale_v1.

Responses
Response Schema: application/json

id
Requiredstring <uuid>

    The ID of the newly created task.
