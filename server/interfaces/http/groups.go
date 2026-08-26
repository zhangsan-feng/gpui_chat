package http

import (
	"errors"
	"fmt"
	applicationconversation "gin_server/application/conversation"
	domainconversation "gin_server/domain/conversation"
	"gin_server/domain/shared"
	"mime/multipart"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

type createGroupRequest struct {
	Name   string `json:"name" binding:"required"`
	Avatar string `json:"avatar"`
}

type updateGroupRequest struct {
	GroupID   string `json:"group_id" form:"group_id" binding:"required"`
	Name      string `json:"name" form:"name" binding:"required"`
	Avatar    string `json:"avatar" form:"avatar"`
	AllowJoin bool   `json:"allow_join" form:"allow_join"`
}

type sendMessageRequest struct {
	GroupID     string                          `json:"group_id" binding:"required"`
	Content     string                          `json:"content"`
	Attachments []domainconversation.Attachment `json:"attachments"`
}

const maxMessageFileSize int64 = 20 * 1024 * 1024
const maxMessageFileCount = 4

type joinGroupRequest struct {
	GroupID string `json:"group_id" binding:"required"`
}

type updateGroupMemberRoleRequest struct {
	GroupID string `json:"group_id" binding:"required"`
	UserID  string `json:"user_id" binding:"required"`
	Role    string `json:"role" binding:"required"`
}

func (h *Handler) CreateGroup(c *gin.Context) {
	var request createGroupRequest
	savedAvatarPath := ""

	if strings.HasPrefix(c.ContentType(), "multipart/") {
		request.Name = c.PostForm("name")
		fileHeader, fileErr := c.FormFile("avatar")
		if fileErr != nil && !errors.Is(fileErr, http.ErrMissingFile) {
			respondBadRequest(c, fileErr)
			return
		}
		if fileHeader != nil {
			if fileHeader.Size > 5*1024*1024 {
				respondError(c, shared.ErrInvalidInput)
				return
			}

			extension := strings.ToLower(filepath.Ext(fileHeader.Filename))
			if !supportedAvatarExtension(extension) {
				respondError(c, shared.ErrInvalidInput)
				return
			}
			if err := os.MkdirAll(h.groupAvatarDir, 0o755); err != nil {
				respondError(c, err)
				return
			}

			filename := uuid.NewString() + extension
			savedAvatarPath = filepath.Join(h.groupAvatarDir, filename)
			if err := c.SaveUploadedFile(fileHeader, savedAvatarPath); err != nil {
				respondError(c, err)
				return
			}

			scheme := "http"
			if c.Request.TLS != nil {
				scheme = "https"
			}
			request.Avatar = fmt.Sprintf("%s://%s/assets/group_avatar/%s", scheme, c.Request.Host, filename)
		}
	} else if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	group, err := h.conversation.CreateGroup(c.Request.Context(), applicationconversation.CreateGroupCommand{
		Name: request.Name, Avatar: request.Avatar, OwnerID: authenticatedUserID(c),
	})
	if err != nil {
		if savedAvatarPath != "" {
			_ = os.Remove(savedAvatarPath)
		}
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusCreated, group)
}

func (h *Handler) UpdateGroup(c *gin.Context) {
	var request updateGroupRequest
	savedAvatarPath := ""

	if strings.HasPrefix(c.ContentType(), "multipart/") {
		request.GroupID = c.PostForm("group_id")
		request.Name = c.PostForm("name")
		allowJoinValue := strings.TrimSpace(c.PostForm("allow_join"))
		if allowJoinValue == "" {
			request.AllowJoin = true
		} else {
			allowJoin, err := strconv.ParseBool(allowJoinValue)
			if err != nil {
				respondBadRequest(c, err)
				return
			}
			request.AllowJoin = allowJoin
		}

		fileHeader, fileErr := c.FormFile("avatar")
		if fileErr != nil && !errors.Is(fileErr, http.ErrMissingFile) {
			respondBadRequest(c, fileErr)
			return
		}
		if fileHeader != nil {
			if fileHeader.Size > 5*1024*1024 {
				respondError(c, shared.ErrInvalidInput)
				return
			}
			extension := strings.ToLower(filepath.Ext(fileHeader.Filename))
			if !supportedAvatarExtension(extension) {
				respondError(c, shared.ErrInvalidInput)
				return
			}
			if err := os.MkdirAll(h.groupAvatarDir, 0o755); err != nil {
				respondError(c, err)
				return
			}

			filename := uuid.NewString() + extension
			savedAvatarPath = filepath.Join(h.groupAvatarDir, filename)
			if err := c.SaveUploadedFile(fileHeader, savedAvatarPath); err != nil {
				respondError(c, err)
				return
			}

			scheme := "http"
			if c.Request.TLS != nil {
				scheme = "https"
			}
			request.Avatar = fmt.Sprintf("%s://%s/assets/group_avatar/%s", scheme, c.Request.Host, filename)
		}
	} else if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}

	group, err := h.conversation.UpdateGroup(c.Request.Context(), applicationconversation.UpdateGroupCommand{
		GroupID: request.GroupID, OperatorID: authenticatedUserID(c), Name: request.Name,
		Avatar: request.Avatar, AllowJoin: request.AllowJoin,
	})
	if err != nil {
		if savedAvatarPath != "" {
			_ = os.Remove(savedAvatarPath)
		}
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, group)
}

