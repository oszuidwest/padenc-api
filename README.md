# ODR Metadata Server

A simple HTTP server for managing DAB metadata used by ODR PADENC.

## Overview

This server provides endpoints to:
- POST new track information (song title, artist)
- DELETE existing track information
- POST program information
- DELETE program information

When no track or program is active (or when their expiry times have passed), the server will output a file containing just the station name.

## Configuration

The server uses the following environment variables:
- `STATION_NAME`: The name of the station to display when no metadata is active
- `OUTPUT_FILE_PATH`: The path where the output file will be written
- `API_KEY`: Secret key used for Bearer token authentication (required)

You can set these variables in a `.env` file in the project root or pass them to the Docker container.

## Authentication

All API endpoints are protected by Bearer token authentication. You must include an `Authorization` header with a valid token in all requests:

```
Authorization: Bearer your_secret_api_key_here
```

The token should match the `API_KEY` environment variable value.

## API Endpoints

### POST /track

Used to add new information for a track.

Request body example:
```json
{
    "item": {
        "title": "Viva la Vida",
        "artist": "Coldplay"
    },
    "expires_at": "2025-05-15T15:00:00Z"
}
```

### DELETE /track

Removes the current track information, reverting to displaying the program (if available) or station name.

### POST /program

Used to add program information.

Request body example:
```json
{
    "name": "Maartens Weekend Boost",
    "expires_at": "2025-05-15T15:00:00Z"
}
```

### DELETE /program

Removes the current program information, reverting to displaying only the station name if no track is available.

## Output Format

The server generates files compatible with ODR PADENC in the following formats:

### Track Metadata Format:

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=1 0 9  # Title tag
DL_PLUS_TAG=4 12 16  # Artist tag
##### parameters } #####
ColdPlay - Test
```

### Program Format:

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=33 0 23  # Program tag
##### parameters } #####
Maartens Weekend Boost
```

### Station Format:

```
##### parameters { #####
DL_PLUS=1
DL_PLUS_TAG=31 0 8  # Station tag
##### parameters } #####
BredaNu
```

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
  -e OUTPUT_FILE_PATH=/data/track.txt \
  -e API_KEY=your_secret_api_key_here \
  -v $(pwd)/data:/data \
  odr-metadata-server
```

### Accessing the API

Once running, the server can be accessed at:
- http://localhost:8080/track
- http://localhost:8080/program

Remember to include the `Authorization: Bearer your_secret_api_key_here` header in all requests.

Example using curl:
```bash
curl -X POST http://localhost:8080/track \
  -H 'Authorization: Bearer your_secret_api_key_here' \
  -H 'Content-Type: application/json' \
  -d '{"item":{"title":"Viva la Vida","artist":"Coldplay"},"expires_at":"2025-05-15T15:00:00Z"}'
```

### Using with ODR PADENC

Mount the output directory into your ODR PADENC container or system and configure PADENC to read from the generated output file.