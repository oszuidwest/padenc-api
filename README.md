# ODR Metadata Server

A simple HTTP server for managing DAB metadata and MOT slideshow images used by ODR PADENC.

## Overview

This server provides endpoints to:
- POST new track information (song title, artist) and optional image
- DELETE existing track information
- POST program information and optional image
- DELETE program information

When no track or program is active (or when their expiry times have passed), the server will output a file containing just the station name and the configured default station image will be used for MOT slideshow (if available).

## Configuration

The server uses the following environment variables:
- `STATION_NAME`: The name of the station to display when no metadata is active (required)
- `API_KEY`: Secret key used for Bearer token authentication (required)
- `DEFAULT_STATION_IMAGE`: Path to default station image file to be used when no track or program image is active (optional)

You can set these variables in a `.env` file in the project root or pass them to the Docker container.

## Fixed Paths

The following paths are fixed within the Docker container:
- DLS output file: `/data/dls.txt`
- Temporary image storage: `/tmp/padenc/images` (no need to persist this)
- MOT directory: `/data/mot`

When running the container, you should mount a volume to `/data` to persist the DLS output and MOT slideshow files. Temporary images are stored in memory and don't need to be persisted.

## Authentication

All API endpoints are protected by Bearer token authentication. You must include an `Authorization` header with a valid token in all requests:

```
Authorization: Bearer your_secret_api_key_here
```

The token should match the `API_KEY` environment variable value.

## API Endpoints

### POST /track

Used to add new information for a track, with optional image for MOT slideshow.

#### JSON Request
Use content-type `application/json` for text-only updates:

```json
{
    "item": {
        "title": "Viva la Vida",
        "artist": "Coldplay"
    },
    "expires_at": "2025-05-15T15:00:00Z"
}
```

#### Multipart Form Request
Use content-type `multipart/form-data` to include an image:

- `track_info`: JSON string with track data (same format as above)
- `image`: Image file (JPEG, PNG)

### DELETE /track

Removes the current track information and its associated image, reverting to displaying the program (if available) or station name.

### POST /program

Used to add program information with optional image for MOT slideshow.

#### JSON Request
Use content-type `application/json` for text-only updates:

```json
{
    "name": "Maartens Weekend Boost",
    "expires_at": "2025-05-15T15:00:00Z"
}
```

#### Multipart Form Request
Use content-type `multipart/form-data` to include an image:

- `program_info`: JSON string with program data (same format as above)
- `image`: Image file (JPEG, PNG)

### DELETE /program

Removes the current program information and its associated image, reverting to displaying only the station name if no track is available.

## Output Format

The server generates files compatible with ODR PADENC in the following formats:

### DLS Output Format

#### Track Metadata Format:

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=1 0 9  # Title tag
DL_PLUS_TAG=4 12 16  # Artist tag
##### parameters } #####
ColdPlay - Test
```

#### Program Format:

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=33 0 23  # Program tag
##### parameters } #####
Maartens Weekend Boost
```

#### Station Format:

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=31 0 8  # Station tag
##### parameters } #####
BredaNu
```

### MOT Output Format

For MOT slideshow images, the server copies the currently active image to the MOT directory for ODR PADENC to process. The image selection follows the same fallback strategy as text:

1. Track image (if track is active)
2. Program image (if no track image but program is active)
3. Default station image (if configured and no track or program image is available)

If no images are available, the MOT directory will be empty.

## Building & Running

### Standard Rust Build

```bash
# Build the project
cargo build --release

# Run the server
cargo run --release
```

### Docker

The server can be run in a Docker container. The port is hardcoded to 8080.

#### Build and run with Docker Compose:

```bash
# Start the server
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the server
docker-compose down
```

#### Build and run with Docker directly:

```bash
# Build the Docker image
docker build -t odr-metadata-server .

# Run the Docker container
docker run -d \
  --name odr-metadata-server \
  -p 8080:8080 \
  -e STATION_NAME=YourStation \
  -e API_KEY=your_secret_api_key_here \
  -e DEFAULT_STATION_IMAGE=/data/default_station.jpg \
  -v $(pwd)/data:/data \
  odr-metadata-server
```

### Accessing the API

Once running, the server can be accessed at:
- http://localhost:8080/track
- http://localhost:8080/program

Remember to include the `Authorization: Bearer your_secret_api_key_here` header in all requests.

Example using curl:

#### JSON request:
```bash
curl -X POST http://localhost:8080/track \
  -H 'Authorization: Bearer your_secret_api_key_here' \
  -H 'Content-Type: application/json' \
  -d '{"item":{"title":"Viva la Vida","artist":"Coldplay"},"expires_at":"2025-05-15T15:00:00Z"}'
```

#### Multipart form with image:
```bash
curl -X POST http://localhost:8080/track \
  -H 'Authorization: Bearer your_secret_api_key_here' \
  -F 'track_info={"item":{"title":"Viva la Vida","artist":"Coldplay"},"expires_at":"2025-05-15T15:00:00Z"}' \
  -F 'image=@/path/to/album_cover.jpg'
```

### Using with ODR PADENC

Mount the `/data` directory into your ODR PADENC container or system and configure PADENC to read from both the DLS output file (for text) and the MOT output directory (for images).

Example PADENC configuration:
```
odr-padenc \
  --dls=/data/dls.txt \
  --dir=/data/mot \
  --erase \
  --output=dab/pad
```

The application automatically manages image files:
- Uploaded images are temporarily stored in `/tmp/padenc/images`
- Active images for the MOT slideshow are copied to `/data/mot`
- Expired images are automatically cleaned up from the temporary storage

Note that the paths `/data/dls.txt` and `/data/mot` are fixed in the application. Make sure your Docker volume mapping aligns with these paths.