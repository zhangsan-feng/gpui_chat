package http

import (
	"errors"
	"gin_server/domain/shared"
	"net/http"

	"github.com/gin-gonic/gin"
)

func respondOK(c *gin.Context, status int, data any) {
	c.JSON(status, gin.H{"data": data})
}

func respondError(c *gin.Context, err error) {
	status := http.StatusInternalServerError
	switch {
	case errors.Is(err, shared.ErrInvalidInput):
		status = http.StatusBadRequest
	case errors.Is(err, shared.ErrNotFound):
		status = http.StatusNotFound
	case errors.Is(err, shared.ErrConflict):
		status = http.StatusConflict
	case errors.Is(err, shared.ErrForbidden):
		status = http.StatusForbidden
	case errors.Is(err, shared.ErrUnauthorized):
		status = http.StatusUnauthorized
	}
	c.JSON(status, gin.H{"error": err.Error()})
}

func respondBadRequest(c *gin.Context, err error) {
	c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
}
