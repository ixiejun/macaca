package handlers

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/macaca/shared/internal/models"
	"github.com/macaca/shared/internal/services"
)

type WeatherHandler struct {
	service *services.WeatherService
	logger  *slog.Logger
}

func NewWeatherHandler(service *services.WeatherService, logger *slog.Logger) *WeatherHandler {
	return &WeatherHandler{service: service, logger: logger}
}

func (h *WeatherHandler) GetCurrent(w http.ResponseWriter, r *http.Request) {
	h.logger.Info("handling current weather request", "method", r.Method, "path", r.URL.Path)

	weather := h.service.GetCurrentWeather()
	writeJSON(w, http.StatusOK, weather)
}

func (h *WeatherHandler) GetForecast(w http.ResponseWriter, r *http.Request) {
	h.logger.Info("handling forecast request", "method", r.Method, "path", r.URL.Path)

	forecast := h.service.GetForecast()
	writeJSON(w, http.StatusOK, forecast)
}

func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func WriteError(w http.ResponseWriter, status int, errType, message string) {
	writeJSON(w, status, models.ErrorResponse{
		Error:   errType,
		Message: message,
	})
}
