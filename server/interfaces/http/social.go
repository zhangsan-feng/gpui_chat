package http

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

type friendRequestBody struct {
	FriendID string `json:"friend_id" binding:"required"`
}

type notificationDecisionBody struct {
	NotificationID string `json:"notification_id" binding:"required"`
	Approved       bool   `json:"approved"`
}

func (h *Handler) RequestFriend(c *gin.Context) {
	var request friendRequestBody
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	value, err := h.social.RequestFriend(c.Request.Context(), authenticatedUserID(c), request.FriendID)
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusCreated, value)
}

func (h *Handler) RemoveFriend(c *gin.Context) {
	var request friendRequestBody
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	if err := h.social.RemoveFriend(c.Request.Context(), authenticatedUserID(c), request.FriendID); err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, nil)
}

func (h *Handler) ReviewFriendRequest(c *gin.Context) {
	var request notificationDecisionBody
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	value, err := h.social.ReviewFriendRequest(c.Request.Context(), request.NotificationID, authenticatedUserID(c), request.Approved)
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, value)
}
