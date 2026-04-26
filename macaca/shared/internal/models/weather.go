package models

type CurrentWeather struct {
	Temperature float64 `json:"temperature"`
	Humidity    int     `json:"humidity"`
	WindSpeed   float64 `json:"wind_speed"`
	Condition   string  `json:"condition"`
	City        string  `json:"city"`
}

type ForecastDay struct {
	Date      string  `json:"date"`
	TempMin   float64 `json:"temp_min"`
	TempMax   float64 `json:"temp_max"`
	Condition string  `json:"condition"`
}

type ForecastResponse struct {
	City string        `json:"city"`
	Days []ForecastDay `json:"days"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
}
