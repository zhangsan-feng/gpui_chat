package realtime

import (
	"context"
	"sync"
)

const (
	userTopicPrefix      = "user:"
	subscriberBufferSize = 128
)

type PubSub struct {
	mu          sync.RWMutex
	nextID      uint64
	subscribers map[string]map[uint64]chan []byte
}

func NewPubSub() *PubSub {
	return &PubSub{subscribers: make(map[string]map[uint64]chan []byte)}
}

func (p *PubSub) Subscribe(topic string) (<-chan []byte, func()) {
	channel := make(chan []byte, subscriberBufferSize)

	p.mu.Lock()
	p.nextID++
	subscriberID := p.nextID
	if p.subscribers[topic] == nil {
		p.subscribers[topic] = make(map[uint64]chan []byte)
	}
	p.subscribers[topic][subscriberID] = channel
	p.mu.Unlock()

	var once sync.Once
	unsubscribe := func() {
		once.Do(func() {
			p.mu.Lock()
			defer p.mu.Unlock()

			subscribers := p.subscribers[topic]
			if _, exists := subscribers[subscriberID]; !exists {
				return
			}
			delete(subscribers, subscriberID)
			close(channel)
			if len(subscribers) == 0 {
				delete(p.subscribers, topic)
			}
		})
	}
	return channel, unsubscribe
}

func (p *PubSub) Publish(ctx context.Context, topics []string, payload []byte) error {
	if ctx != nil {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}
	}

	p.mu.RLock()
	defer p.mu.RUnlock()
	for _, topic := range topics {
		for _, subscriber := range p.subscribers[topic] {
			select {
			case subscriber <- payload:
			default:
			}
		}
	}
	return nil
}

func userTopic(userID string) string {
	return userTopicPrefix + userID
}