func (h *Handler) RequestJoinGroup(c *gin.Context) {
	var request joinGroupRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	value, err := h.conversation.RequestJoinGroup(c.Request.Context(), request.GroupID, authenticatedUserID(c))
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusCreated, value)
}

func (h *Handler) ReviewJoinRequest(c *gin.Context) {
	var request notificationDecisionBody
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	value, err := h.conversation.ReviewJoinRequest(c.Request.Context(), request.NotificationID, authenticatedUserID(c), request.Approved)
	if err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, value)
}

func (h *Handler) LeaveGroup(c *gin.Context) {
	var request joinGroupRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	if err := h.conversation.LeaveGroup(c.Request.Context(), request.GroupID, authenticatedUserID(c)); err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, nil)
}

func (h *Handler) DisbandGroup(c *gin.Context) {
	var request joinGroupRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	if err := h.conversation.DisbandGroup(c.Request.Context(), request.GroupID, authenticatedUserID(c)); err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, nil)
}

func (h *Handler) UpdateGroupMemberRole(c *gin.Context) {
	var request updateGroupMemberRoleRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	if err := h.conversation.UpdateMemberRole(
		c.Request.Context(),
		request.GroupID,
		authenticatedUserID(c),
		request.UserID,
		request.Role,
	); err != nil {
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusOK, nil)
}

func (h *Handler) SendMessage(c *gin.Context) {
	var request sendMessageRequest
	savedFiles := make([]string, 0)

	if strings.HasPrefix(c.ContentType(), "multipart/") {
		request.GroupID = c.PostForm("group_id")
		request.Content = c.PostForm("content")
		attachments, paths, err := h.saveMessageFiles(c)
		if err != nil {
			removeFiles(paths)
			respondError(c, err)
			return
		}
		request.Attachments = attachments
		savedFiles = paths
	} else if err := c.ShouldBindJSON(&request); err != nil {
		respondBadRequest(c, err)
		return
	}
	message, err := h.conversation.SendMessage(c.Request.Context(), request.GroupID, applicationconversation.SendMessageCommand{
		SenderID: authenticatedUserID(c), Content: request.Content, Attachments: request.Attachments,
	})
	if err != nil {
		removeFiles(savedFiles)
		respondError(c, err)
		return
	}
	respondOK(c, http.StatusCreated, message)
}

func (h *Handler) saveMessageFiles(c *gin.Context) ([]domainconversation.Attachment, []string, error) {
	form, err := c.MultipartForm()
	if err != nil {
		return nil, nil, err
	}

	fileHeaders := append([]*multipart.FileHeader{}, form.File["files"]...)
	fileHeaders = append(fileHeaders, form.File["file"]...)
	if len(fileHeaders) == 0 {
		return []domainconversation.Attachment{}, []string{}, nil
	}
	if len(fileHeaders) > maxMessageFileCount {
		return nil, nil, shared.ErrInvalidInput
	}
	if err := os.MkdirAll(h.messageFileDir, 0o755); err != nil {
		return nil, nil, err
	}

	attachments := make([]domainconversation.Attachment, 0, len(fileHeaders))
	savedPaths := make([]string, 0, len(fileHeaders))
	scheme := "http"
	if c.Request.TLS != nil {
		scheme = "https"
	}
	for _, fileHeader := range fileHeaders {
		if fileHeader.Size > maxMessageFileSize {
			return nil, savedPaths, shared.ErrInvalidInput
		}

		extension := strings.ToLower(filepath.Ext(fileHeader.Filename))
		filename := uuid.NewString() + extension
		savedPath := filepath.Join(h.messageFileDir, filename)
		if err := c.SaveUploadedFile(fileHeader, savedPath); err != nil {
			return nil, savedPaths, err
		}

		savedPaths = append(savedPaths, savedPath)
		attachments = append(attachments, domainconversation.Attachment{
			Name: fileHeader.Filename,
			URL:  fmt.Sprintf("%s://%s/assets/message_file/%s", scheme, c.Request.Host, filename),
		})
	}
	return attachments, savedPaths, nil
}

func removeFiles(paths []string) {
	for _, path := range paths {
		_ = os.Remove(path)
	}
}
