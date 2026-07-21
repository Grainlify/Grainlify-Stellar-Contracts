package didit

import (
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

const mockPII = "Applicant: John Doe, Passport: AB1234567, DOB: 1990-01-15"

func TestAPIError_ErrorExcludesBody(t *testing.T) {
	err := &APIError{
		StatusCode: 400,
		Message:    "invalid document",
		Body:       mockPII,
	}

	errStr := err.Error()

	if strings.Contains(errStr, mockPII) {
		t.Fatalf("Error() must not contain raw response body (PII), got: %s", errStr)
	}
	if !strings.Contains(errStr, "400") {
		t.Fatalf("Error() must contain status code, got: %s", errStr)
	}
	if !strings.Contains(errStr, "invalid document") {
		t.Fatalf("Error() must contain parsed message, got: %s", errStr)
	}
}

func TestAPIError_BodyPreserved(t *testing.T) {
	err := &APIError{
		StatusCode: 403,
		Message:    "rejected",
		Body:       mockPII,
	}

	if err.Body != mockPII {
		t.Fatalf("APIError.Body must preserve raw response, got: %s", err.Body)
	}
}

func TestAPIError_StatusCodeAndMessagePreserved(t *testing.T) {
	err := &APIError{
		StatusCode: 502,
		Message:    "upstream error",
		Body:       "some body",
	}

	if err.StatusCode != 502 {
		t.Fatalf("expected StatusCode 502, got %d", err.StatusCode)
	}
	if err.Message != "upstream error" {
		t.Fatalf("expected Message 'upstream error', got %s", err.Message)
	}
}

func TestCreateSession_Success(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(SessionResponse{
			SessionID: "sess_abc123",
			Status:    "pending",
		})
	}))
	defer server.Close()

	c := &Client{
		httpClient: server.Client(),
		baseURL:    server.URL,
		apiKey:     "test-key",
	}

	session, err := c.CreateSession()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if session.SessionID != "sess_abc123" {
		t.Fatalf("expected session ID sess_abc123, got %s", session.SessionID)
	}
}

func TestCreateSession_APIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		w.Write([]byte(`{"error":"invalid document","details":{"applicant_name":"Jane Doe","passport":"XY9998887"}}`))
	}))
	defer server.Close()

	c := &Client{
		httpClient: server.Client(),
		baseURL:    server.URL,
		apiKey:     "test-key",
	}

	_, err := c.CreateSession()
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T: %v", err, err)
	}

	errStr := apiErr.Error()
	if strings.Contains(errStr, "Jane Doe") || strings.Contains(errStr, "XY9998887") {
		t.Fatalf("Error() must not contain PII from response body, got: %s", errStr)
	}
	if !strings.Contains(errStr, "400") {
		t.Fatalf("Error() must contain status code 400, got: %s", errStr)
	}
	if !strings.Contains(errStr, "invalid document") {
		t.Fatalf("Error() must contain message 'invalid document', got: %s", errStr)
	}
	if apiErr.Body == "" {
		t.Fatal("APIError.Body must preserve the raw response")
	}
}

func TestGetSessionDecisionOnce_APIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		w.Write([]byte(`{"error":"session not found","pii_dump":"SSN: 123-45-6789"}`))
	}))
	defer server.Close()

	c := &Client{
		httpClient: server.Client(),
		baseURL:    server.URL,
		apiKey:     "test-key",
	}

	_, err := c.getSessionDecisionOnce("sess_unknown")
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T: %v", err, err)
	}

	errStr := apiErr.Error()
	if strings.Contains(errStr, "123-45-6789") {
		t.Fatalf("Error() must not contain PII, got: %s", errStr)
	}
	if strings.Contains(errStr, "SSN") {
		t.Fatalf("Error() must not contain PII, got: %s", errStr)
	}
	if !strings.Contains(errStr, "session not found") {
		t.Fatalf("Error() must contain message 'session not found', got: %s", errStr)
	}
	if apiErr.Body == "" {
		t.Fatal("APIError.Body must preserve the raw response")
	}
}

func TestGetSessionDecisionOnce_Success(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(DecisionResponse{
			SessionID: "sess_abc123",
			Status:    "completed",
			Decision:  "approved",
		})
	}))
	defer server.Close()

	c := &Client{
		httpClient: server.Client(),
		baseURL:    server.URL,
		apiKey:     "test-key",
	}

	decision, err := c.getSessionDecisionOnce("sess_abc123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if decision.Decision != "approved" {
		t.Fatalf("expected decision 'approved', got %s", decision.Decision)
	}
}

func TestCreateSession_NonJSONError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte("raw HTML error page with user data: John Smith, SSN 999-00-1234"))
	}))
	defer server.Close()

	c := &Client{
		httpClient: server.Client(),
		baseURL:    server.URL,
		apiKey:     "test-key",
	}

	_, err := c.CreateSession()
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T: %v", err, err)
	}

	errStr := apiErr.Error()
	if strings.Contains(errStr, "John Smith") || strings.Contains(errStr, "999-00-1234") {
		t.Fatalf("Error() must not contain PII from non-JSON response, got: %s", errStr)
	}
	if !strings.Contains(errStr, "unexpected status code") {
		t.Fatalf("Error() must fall back to generic message for non-JSON, got: %s", errStr)
	}
	if apiErr.Body == "" {
		t.Fatal("APIError.Body must preserve the raw non-JSON response")
	}
}

func TestErrorsAs_WorksCorrectly(t *testing.T) {
	original := &APIError{
		StatusCode: 422,
		Message:    "validation failed",
		Body:       "PII: secret data",
	}

	wrapped := errors.New("outer: " + original.Error())

	var target *APIError
	if errors.As(wrapped, &target) {
		t.Fatal("errors.As should not match a plain wrapped error that is not *APIError")
	}

	direct := error(original)
	var target2 *APIError
	if !errors.As(direct, &target2) {
		t.Fatal("errors.As should match a direct *APIError")
	}
}
