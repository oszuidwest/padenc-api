# PADENC API Server

A Rust HTTP server for managing DAB metadata (DLS text and MOT slideshow images) for ODR PADENC.

## What is PADENC API?

PADENC API provides a simple HTTP interface for dynamically managing DAB metadata in digital radio broadcasts. It handles:

- Track information (song titles and artists)
- Program information (show names)
- Station information (default fallback)
- MOT slideshow images for each content type
- DL Plus tagging for improved text display

The server automatically formats output for compatibility with [ODR PADENC](https://github.com/Opendigitalradio/ODR-PadEnc) and handles content expiration for seamless transitions between tracks, programs, and fallback station information.

## Integration with ODR PADENC

This API server generates two key outputs:

1. **DLS text file** (`/data/dls.txt`) - Contains formatted text with DL Plus tags
2. **MOT slideshow images** (`/data/mot` directory) - Contains the currently active image

To connect ODR PADENC to this API:

1. Mount the `/data` directory as a volume in both containers
2. Configure ODR PADENC to read from these locations:

```
odr-padenc \
  --dls=/data/dls.txt \
  --dir=/data/mot \
  --erase \
  --output=dab/pad
```

## Configuration

### Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `STATION_NAME` | Station name displayed when no track/program is active | Yes | - |
| `API_KEY` | Secret key for Bearer token authentication | Yes | - |
| `DEFAULT_STATION_IMAGE` | Path to default station image | No | - |
| `RUST_LOG` | Log level (info, debug, etc.) | No | info |

### Fixed Paths

| Path | Description |
|------|-------------|
| `/data/dls.txt` | DLS output file read by ODR PADENC |
| `/data/mot` | MOT slideshow directory read by ODR PADENC |
| `/tmp/padenc/images` | Internal storage for images (no persistence needed) |

## API Endpoints

All endpoints require authentication with a Bearer token matching the `API_KEY`.

### POST /track

Sets the currently playing track information and optional image.

#### JSON Request (text only)

```bash
curl -X POST http://localhost:8080/track \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_secret_api_key_here" \
  -d '{
    "item": {
        "title": "Viva la Vida",
        "artist": "Coldplay"
    },
    "expires_at": "2023-12-31T23:59:59Z"
  }'
```

#### Multipart Form Request (with image)

```bash
curl -X POST http://localhost:8080/track \
  -H "Authorization: Bearer your_secret_api_key_here" \
  -F 'track_info={
    "item": {
      "title": "Viva la Vida",
      "artist": "Coldplay"
    },
    "expires_at": "2023-12-31T23:59:59Z"
  }' \
  -F 'image=@/path/to/album_cover.jpg'
```

#### Response

```
Status: 200 OK
{
    "status": "success",
    "message": "Track updated successfully"
}
```

### DELETE /track

Removes the current track information and associated image.

```bash
curl -X DELETE http://localhost:8080/track \
  -H "Authorization: Bearer your_secret_api_key_here"
```

#### Response

```
Status: 200 OK
{
    "status": "success",
    "message": "Track removed successfully"
}
```

### POST /program

Sets the current program information and optional image.

#### JSON Request (text only)

```bash
curl -X POST http://localhost:8080/program \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_secret_api_key_here" \
  -d '{
    "name": "Morning Show with DJ Smith",
    "expires_at": "2023-12-31T12:00:00Z"
  }'
```

#### Multipart Form Request (with image)

```bash
curl -X POST http://localhost:8080/program \
  -H "Authorization: Bearer your_secret_api_key_here" \
  -F 'program_info={
    "name": "Morning Show with DJ Smith",
    "expires_at": "2023-12-31T12:00:00Z"
  }' \
  -F 'image=@/path/to/program_logo.jpg'
```

#### Response

```
Status: 200 OK
{
    "status": "success",
    "message": "Program updated successfully"
}
```

### DELETE /program

Removes the current program information and associated image.

```bash
curl -X DELETE http://localhost:8080/program \
  -H "Authorization: Bearer your_secret_api_key_here"
```

#### Response

```
Status: 200 OK
{
    "status": "success",
    "message": "Program removed successfully"
}
```

## Output Format

### DLS Text Format

The server generates properly formatted DLS text files with DL Plus tags for optimal display on DAB receivers:

#### Track Format

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=1 0 9  # Title tag
DL_PLUS_TAG=4 12 16  # Artist tag
##### parameters } #####
Coldplay - Viva la Vida
```

#### Program Format

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=33 0 23  # Program tag
##### parameters } #####
Morning Show with DJ Smith
```

#### Station Format (fallback)

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=31 0 8  # Station tag
##### parameters } #####
Radio One
```

### MOT Slideshow Format

The server automatically manages images in the MOT directory following this priority:

1. Track image (if a track is active)
2. Program image (if no track but program is active)
3. Default station image (if configured and no track/program is active)

## Deployment with Docker

The easiest way to deploy PADENC API is to use the prebuilt Docker images:

```bash
# Pull the latest version
docker pull ghcr.io/oszuidwest/padenc-api:latest

# Or pull a specific version
docker pull ghcr.io/oszuidwest/padenc-api:v1.0.0
```

Run the container:

```bash
docker run -d \
  --name padenc-api \
  -p 8080:8080 \
  -e STATION_NAME="My Radio Station" \
  -e API_KEY="your_secret_key_here" \
  -e DEFAULT_STATION_IMAGE="/data/default_station.jpg" \
  -v $(pwd)/data:/data \
  ghcr.io/oszuidwest/padenc-api:latest
```

### Quick Start with Docker Compose

1. Create a `.env` file based on `.env.example`:

```
STATION_NAME=My Radio Station
API_KEY=your_secret_key_here
DEFAULT_STATION_IMAGE=/data/default_station.jpg
RUST_LOG=info
```

2. Create a `data` directory and add your default station image (optional):

```bash
mkdir -p data
cp path/to/your/logo.jpg data/default_station.jpg
```

3. Start the server using Docker Compose:

```bash
docker-compose up -d
```

### Complete ODR Stack Example

Here's a complete `docker-compose.yml` example with PADENC API, ODR-PadEnc, and ODR-AudioEnc:

```yaml
services:
  padenc-api:
    image: ghcr.io/oszuidwest/padenc-api:latest
    container_name: padenc-api
    ports:
      - "8080:8080"
    environment:
      - STATION_NAME=My Radio Station
      - DEFAULT_STATION_IMAGE=/data/default_station.jpg
      - RUST_LOG=info
      - API_KEY=your_secret_api_key_here
    volumes:
      - ./data:/data
    restart: unless-stopped

  odr-padenc:
    image: ghcr.io/oszuidwest/odr-padenc:latest
    container_name: odr-padenc
    depends_on:
      - padenc-api
    volumes:
      - ./data:/data
      - odr_socket:/tmp/dab
    environment:
      - DLS_FILE=/data/dls.txt
    command: >
      odr-padenc
      --dls=/data/dls.txt
      --dir=/data/mot
      --output=dab/pad
      --charset=0
      --erase
    restart: unless-stopped

  odr-audioenc:
    image: ghcr.io/oszuidwest/odr-audioenc-full:latest
    container_name: odr-audioenc
    volumes:
      - odr_socket:/tmp/dab
    command: >
      odr-audioenc
      --vlc-uri http://stream.example.com/mystream
      -r 48000 -b 96
      -P dab/pad
      -e tcp://dabmux:9000
    restart: unless-stopped

volumes:
  odr_socket:
```

## Common Usage Examples

### Set a New Track with Image

```bash
curl -X POST http://localhost:8080/track \
  -H "Authorization: Bearer your_secret_api_key_here" \
  -F 'track_info={
    "item": {
      "title": "Bohemian Rhapsody",
      "artist": "Queen"
    },
    "expires_at": "2023-12-31T23:59:59Z"
  }' \
  -F 'image=@/path/to/queen_cover.jpg'
