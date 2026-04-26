package services

import (
	"fmt"
	"time"

	"github.com/macaca/shared/internal/models"
)

type WeatherService struct{}

func NewWeatherService() *WeatherService {
	return &WeatherService{}
}

func (s *WeatherService) GetCurrentWeather() models.CurrentWeather {
	return models.CurrentWeather{
		Temperature: 22.5,
		Humidity:    45,
		WindSpeed:   12.3,
		Condition:   "Partly Cloudy",
		City:        "Beijing",
	}
}

func (s *WeatherService) GetForecast() models.ForecastResponse {
	today := time.Now()
	days := make([]models.ForecastDay, 7)

	conditions := []string{
		"Sunny", "Partly Cloudy", "Cloudy",
		"Light Rain", "Sunny", "Windy", "Clear",
	}
	mins := []float64{15, 14, 16, 13, 17, 15, 16}
	maxs := []float64{26, 24, 23, 20, 28, 25, 27}

	for i := range 7 {
		date := today.AddDate(0, 0, i)
		days[i] = models.ForecastDay{
			Date:      fmt.Sprintf("%d-%02d-%02d", date.Year(), date.Month(), date.Day()),
			TempMin:   mins[i],
			TempMax:   maxs[i],
			Condition: conditions[i],
		}
	}

	return models.ForecastResponse{
		City: "Beijing",
		Days: days,
	}
}
