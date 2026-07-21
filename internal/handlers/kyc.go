package handlers

import (
	"errors"
	"log"
	"net/http"

	"grainlify-stellar-contracts/internal/didit"
)

type KYCHandler struct {
	client *didit.Client
}

func NewKYCHandler(client *didit.Client) *KYCHandler {
	return &KYCHandler{client: client}
}

func (h *KYCHandler) CreateSession(w http.ResponseWriter, r *http.Request) {
	session, err := h.client.CreateSession()
	if err != nil {
		var apiErr *didit.APIError
		if errors.As(err, &apiErr) {
			log.Printf("Didit API error: status=%d message=%s", apiErr.StatusCode, apiErr.Message)
			http.Error(w, "KYC session creation failed", apiErr.StatusCode)
			return
		}
		log.Printf("Internal error creating KYC session: %v", err)
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusCreated)
	w.Write([]byte(`{"session_id":"` + session.SessionID + `"}`))
}

func (h *KYCHandler) GetDecision(w http.ResponseWriter, r *http.Request) {
	sessionID := r.URL.Query().Get("session_id")
	if sessionID == "" {
		http.Error(w, "session_id required", http.StatusBadRequest)
		return
	}

	decision, err := h.client.GetSessionDecision(sessionID)
	if err != nil {
		var apiErr *didit.APIError
		if errors.As(err, &apiErr) {
			log.Printf("Didit API error: status=%d message=%s", apiErr.StatusCode, apiErr.Message)
			http.Error(w, "failed to get KYC decision", apiErr.StatusCode)
			return
		}
		log.Printf("Internal error getting KYC decision: %v", err)
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
	w.Write([]byte(`{"status":"` + decision.Status + `","decision":"` + decision.Decision + `"}`))
}
