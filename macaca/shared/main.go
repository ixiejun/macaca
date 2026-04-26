package main

import (
	"log/slog"
	"net/http"
	"os"

	"github.com/go-chi/chi/v5"
	chiMiddleware "github.com/go-chi/chi/v5/middleware"

	"github.com/macaca/shared/internal/config"
	"github.com/macaca/shared/internal/handlers"
	"github.com/macaca/shared/internal/middleware"
	"github.com/macaca/shared/internal/services"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	cfg := config.Load()

	weatherService := services.NewWeatherService()
	weatherHandler := handlers.NewWeatherHandler(weatherService, logger)

	r := chi.NewRouter()
	r.Use(middleware.CORS)
	r.Use(chiMiddleware.Recoverer)
	r.Use(chiMiddleware.RequestID)

	r.Route("/api/weather", func(r chi.Router) {
		r.Get("/current", weatherHandler.GetCurrent)
		r.Get("/forecast", weatherHandler.GetForecast)
	})

	r.NotFound(func(w http.ResponseWriter, r *http.Request) {
		handlers.WriteError(w, http.StatusNotFound, "not_found", "endpoint not found")
	})

	logger.Info("server starting", "port", cfg.Port)
	if err := http.ListenAndServe(":"+cfg.Port, r); err != nil {
		logger.Error("server failed", "error", err)
		os.Exit(1)
	}
}
