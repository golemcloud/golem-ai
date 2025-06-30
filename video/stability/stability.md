Image-to-Video
Start generation

Generate a short video based on an initial image with Stable Video Diffusion, a latent video diffusion model.
How to use

Please invoke this endpoint with a POST request.

The headers of the request must include an API key in the authorization field. The body of the request must be multipart/form-data.

The body of the request should include:

    image

The body may optionally include:

    seed
    cfg_scale
    motion_bucket_id

    Note: for more details about these parameters please see the request schema below.

After invoking this endpoint with the required parameters, use the id in the response to poll for results at the image-to-video/result/{id} endpoint. Rate-limiting or other errors may occur if you poll more than once every 10 seconds.
Credits

Flat rate of 20 credits per successful generation. You will not be charged for failed generations.
Authorizations:
STABILITY_API_KEY
API Key: STABILITY_API_KEY

Use your Stability API key to authentication requests to this App.
Header parameter name: authorization
header Parameters
authorization
required
	
string non-empty

Your Stability API key, used to authenticate your requests. Although you may have multiple keys in your account, you should use the same key for all requests to this API.
content-type
required
	
string non-empty
Example: multipart/form-data

The content type of the request body. Do not manually specify this header; your HTTP client library will automatically include the appropriate boundary parameter.
stability-client-id	
string (StabilityClientID) <= 256 characters
Example: my-awesome-app

The name of your application, used to help us communicate app-specific debugging or moderation issues to you.
stability-client-user-id	
string (StabilityClientUserID) <= 256 characters
Example: DiscordUser#9999

A unique identifier for your end user. Used to help us communicate user-specific debugging or moderation issues to you. Feel free to obfuscate this value to protect user privacy.
stability-client-version	
string (StabilityClientVersion) <= 256 characters
Example: 1.2.1

The version of your application, used to help us communicate version-specific debugging or moderation issues to you.
Request Body schema: multipart/form-data
image
required
	
string <binary>

The source image used in the video generation process.

Supported Formats:

    jpeg
    png

Supported Dimensions:

    1024x576
    576x1024
    768x768

seed	
number [ 0 .. 4294967294 ]
Default: 0

A specific value that is used to guide the 'randomness' of the generation. (Omit this parameter or pass 0 to use a random seed.)
cfg_scale	
number [ 0 .. 10 ]
Default: 1.8

How strongly the video sticks to the original image. Use lower values to allow the model more freedom to make changes and higher values to correct motion distortions.
motion_bucket_id	
number [ 1 .. 255 ]
Default: 127

Lower values generally result in less motion in the output video, while higher values generally result in more motion. This parameter corresponds to the motion_bucket_id parameter from the paper.

Responses

## 200
Response Schema: application/json
id
required
	
string (GenerationID) = 64 characters

The id of a generation, typically used for async generations, that can be used to check the status of the generation or retrieve the result.
Response Schema: application/json
id
required
	
## 400    
string non-empty

A unique identifier associated with this error. Please include this in any support tickets you file, as it will greatly assist us in diagnosing the root cause of the problem.
name
required
	
string non-empty

Short-hand name for an error, useful for discriminating between errors with the same status code.
errors
required
	
Array of strings non-empty

One or more error messages indicating what went wrong.
Response Schema: application/json
id
required
	
## 403 
string non-empty

A unique identifier associated with this error. Please include this in any support tickets you file, as it will greatly assist us in diagnosing the root cause of the problem.
name
required
	
string non-empty
Value: content_moderation

Our content moderation system has flagged some part of your request and subsequently denied it. You were not charged for this request. While this may at times be frustrating, it is necessary to maintain the integrity of our platform and ensure a safe experience for all users.

If you would like to provide feedback, please use the Support Form.
errors
required
	
Array of strings non-empty

One or more error messages indicating what went wrong.
Response Schema: application/json
id
required
	
## 413
string non-empty

A unique identifier associated with this error. Please include this in any support tickets you file, as it will greatly assist us in diagnosing the root cause of the problem.
name
required
	
string non-empty

Short-hand name for an error, useful for discriminating between errors with the same status code.
errors
required
	
Array of strings non-empty

One or more error messages indicating what went wrong.
Response Schema: application/json
id
required
	
## 422
string non-empty

A unique identifier associated with this error. Please include this in any support tickets you file, as it will greatly assist us in diagnosing the root cause of the problem.
name
required
	
string non-empty

Short-hand name for an error, useful for discriminating between errors with the same status code.
errors
required
	
Array of strings non-empty

One or more error messages indicating what went wrong.
Response Schema: application/json
id
required
	
## 429
string non-empty

A unique identifier associated with this error. Please include this in any support tickets you file, as it will greatly assist us in diagnosing the root cause of the problem.
name
required
	
string non-empty

Short-hand name for an error, useful for discriminating between errors with the same status code.
errors
required
	
Array of strings non-empty

One or more error messages indicating what went wrong.
Response Schema: application/json
id
required

## 500

string non-empty

A unique identifier associated with this error. Please include this in any support tickets you file, as it will greatly assist us in diagnosing the root cause of the problem.
name
required
	
string non-empty

Short-hand name for an error, useful for discriminating between errors with the same status code.
errors
required
	
Array of strings non-empty

One or more error messages indicating what went wrong.

Example of a curl call for image to video
curl -f -sS "https://api.stability.ai/v2beta/image-to-video" \
  -H "authorization: Bearer sk-MYAPIKEY" \
  -F image=@"./image.png" \
  -F seed=0 \
  -F cfg_scale=1.8 \
  -F motion_bucket_id=127 \
  -o "./output.json"

Example of a curl call for polling 
generation_id="e52772ac75b..."      
url="https://api.stability.ai/v2beta/image-to-video/result/$generation_id"
http_status=$(curl -sS -f -o "./output.mp4" -w '%{http_code}' -H "authorization: sk-MYAPIKEY" -H 'accept: video/*' "$url")

case $http_status in
    202)
        echo "Still processing. Retrying in 10 seconds..."
        ;;
    200)
        echo "Download complete!"
        ;;
    4*|5*)
        mv "./output.mp4" "./error.json"
        echo "Error: Check ./error.json for details."
        exit 1
        ;;
esac
