package websocket

import (
	"net/http"
	"net/url"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/gorilla/websocket"
)

type Authenticator interface {
	Authenticate(rawToken string) (string, error)
}

type Handler struct {
	authenticator Authenticator
	hub           SessionRegistry
	upgrader      websocket.Upgrader
}

type SessionRegistry interface {
	Register(userID string, conn *websocket.Conn)
	Unregister(userID string, conn *websocket.Conn)
}

func NewHandler(authenticator Authenticator, hub SessionRegistry) *Handler {
	return &Handler{
		authenticator: authenticator,
		hub:           hub,
		upgrader:      websocket.Upgrader{CheckOrigin: sameOrigin},
	}
}

func (h *Handler) Connect(c *gin.Context) {
	userID, err := h.authenticator.Authenticate(websocketToken(c.Request))
	if err != nil {
		c.AbortWithStatus(http.StatusUnauthorized)
		return
	}
	connection, err := h.upgrader.Upgrade(c.Writer, c.Request, nil)
	if err != nil {
		return
	}
	defer func() { _ = connection.Close() }()
	h.hub.Register(userID, connection)
	defer h.hub.Unregister(userID, connection)

	for {
		if _, _, err := connection.ReadMessage(); err != nil {
			return
		}
	}
}

func websocketToken(request *http.Request) string {
	parts := strings.Fields(request.Header.Get("Authorization"))
	if len(parts) == 2 && strings.EqualFold(parts[0], "Bearer") {
		return parts[1]
	}
	return ""
}

func sameOrigin(request *http.Request) bool {
	origin := strings.TrimSpace(request.Header.Get("Origin"))
	if origin == "" {
		return true
	}
	parsed, err := url.Parse(origin)
	if err != nil {
		return false
	}
	return strings.EqualFold(parsed.Host, request.Host)
}
