package realtime

import (
	"gin_server/infrastructure/safety"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

const writeWait = 5 * time.Second

type client struct {
	conn        *websocket.Conn
	events      <-chan []byte
	unsubscribe func()
}

type Hub struct {
	pubSub  *PubSub
	mu      sync.RWMutex
	clients map[string]*client
}

func NewHub(pubSub *PubSub) *Hub {
	return &Hub{pubSub: pubSub, clients: make(map[string]*client)}
}

func (h *Hub) Register(userID string, conn *websocket.Conn) {
	events, unsubscribe := h.pubSub.Subscribe(userTopic(userID))
	newClient := &client{conn: conn, events: events, unsubscribe: unsubscribe}
	h.mu.Lock()
	previous := h.clients[userID]
	h.clients[userID] = newClient
	h.mu.Unlock()
	if previous != nil {
		previous.unsubscribe()
	}
	safety.Go("websocket-write-loop", func() {
		h.writeLoop(userID, newClient)
	})
}

func (h *Hub) Unregister(userID string, conn *websocket.Conn) {
	h.mu.Lock()
	client := h.clients[userID]
	if client == nil || client.conn != conn {
		h.mu.Unlock()
		return
	}
	delete(h.clients, userID)
	h.mu.Unlock()
	client.unsubscribe()
}

func (h *Hub) writeLoop(userID string, client *client) {
	defer func() {
		h.Unregister(userID, client.conn)
		_ = client.conn.Close()
	}()
	for payload := range client.events {
		_ = client.conn.SetWriteDeadline(time.Now().Add(writeWait))
		if err := client.conn.WriteMessage(websocket.TextMessage, payload); err != nil {
			return
		}
	}
}
