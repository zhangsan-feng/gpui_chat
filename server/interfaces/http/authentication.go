package http

import (
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"
)

const authenticatedUserIDKey = "authenticated_user_id"

type Authenticator interface {
	Authenticate(rawToken string) (string, error)
}

func requireAuthentication(authenticator Authenticator) gin.HandlerFunc {
	return func(context *gin.Context) {
		authorization := strings.TrimSpace(context.GetHeader("Authorization"))
		parts := strings.Fields(authorization)
		if len(parts) != 2 || !strings.EqualFold(parts[0], "Bearer") {
			context.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "authentication required"})
			return
		}
		userID, err := authenticator.Authenticate(parts[1])
		if err != nil {
			context.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "invalid token"})
			return
		}
		context.Set(authenticatedUserIDKey, userID)
		context.Next()
	}
}

func authenticatedUserID(context *gin.Context) string {
	userID, _ := context.Get(authenticatedUserIDKey)
	value, _ := userID.(string)
	return value
}
