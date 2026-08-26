package http

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

func (h *Handler) ListNotifications(c *gin.Context) {
	values, err := h.notification.ListForUser(c.Request.Context(), authenticatedUserID(c))
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, values)
}
