package realtime

import (
	"context"
	"encoding/json"
	"gin_server/domain/shared"
)

var _ shared.Notifier = (*Notifier)(nil)

type Notifier struct {
	pubSub *PubSub
}

func NewNotifier(pubSub *PubSub) *Notifier {
	return &Notifier{pubSub: pubSub}
}

func (n *Notifier) Notify(ctx context.Context, userIDs []string, notification shared.Notification) error {
	payload, err := json.Marshal(notification)
	if err != nil {
		return err
	}

	topics := make([]string, 0, len(userIDs))
	for _, userID := range userIDs {
		topics = append(topics, userTopic(userID))
	}
	return n.pubSub.Publish(ctx, topics, payload)
}
