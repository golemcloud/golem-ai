POST
/v1/text_to_image

This endpoint will start a new task to generate images from text.
Authentication

Authorization

    Use the HTTP Authorization header with the Bearer scheme along with an API key.

Headers

X-Runway-Version
Requiredstring

    This field must be set to the exact value 2024-11-06.

Request body

promptText
Requiredstring<= 1000 characters

    A non-empty string up to 1000 characters (measured in UTF-16 code units). This should describe in detail what should appear in the output.
ratio
Requiredstring
    Accepted values:"1920:1080""1080:1920""1024:1024""1360:768""1080:1080""1168:880""1440:1080""1080:1440""1808:768""2112:912""1280:720""720:1280""720:720""960:720""720:960""1680:720"

    The resolution of the output image(s).
model
Requiredstring

    The model variant to use.

    This field must be set to the exact value gen4_image.
seed
integer[ 0 .. 4294967295 ]

    If unspecified, a random number is chosen. Varying the seed integer is a way to get different results for the same other request parameters. Using the same seed integer for an identical request will produce similar results.
referenceImages
Array of objects

    An array of images to be used as references for the generated image output. Up to three reference images can be provided.

    uri
    Requiredstring <uri>

        A HTTPS URL or data URI containing an encoded image to be used as reference for the generated output image. See our docs on image inputs for more information.
    tag
    string

        A name used to refer to the image reference, from 3 to 16 characters in length. Tags must be alphanumeric (plus underscores) and start with a letter. You can refer to the reference image's tag in the prompt text with at-mention syntax: @tag. Tags are case-sensitive.

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

    The ID of the newly created task.

429
   rate-limit-exceeded
