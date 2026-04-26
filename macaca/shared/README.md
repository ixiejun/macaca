# Weather API Service

A lightweight weather API service built with a Go backend and vanilla JavaScript frontend. The backend provides RESTful endpoints for current weather conditions and 7-day forecasts, while the frontend renders the data with automatic retry and error handling.

## Features

- **RESTful API** with current weather and forecast endpoints
- **Mock weather data** for development and testing (no external API key required)
- **CORS support** for cross-origin frontend requests
- **Structured JSON logging** via Go's `log/slog`
- **Request recovery** middleware to handle panics gracefully
- **Request ID tracking** on every request
- **Vanilla JavaScript frontend** with exponential-backoff retry logic
- **Configurable port** via environment variable

## Prerequisites

- [Go](https://go.dev/dl/) 1.23 or higher

## Installation

Clone the repository and download dependencies:

```bash
git clone <repository-url>
cd shared
go mod download
```

## Configuration

| Variable | Description       | Default |
|----------|-------------------|---------|
| `PORT`   | Server listen port | `8080`  |

The service currently uses mock data, so no external API key is needed. A real weather provider can be integrated in the future by extending `internal/services/weather_service.go`.

## Running the Backend

Start the development server:

```bash
make run
```

Or run directly with Go:

```bash
go run main.go
```

To use a custom port:

```bash
PORT=3000 make run
```

The server will start and log:

```json
{"level":"INFO","msg":"server starting","port":"8080"}
```

## Building

Compile the binary into `bin/server`:

```bash
make build
```

Run the compiled binary:

```bash
./bin/server
```

## Frontend Setup

The frontend is a single JavaScript file (`weather.js`) designed to be included in an HTML page. Create an `index.html` file with the required DOM elements:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Weather App</title>
</head>
<body>
  <div id="loading-indicator">Loading weather data...</div>
  <div id="error-message"></div>
  <div id="current-weather-section"></div>
  <div id="forecast-section"></div>
  <button id="refresh-button">Refresh</button>

  <script src="weather.js"></script>
</body>
</html>
```

The frontend expects the API to be available at the same origin (e.g., via a reverse proxy or by serving the HTML from the Go server).

## API Endpoints

### `GET /api/weather/current`

Returns the current weather conditions.

| Field         | Type   | Description                    |
|---------------|--------|--------------------------------|
| `temperature` | number | Temperature in Celsius         |
| `humidity`    | int    | Humidity percentage            |
| `wind_speed`  | number | Wind speed in km/h             |
| `condition`   | string | Weather condition description  |
| `city`        | string | City name                      |

### `GET /api/weather/forecast`

Returns a 7-day weather forecast.

| Field  | Type   | Description                          |
|--------|--------|--------------------------------------|
| `city` | string | City name                            |
| `days` | array  | Array of daily forecast objects      |

Each forecast day object:

| Field       | Type   | Description                   |
|-------------|--------|-------------------------------|
| `date`      | string | Date in `YYYY-MM-DD` format   |
| `temp_min`  | number | Minimum temperature (Celsius) |
| `temp_max`  | number | Maximum temperature (Celsius) |
| `condition` | string | Weather condition description |

## API Response Examples

### Current Weather

```bash
curl http://localhost:8080/api/weather/current
```

```json
{
  "temperature": 22.5,
  "humidity": 45,
  "wind_speed": 12.3,
  "condition": "Partly Cloudy",
  "city": "Beijing"
}
```

### 7-Day Forecast

```bash
curl http://localhost:8080/api/weather/forecast
```

```json
{
  "city": "Beijing",
  "days": [
    {
      "date": "2025-01-15",
      "temp_min": 15,
      "temp_max": 26,
      "condition": "Sunny"
    },
    {
      "date": "2025-01-16",
      "temp_min": 14,
      "temp_max": 24,
      "condition": "Partly Cloudy"
    },
    {
      "date": "2025-01-17",
      "temp_min": 16,
      "temp_max": 23,
      "condition": "Cloudy"
    }
  ]
}
```

### Error Response

```json
{
  "error": "not_found",
  "message": "endpoint not found"
}
```

## Project Structure

```
shared/
├── main.go                          # Application entrypoint and router setup
├── Makefile                         # Build, run, test, and clean targets
├── go.mod                           # Go module definition
├── go.sum                           # Dependency checksums
├── weather.js                       # Vanilla JS frontend client
├── internal/
│   ├── config/
│   │   └── config.go                # Environment-based configuration
│   ├── handlers/
│   │   └── weather_handler.go       # HTTP handlers for weather endpoints
│   ├── middleware/
│   │   └── cors.go                  # CORS middleware
│   ├── models/
│   │   └── weather.go               # Data models (request/response types)
│   └── services/
│       └── weather_service.go       # Weather data service (mock data)
└── openspec/
    └── changes/
        └── greeting-endpoint/       # Planned greeting API specification
            └── specs/
                └── api.yaml
```

## Development

Run tests:

```bash
make test
```

Build the binary:

```bash
make build
```

Clean build artifacts:

```bash
make clean
```

## Future Enhancements

- **Greeting endpoint** (`GET /api/hello`) — A personalized greeting API is planned via OpenSpec. It will accept an optional `name` query parameter and return a JSON message (e.g., `{"message": "Hello, Alice!"}`). See `openspec/changes/greeting-endpoint/specs/api.yaml` for the full specification.
- Integration with a real weather data provider
- Query parameters for city selection
- Unit conversion (Celsius/Fahrenheit)
- Response caching

## License

This project is available under the [MIT License](LICENSE).
