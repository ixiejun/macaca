(function () {
  'use strict';

  var MAX_RETRIES = 3;
  var BASE_DELAY_MS = 1000;

  var elements = {};

  function cacheElements() {
    elements = {
      currentWeather: document.getElementById('current-weather-section'),
      forecast: document.getElementById('forecast-section'),
      loading: document.getElementById('loading-indicator'),
      error: document.getElementById('error-message'),
      refresh: document.getElementById('refresh-button'),
    };
  }

  function showLoading() {
    if (elements.loading) {
      elements.loading.style.display = 'flex';
      elements.loading.textContent = 'Loading weather data...';
    }
    if (elements.error) {
      elements.error.style.display = 'none';
      elements.error.textContent = '';
    }
  }

  function hideLoading() {
    if (elements.loading) {
      elements.loading.style.display = 'none';
    }
  }

  function showError(message) {
    hideLoading();
    if (elements.error) {
      elements.error.style.display = 'block';
      elements.error.textContent = message;
    }
  }

  function formatDate(dateString) {
    var date = new Date(dateString);
    if (isNaN(date.getTime())) {
      return dateString;
    }
    return date.toLocaleDateString(undefined, {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
    });
  }

  function formatTemp(value, unit) {
    var suffix = unit === 'fahrenheit' ? '\u00B0F' : '\u00B0C';
    return Math.round(value) + suffix;
  }

  function fetchWithRetry(url, attempt) {
    if (attempt === undefined) attempt = 0;

    return fetch(url).then(function (response) {
      if (!response.ok) {
        var msg = 'Server error: ' + response.status + ' ' + response.statusText;
        if (attempt < MAX_RETRIES) {
          var delay = BASE_DELAY_MS * Math.pow(2, attempt);
          return new Promise(function (resolve) {
            setTimeout(resolve, delay);
          }).then(function () {
            return fetchWithRetry(url, attempt + 1);
          });
        }
        throw new Error(msg);
      }
      return response.json();
    }).catch(function (err) {
      if (err.name === 'TypeError') {
        if (attempt < MAX_RETRIES) {
          var delay = BASE_DELAY_MS * Math.pow(2, attempt);
          return new Promise(function (resolve) {
            setTimeout(resolve, delay);
          }).then(function () {
            return fetchWithRetry(url, attempt + 1);
          });
        }
        throw new Error('Network error: unable to reach the server. Check your connection.');
      }
      throw err;
    });
  }

  function renderCurrentWeather(data) {
    if (!elements.currentWeather) return;

    var unit = data.unit || 'celsius';
    var html =
      '<div class="weather-current">' +
        '<h2>' + escapeHtml(data.location || 'Unknown') + '</h2>' +
        '<div class="weather-temp">' + formatTemp(data.temperature, unit) + '</div>' +
        '<div class="weather-desc">' + escapeHtml(data.description || '') + '</div>' +
        '<div class="weather-details">' +
          '<span>Humidity: ' + (data.humidity != null ? data.humidity + '%' : '--') + '</span>' +
          '<span>Wind: ' + (data.wind_speed != null ? data.wind_speed + ' km/h' : '--') + '</span>' +
        '</div>' +
      '</div>';

    elements.currentWeather.innerHTML = html;
  }

  function renderForecast(data) {
    if (!elements.forecast) return;

    var days = Array.isArray(data) ? data : data.days || data.forecast || [];
    if (days.length === 0) {
      elements.forecast.innerHTML = '<p>No forecast data available.</p>';
      return;
    }

    var unit = days[0].unit || data.unit || 'celsius';
    var cards = days.map(function (day) {
      return (
        '<div class="forecast-card">' +
          '<div class="forecast-date">' + formatDate(day.date) + '</div>' +
          '<div class="forecast-temps">' +
            '<span class="temp-high">' + formatTemp(day.high, unit) + '</span>' +
            '<span class="temp-low">' + formatTemp(day.low, unit) + '</span>' +
          '</div>' +
          '<div class="forecast-desc">' + escapeHtml(day.description || '') + '</div>' +
        '</div>'
      );
    });

    elements.forecast.innerHTML = '<div class="forecast-grid">' + cards.join('') + '</div>';
  }

  function escapeHtml(str) {
    var div = document.createElement('div');
    div.appendChild(document.createTextNode(str));
    return div.innerHTML;
  }

  function loadWeather() {
    showLoading();

    var currentPromise = fetchWithRetry('/api/weather/current');
    var forecastPromise = fetchWithRetry('/api/weather/forecast');

    Promise.all([currentPromise, forecastPromise])
      .then(function (results) {
        hideLoading();
        renderCurrentWeather(results[0]);
        renderForecast(results[1]);
      })
      .catch(function (err) {
        showError('Failed to load weather data: ' + err.message);
      });
  }

  function init() {
    cacheElements();

    if (elements.refresh) {
      elements.refresh.addEventListener('click', function () {
        loadWeather();
      });
    }

    loadWeather();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
