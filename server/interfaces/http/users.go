package http

import (
	"errors"
	"fmt"
	applicationidentity "gin_server/application/identity"
	"gin_server/domain/shared"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

func (h *Handler) GetUser(c *gin.Context) {
	userID := c.Query("user_id")
	if userID == "" {
		respondError(c, shared.ErrInvalidInput)
		return
	}
	user, err := h.identity.GetUser(c.Request.Context(), userID)
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, user)
}

func (h *Handler) UpdateProfile(c *gin.Context) {
	fileHeader, fileErr := c.FormFile("avatar")
	if fileErr != nil && !errors.Is(fileErr, http.ErrMissingFile) {
		respondBadRequest(c, fileErr)
		return
	}

	avatarURL := ""
	savedAvatarPath := ""
	if fileHeader != nil {
		var filename string
		var err error
		savedAvatarPath, filename, err = saveUserAvatar(fileHeader, h.avatarDir)
		if err != nil {
			respondBadRequest(c, err)
			return
		}

		scheme := "http"
		if c.Request.TLS != nil {
			scheme = "https"
		}
		avatarURL = fmt.Sprintf("%s://%s/assets/user_avatar/%s", scheme, c.Request.Host, filename)
	}

	result, err := h.identity.UpdateProfile(c.Request.Context(), applicationidentity.UpdateProfileCommand{
		UserID:          authenticatedUserID(c),
		Username:        c.PostForm("username"),
		LoginName:       c.PostForm("login_name"),
		CurrentPassword: c.PostForm("current_password"),
		NewPassword:     c.PostForm("new_password"),
		Avatar:          avatarURL,
	})
	if err != nil {
		if savedAvatarPath != "" {
			_ = os.Remove(savedAvatarPath)
		}
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, result)
}

func (h *Handler) ListConversations(c *gin.Context) {
	groups, err := h.conversation.ListForUser(c.Request.Context(), authenticatedUserID(c))
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, groups)
}

func (h *Handler) Search(c *gin.Context) {
	result, err := h.search.Search(c.Request.Context(), authenticatedUserID(c), c.Query("keyword"))
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, result)
}