```

### Update Program Information

```bash
curl -X POST http://localhost:8080/program \
  -H "Authorization: Bearer your_secret_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Evening Jazz",
    "expires_at": "2023-12-31T23:59:59Z"
  }'
```

### Clear Track Information

```bash
curl -X DELETE http://localhost:8080/track \
  -H "Authorization: Bearer your_secret_api_key_here"
```

## Troubleshooting

### Common Issues

1. **No DLS Text Output**
   - Check if `/data/dls.txt` exists and is writable
   - Verify environment variables are correctly set
   - Check logs for any file permission errors

2. **MOT Images Not Showing**
   - Verify image format (only JPEG/PNG supported)
   - Check if `/data/mot` directory exists and is writable
   - Verify image file size (should be reasonable for DAB transmission)

3. **Authentication Failures**
   - Confirm `API_KEY` environment variable matches the Bearer token
   - Check for typos or whitespace in the token

### Logging

Adjust the `RUST_LOG` environment variable for more detailed logs:

```
RUST_LOG=debug   # For detailed debugging information
RUST_LOG=info    # For normal operation information
RUST_LOG=warn    # For warnings only
```

## Security Considerations

- Always use a strong, random API key
- Deploy behind a reverse proxy with HTTPS in production
- Consider IP whitelisting for additional protection
