package http

import "github.com/gin-gonic/gin"

func registerAuthRoutes(api *gin.RouterGroup, handler *Handler) {
	api.POST("/auth/login", handler.Login)
	api.POST("/auth/register", handler.Register)
	api.POST("/auth/refresh", handler.Refresh)
}

func registerUserRoutes(api *gin.RouterGroup, handler *Handler) {
	api.GET("/user/get", handler.GetUser)
	api.POST("/user/profile", handler.UpdateProfile)
	api.GET("/search", handler.Search)
}

func registerConversationRoutes(api *gin.RouterGroup, handler *Handler) {
	api.GET("/conversation/list", handler.ListConversations)
}

func registerFriendRoutes(api *gin.RouterGroup, handler *Handler) {
	api.POST("/friend/request", handler.RequestFriend)
	api.POST("/friend/remove", handler.RemoveFriend)
	api.POST("/friend/review-request", handler.ReviewFriendRequest)
}

func registerGroupRoutes(api *gin.RouterGroup, handler *Handler) {
	api.POST("/group/create", handler.CreateGroup)
	api.POST("/group/update", handler.UpdateGroup)
	api.POST("/group/request-join", handler.RequestJoinGroup)
	api.POST("/group/review-join-request", handler.ReviewJoinRequest)
	api.POST("/group/leave", handler.LeaveGroup)
	api.POST("/group/disband", handler.DisbandGroup)
	api.POST("/group/member-role", handler.UpdateGroupMemberRole)
}

func registerMessageRoutes(api *gin.RouterGroup, handler *Handler) {
	api.POST("/message/send", handler.SendMessage)
}

func registerNotificationRoutes(api *gin.RouterGroup, handler *Handler) {
	api.GET("/notification/list", handler.ListNotifications)
}
