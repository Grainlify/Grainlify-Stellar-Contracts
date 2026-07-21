package didit

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

const defaultBaseURL = "https://api.didit.me"

// APIError represents a structured error returned by the Didit API.
// The raw response body is preserved in Body for explicit access
// but is excluded from Error() to prevent PII leakage in logs.
type APIError struct {
	StatusCode int
	Message    string
	Body       string
}

func (e *APIError) Error() string {
	return fmt.Sprintf("didit API error: status %d, error: %s", e.StatusCode, e.Message)
}

type Client struct {
	httpClient *http.Client
	baseURL    string
	apiKey     string
}

type SessionResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
}

type DecisionResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
	Decision  string `json:"decision"`
}

func NewClient(apiKey string) *Client {
	return &Client{
		httpClient: &http.Client{Timeout: 30 *time.Second},
		baseURL:    defaultBaseURL,
		apiKey:     apiKey,
	}
}

func (c *Client) CreateSession() (*SessionResponse, error) {
	url := c.baseURL + "/v1/sessions"

	req, err := http.NewRequest(http.MethodPost, url, nil)
	if err != nil {
		return nil, fmt.Errorf("creating request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+c.apiKey)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("sending request: %w", err)
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response: %w", err)
	}

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusCreated {
		apiErr := &APIError{
			StatusCode: resp.StatusCode,
			Body:       string(bodyBytes),
		}

		var errBody struct {
			Error string `json:"error"`
		}
		if json.Unmarshal(bodyBytes, &errBody) == nil && errBody.Error != "" {
			apiErr.Message = errBody.Error
		} else {
			apiErr.Message = "unexpected status code"
		}

		return nil, apiErr
	}

	var session SessionResponse
	if err := json.Unmarshal(bodyBytes, &session); err != nil {
		return nil, fmt.Errorf("decoding response: %w", err)
	}

	return &session, nil
}

func (c *Client) GetSessionDecision(sessionID string) (*DecisionResponse, error) {
	return c.getSessionDecisionOnce(sessionID)
}

func (c *Client) getSessionDecisionOnce(sessionID string) (*DecisionResponse, error) {
	url := fmt.Sprintf("%s/v1/sessions/%s/decision", c.baseURL, sessionID)

	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("creating request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+c.apiKey)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("sending request: %w", err)
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		apiErr := &APIError{
			StatusCode: resp.StatusCode,
			Body:       string(bodyBytes),
		}

		var errBody struct {
			Error string `json:"error"`
		}
		if json.Unmarshal(bodyBytes, &errBody) == nil && errBody.Error != "" {
			apiErr.Message = errBody.Error
		} else {
			apiErr.Message = "unexpected status code"
		}

		return nil, apiErr
	}

	var decision DecisionResponse
	if err := json.Unmarshal(bodyBytes, &decision); err != nil {
		return nil, fmt.Errorf("decoding response: %w", err)
	}

	return &decision, nil
}
