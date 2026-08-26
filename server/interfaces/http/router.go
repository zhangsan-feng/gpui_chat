package http

import (
	applicationconversation "gin_server/application/conversation"
	applicationidentity "gin_server/application/identity"
	applicationnotification "gin_server/application/notification"
	"gin_server/application/search"
	"gin_server/application/social"
	ws "gin_server/interfaces/websocket"
	"log"

	"github.com/gin-gonic/gin"
)

func NewRouter(identity *applicationidentity.Service, conversation *applicationconversation.Service, notificationService *applicationnotification.Service, searchService *search.Service, socialService *social.Service, authenticator Authenticator, socketHandler *ws.Handler, staticDirectory string) *gin.Engine {
	router := gin.New()
	requestLogWriter := log.Writer()
	router.Use(requestLoggingMiddleware(), gin.LoggerWithWriter(requestLogWriter), gin.RecoveryWithWriter(requestLogWriter))
	router.Static("/assets", staticDirectory)

	handler := NewHandler(identity, conversation, notificationService, searchService, socialService, staticDirectory)
	api := router.Group("/api/v1")
	registerAuthRoutes(api, handler)
	authenticated := api.Group("")
	authenticated.Use(requireAuthentication(authenticator))
	registerUserRoutes(authenticated, handler)
	registerConversationRoutes(authenticated, handler)
	registerFriendRoutes(authenticated, handler)
	registerGroupRoutes(authenticated, handler)
	registerMessageRoutes(authenticated, handler)
	registerNotificationRoutes(authenticated, handler)

	router.GET("/ws", socketHandler.Connect)
	return router
}
