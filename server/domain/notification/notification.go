package notification

import "context"

const (
	TypeFriendRequest    = "friend_request"
	TypeGroupJoinRequest = "group_join_request"
	TypeGroupJoinResult  = "group_join_result"

	StatusPending   = "pending"
	StatusApproved  = "approved"
	StatusRejected  = "rejected"
	StatusCancelled = "cancelled"
)

type Notification struct {
	ID              string `json:"id"`
	Type            string `json:"type"`
	Status          string `json:"status"`
	RecipientID     string `json:"recipient_id"`
	RecipientName   string `json:"recipient_name,omitempty"`
	RecipientAvatar string `json:"recipient_avatar,omitempty"`
	SenderID        string `json:"sender_id"`
	SenderName      string `json:"sender_name,omitempty"`
	SenderAvatar    string `json:"sender_avatar,omitempty"`
	GroupID         string `json:"group_id"`
	GroupName       string `json:"group_name,omitempty"`
	GroupAvatar     string `json:"group_avatar,omitempty"`
	HandledByID     string `json:"handled_by_id"`
	CreatedAt       int64  `json:"created_at"`
	HandledAt       int64  `json:"handled_at"`
}

type Repository interface {
	Create(ctx context.Context, notification Notification) error
	FindByID(ctx context.Context, id string) (Notification, error)
	Update(ctx context.Context, notification Notification) error
	ListForRecipient(ctx context.Context, recipientID string) ([]Notification, error)
	CancelPendingFriendRequests(ctx context.Context, firstUserID string, secondUserID string, handledByID string, handledAt int64) error
	ResolvePendingGroupJoinRequests(ctx context.Context, groupID string, senderID string, handledByID string, handledAt int64, status string) error
}
