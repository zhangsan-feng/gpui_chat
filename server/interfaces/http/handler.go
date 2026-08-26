package http

import (
	applicationconversation "gin_server/application/conversation"
	applicationidentity "gin_server/application/identity"
	applicationnotification "gin_server/application/notification"
	"gin_server/application/search"
	"gin_server/application/social"
	"path/filepath"
)

type Handler struct {
	identity       *applicationidentity.Service
	conversation   *applicationconversation.Service
	notification   *applicationnotification.Service
	search         *search.Service
	social         *social.Service
	avatarDir      string
	groupAvatarDir string
	messageFileDir string
}

func NewHandler(identity *applicationidentity.Service, conversation *applicationconversation.Service, notificationService *applicationnotification.Service, searchService *search.Service, socialService *social.Service, staticDirectory string) *Handler {
	return &Handler{
		identity: identity, conversation: conversation, notification: notificationService, search: searchService, social: socialService,
		avatarDir: filepath.Join(staticDirectory, "user_avatar"), groupAvatarDir: filepath.Join(staticDirectory, "group_avatar"),
		messageFileDir: filepath.Join(staticDirectory, "message_file"),
	}
}
