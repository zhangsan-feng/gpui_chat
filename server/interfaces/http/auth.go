package http

import (
	applicationidentity "gin_server/application/identity"
	"net/http"

	"github.com/gin-gonic/gin"
)

type loginRequest struct {
	LoginName string `json:"login_name" binding:"required"`
	Username  string `json:"username"`
	Password  string `json:"password"`
}

type refreshRequest struct {
	RefreshToken string `json:"refresh_token" binding:"required"`
}

func (h *Handler) Login(c *gin.Context) {
	var request loginRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	result, err := h.identity.Login(c.Request.Context(), applicationidentity.LoginCommand{
		LoginName: request.LoginName, Password: request.Password,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, result)
}

func (h *Handler) Register(c *gin.Context) {
	var request loginRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	result, err := h.identity.Register(c.Request.Context(), applicationidentity.RegisterCommand{
		Username: request.Username, LoginName: request.LoginName, Password: request.Password,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusCreated, result)
}

func (h *Handler) Refresh(c *gin.Context) {
	var request refreshRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	result, err := h.identity.Refresh(c.Request.Context(), applicationidentity.RefreshCommand{
		RefreshToken: request.RefreshToken,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, result)
}
