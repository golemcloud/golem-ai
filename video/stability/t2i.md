Stable Image Core

Our primary service for text-to-image generation, Stable Image Core represents the best quality achievable at high speed. No prompt engineering is required! Try asking for a style, a scene, or a character, and see what you get.
Try it out

Grab your API key and head over to Open Google Colab
How to use

Please invoke this endpoint with a POST request.

The headers of the request must include an API key in the authorization field. The body of the request must be multipart/form-data, and the accept header should be set to one of the following:

    image/* to receive the image in the format specified by the output_format parameter.
    application/json to receive the image encoded as base64 in a JSON response.

The body of the request should include:

    prompt

The body may optionally include:

    aspect_ratio
    negative_prompt
    seed
    style_preset
    output_format

    Note: for more details about these parameters please see the request schema below.

Output

The resolution of the generated image will be 1.5 megapixels.
Credits

Flat rate of 3 credits per successful generation. You will not be charged for failed generations.
Authorizations:
STABILITY_API_KEY
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
accept	
string
Default: image/*
Enum: application/json image/*

Specify image/* to receive the bytes of the image directly. Otherwise specify application/json to receive the image as base64 encoded JSON.
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
prompt
required
	
string [ 1 .. 10000 ] characters

What you wish to see in the output image. A strong, descriptive prompt that clearly defines elements, colors, and subjects will lead to better results.

To control the weight of a given word use the format (word:weight), where word is the word you'd like to control the weight of and weight is a value between 0 and 1. For example: The sky was a crisp (blue:0.3) and (green:0.8) would convey a sky that was blue and green, but more green than blue.
aspect_ratio	
string
Default: 1:1
Enum: 16:9 1:1 21:9 2:3 3:2 4:5 5:4 9:16 9:21

Controls the aspect ratio of the generated image.
negative_prompt	
string <= 10000 characters

A blurb of text describing what you do not wish to see in the output image. This is an advanced feature.
seed	
number [ 0 .. 4294967294 ]
Default: 0

A specific value that is used to guide the 'randomness' of the generation. (Omit this parameter or pass 0 to use a random seed.)
style_preset	
string
Enum: 3d-model analog-film anime cinematic comic-book digital-art enhance fantasy-art isometric line-art low-poly modeling-compound neon-punk origami photographic pixel-art tile-texture

Guides the image model towards a particular style.
output_format	
string
Default: png
Enum: jpeg png webp

Dictates the content-type of the generated image.
Responses


200 success
400 invalid parameter
403 content moderation
422 your request was well formed but rejected
429 too many requests
500 internal server error

Response Headers x-request-id	
string

A unique identifier for this request.
content-type	
string
Examples:

    image/png - raw bytes
    application/json; type=image/png - base64 encoded
    image/jpeg - raw bytes
    application/json; type=image/jpeg - base64 encoded
    image/webp - raw bytes
    application/json; type=image/webp - base64 encoded

The format of the generated image.

To receive the bytes of the image directly, specify image/* in the accept header. To receive the bytes base64 encoded inside of a JSON payload, specify application/json.
finish-reason	
string
Enum: CONTENT_FILTERED SUCCESS

Indicates the reason the generation finished.

    SUCCESS = successful generation.
    CONTENT_FILTERED = successful generation, however the output violated our content moderation policy and has been blurred as a result.

    NOTE: This header is absent on JSON encoded responses because it is present in the body as finish_reason.

seed	
string
Example: "343940597"

The seed used as random noise for this generation.

    NOTE: This header is absent on JSON encoded responses because it is present in the body as seed.

