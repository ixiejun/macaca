# Go Hello World API

A simple Go HTTP server that returns "Hello, World!" in JSON format.

## Features

- Basic HTTP server using Go's standard `net/http` package
- JSON response formatting
- Error handling and logging
- Health check endpoint
- Production-ready foundation for future API development
- Configurable port via PORT environment variable

## Endpoints

- `GET /hello` - Returns `{"message": "Hello, World!"}`
- `GET /health` - Health check endpoint returns `{"message": "OK"}`

## Requirements

- Go 1.19 or higher

## Installation and Running

### Method 1: Direct execution
```bash
# Clone or download this repository
# Navigate to the project directory
cd go-hello-world-api

# Run the server (default port 8080)
go run main.go

# Or specify a different port
PORT=3000 go run main.go
```

### Method 2: Build and run
```bash
# Build the binary
go build -o hello-api main.go

# Run the binary (default port 8080)
./hello-api

# Or specify a different port
PORT=3000 ./hello-api
```

## Testing

Once the server is running, you can test it with curl:

```bash
# Test the main endpoint
curl http://localhost:8080/hello

# Expected response:
# {"message":"Hello, World!"}

# Test health check
curl http://localhost:8080/health

# Expected response:
# {"message":"OK"}
```

## Server Information

- Default Port: 8080 (configurable via PORT environment variable)
- Content-Type: application/json
- Logging: Console output with request information

## Extending the API

This serves as a foundation for building more complex APIs. You can:
- Add more route handlers
- Implement middleware for authentication, logging, etc.
- Add database connections
- Implement proper configuration management

## License

MIT License