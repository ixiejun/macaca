---
name: golang
description: Provides concrete Go patterns for building backend services, including project structure, error handling, HTTP handlers, repositories, context usage, middleware, configuration, and testing.
---
# Go Backend Service Patterns

Comprehensive Go patterns and idioms for constructing dependable backend services.

## Project Structure

```
cmd/
  api/
    main.go           # Entry point
internal/
  handler/            # HTTP handlers
  service/            # Business logic
  repository/         # Data access
  middleware/         # HTTP middleware
  model/              # Domain types
  config/             # Configuration
pkg/
  httputil/           # Shared HTTP utilities
configs/
  config.yaml         # Default configuration
migrations/           # Database migrations
```

## HTTP Handlers (chi router)

```go
func NewRouter(svc *service.Service) *chi.Mux {
    r := chi.NewRouter()
    r.Use(middleware.RequestID)
    r.Use(middleware.Logger)
    r.Use(middleware.Recoverer)

    r.Route("/api/v1", func(r chi.Router) {
        r.Get("/items", listItems(svc))
        r.Post("/items", createItem(svc))
        r.Route("/{id}", func(r chi.Router) {
            r.Get("/", getItem(svc))
            r.Put("/", updateItem(svc))
            r.Delete("/", deleteItem(svc))
        })
    })
    return r
}

func listItems(svc *service.Service) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        items, err := svc.ListItems(r.Context())
        if err != nil {
            respondError(w, err)
            return
        }
        respondJSON(w, http.StatusOK, items)
    }
}
```

## Error Handling

```go
type AppError struct {
    Code    int    `json:"-"`
    Message string `json:"message"`
    Err     error  `json:"-"`
}

func (e *AppError) Error() string { return e.Message }
func (e *AppError) Unwrap() error { return e.Err }

func NewNotFound(msg string) *AppError {
    return &AppError{Code: http.StatusNotFound, Message: msg}
}

func respondError(w http.ResponseWriter, err error) {
    var appErr *AppError
    if errors.As(err, &appErr) {
        respondJSON(w, appErr.Code, appErr)
    } else {
        respondJSON(w, http.StatusInternalServerError,
            map[string]string{"message": "internal server error"})
    }
}
```

## Repository Pattern

```go
type ItemRepository interface {
    List(ctx context.Context) ([]model.Item, error)
    GetByID(ctx context.Context, id string) (*model.Item, error)
    Create(ctx context.Context, item *model.Item) error
    Update(ctx context.Context, item *model.Item) error
    Delete(ctx context.Context, id string) error
}

type pgItemRepo struct {
    pool *pgxpool.Pool
}

func NewItemRepository(pool *pgxpool.Pool) ItemRepository {
    return &pgItemRepo{pool: pool}
}
```

## Configuration

```go
type Config struct {
    Server   ServerConfig   `yaml:"server"`
    Database DatabaseConfig `yaml:"database"`
}

type ServerConfig struct {
    Port         int           `yaml:"port" env:"PORT" env-default:"8080"`
    ReadTimeout  time.Duration `yaml:"read_timeout" env-default:"5s"`
    WriteTimeout time.Duration `yaml:"write_timeout" env-default:"10s"`
}

type DatabaseConfig struct {
    URL             string `yaml:"url" env:"DATABASE_URL"`
    MaxOpenConns    int    `yaml:"max_open_conns" env-default:"25"`
    MaxIdleConns    int    `yaml:"max_idle_conns" env-default:"5"`
}
```

## Testing

```go
func TestListItems(t *testing.T) {
    repo := &mockRepo{items: []model.Item{{ID: "1", Name: "Test"}}}
    svc := service.New(repo)
    handler := listItems(svc)

    req := httptest.NewRequest(http.MethodGet, "/api/v1/items", nil)
    rec := httptest.NewRecorder()
    handler.ServeHTTP(rec, req)

    assert.Equal(t, http.StatusOK, rec.Code)
}
```

## Graceful Shutdown

```go
func main() {
    srv := &http.Server{Addr: ":8080", Handler: router}

    go func() {
        if err := srv.ListenAndServe(); err != http.ErrServerClosed {
            log.Fatal(err)
        }
    }()

    quit := make(chan os.Signal, 1)
    signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
    <-quit

    ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
    defer cancel()
    srv.Shutdown(ctx)
}
```

## Middleware

```go
func RequestIDMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        id := r.Header.Get("X-Request-ID")
        if id == "" {
            id = uuid.New().String()
        }
        ctx := context.WithValue(r.Context(), requestIDKey, id)
        w.Header().Set("X-Request-ID", id)
        next.ServeHTTP(w, r.WithContext(ctx))
    })
}
```
