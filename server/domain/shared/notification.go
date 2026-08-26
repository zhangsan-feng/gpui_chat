package shared

import "context"

type Notification struct {
	Type string `json:"type"`
	Data any    `json:"data"`
}

type Notifier interface {
	Notify(ctx context.Context, userIDs []string, notification Notification) error
}
