package http

import (
	"bytes"
	"encoding/json"
	"io"
	"log"
	"mime/multipart"
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"
)

const maxLoggedBodyBytes = 1024 * 1024

func requestLoggingMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		request := c.Request
		contentType := request.Header.Get("Content-Type")
		log.Printf(
			"[Request] %s %s | ClientIP: %s | Content-Type: %s | Content-Length: %d",
			request.Method,
			request.URL.Path,
			c.ClientIP(),
			contentType,
			request.ContentLength,
		)

		logQueryParameters(request)
		if request.Method == http.MethodPost || request.Method == http.MethodPut || request.Method == http.MethodPatch {
			logRequestBody(c, contentType)
		}

		c.Next()
	}
}

func logQueryParameters(request *http.Request) {
	query := request.URL.Query()
	if len(query) == 0 {
		return
	}

	log.Println("[Query Parameters]")
	for key, values := range query {
		for _, value := range values {
			log.Printf("  %s = %s", key, redactValue(key, value))
		}
	}
}

func logRequestBody(c *gin.Context, contentType string) {
	request := c.Request
	if strings.HasPrefix(contentType, "multipart/form-data") {
		logMultipartForm(request)
		return
	}

	if strings.HasPrefix(contentType, "application/x-www-form-urlencoded") {
		if err := request.ParseForm(); err != nil {
			log.Printf("Failed to parse form: %v", err)
			return
		}
		logFormValues("[Post Form]", request.PostForm)
		return
	}

	body, err := io.ReadAll(request.Body)
	if err != nil {
		log.Printf("Failed to read request body: %v", err)
		return
	}
	request.Body = io.NopCloser(bytes.NewReader(body))
	if len(body) == 0 {
		return
	}

	if strings.HasPrefix(contentType, "application/json") || contentType == "" {
		logJSONBody(body)
		return
	}

	log.Printf("[Body - %s]\n%s", contentType, truncateForLog(string(body)))
}

func logMultipartForm(request *http.Request) {
	if err := request.ParseMultipartForm(32 << 20); err != nil {
		log.Printf("Failed to parse multipart form: %v", err)
		return
	}

	if request.MultipartForm == nil {
		return
	}
	if len(request.MultipartForm.File) > 0 {
		log.Println("[Uploaded Files]")
		for fieldName, fileHeaders := range request.MultipartForm.File {
			for _, fileHeader := range fileHeaders {
				logMultipartFile(fieldName, fileHeader)
			}
		}
	}
	logFormValues("[Form Fields]", request.MultipartForm.Value)
}

func logMultipartFile(fieldName string, fileHeader *multipart.FileHeader) {
	log.Printf(
		"  Field: %s | Filename: %s | Size: %d bytes | Header: %v",
		fieldName,
		fileHeader.Filename,
		fileHeader.Size,
		fileHeader.Header,
	)
}

func logFormValues(title string, values map[string][]string) {
	if len(values) == 0 {
		return
	}

	log.Println(title)
	for key, items := range values {
		for _, value := range items {
			log.Printf("  %s = %s", key, redactValue(key, value))
		}
	}
}

func logJSONBody(body []byte) {
	var value any
	if err := json.Unmarshal(body, &value); err != nil {
		log.Printf("[Raw Body] <invalid json body omitted: %d bytes>", len(body))
		return
	}

	redacted := redactJSONValue(value)
	prettyJSON, err := json.MarshalIndent(redacted, "", "  ")
	if err != nil {
		log.Printf("[JSON Body] <failed to format body: %v>", err)
		return
	}
	log.Printf("[JSON Body]\n%s", truncateForLog(string(prettyJSON)))
}

func redactJSONValue(value any) any {
	switch typed := value.(type) {
	case map[string]any:
		redacted := make(map[string]any, len(typed))
		for key, item := range typed {
			if isSensitiveKey(key) {
				redacted[key] = "[REDACTED]"
				continue
			}
			redacted[key] = redactJSONValue(item)
		}
		return redacted
	case []any:
		redacted := make([]any, len(typed))
		for index, item := range typed {
			redacted[index] = redactJSONValue(item)
		}
		return redacted
	default:
		return value
	}
}

func redactValue(key string, value string) string {
	if isSensitiveKey(key) {
		return "[REDACTED]"
	}
	return value
}

func isSensitiveKey(key string) bool {
	switch strings.ToLower(strings.TrimSpace(key)) {
	case "authorization", "access_token", "refresh_token", "token", "user_token", "password", "password_confirmation":
		return true
	default:
		return false
	}
}

func truncateForLog(value string) string {
	if len(value) <= maxLoggedBodyBytes {
		return value
	}
	return value[:maxLoggedBodyBytes] + "... [truncated]"
}
