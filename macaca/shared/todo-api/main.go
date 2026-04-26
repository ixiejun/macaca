package main

import (
	"encoding/json"
	"net/http"
	"sync"
)

type Todo struct {
	ID        int    `json:"id"`
	Title     string `json:"title"`
	Completed bool   `json:"completed"`
}

var (
	todos   []Todo
	mu      sync.Mutex
	nextID  int = 1
)

func todosHandler(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		handleGetTodos(w, r)
	case http.MethodPost:
		handleCreateTodo(w, r)
	default:
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
	}
}

func handleGetTodos(w http.ResponseWriter, r *http.Request) {
	mu.Lock()
	result := make([]Todo, len(todos))
	copy(result, todos)
	mu.Unlock()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(result)
}

func handleCreateTodo(w http.ResponseWriter, r *http.Request) {
	var input struct {
		Title string `json:"title"`
	}
	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		http.Error(w, "invalid request body", http.StatusBadRequest)
		return
	}
	if input.Title == "" {
		http.Error(w, "title is required", http.StatusBadRequest)
		return
	}

	mu.Lock()
	todo := Todo{
		ID:        nextID,
		Title:     input.Title,
		Completed: false,
	}
	nextID++
	todos = append(todos, todo)
	mu.Unlock()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(todo)
}

func main() {
	todos = make([]Todo, 0)
	http.HandleFunc("/todos", todosHandler)
	http.ListenAndServe(":8080", nil)
}
