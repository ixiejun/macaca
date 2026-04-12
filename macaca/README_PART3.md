## Running the Application

### Starting the Backend

```bash
# From project root
cargo run --release --bin macaca-web

# Or use the binary directly
./target/release/macaca-web

# The API server starts on port 3001 by default
```

You should see:
```
Macaca OS API server: http://localhost:3001
```

### Starting the Frontend

In a new terminal:

```bash
cd frontend

# Development mode (with hot reload)
npm run dev

# Or production mode
npm run build
npm start
```

The frontend runs on http://localhost:3000

### Accessing the Application

1. Open your browser to http://localhost:3000
2. You'll see the Macaca dashboard with:
   - List of discovered applications
   - Agent status panels
   - Chat interface
   - Task board view

### Running with Docker (Optional)

```bash
# Build and run backend
docker build -t macaca-backend .
docker run -p 3001:3001 \
  -e OPENAI_API_KEY="sk-..." \
  -v $(pwd)/data:/app/data \
  macaca-backend

# Frontend
cd frontend
docker build -t macaca-frontend .
docker run -p 3000:3000 macaca-frontend
```

