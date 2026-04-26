package main

import (
	"encoding/json"
	"log"
	"net/http"
)

type HelloResponse struct {
	Message string `json:"message"`
}

func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}

		next.ServeHTTP(w, r)
	})
}

func helloHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)

	if err := json.NewEncoder(w).Encode(HelloResponse{Message: "Hello World"}); err != nil {
		log.Printf("error encoding response: %v", err)
	}
}

func main() {
	mux := http.NewServeMux()
	mux.HandleFunc("/api/hello", helloHandler)

	log.Printf("server listening on :8080")
	log.Fatal(http.ListenAndServe(":8080", corsMiddleware(mux)))
}
