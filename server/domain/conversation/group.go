package conversation

import "context"

const (
	RoleOwner  = "owner"
	RoleAdmin  = "admin"
	RoleMember = "member"
)

func IsValidRole(role string) bool {
	return role == RoleOwner || role == RoleAdmin || role == RoleMember
}

type Member struct {
	UserID   string `json:"user_id"`
	Username string `json:"username"`
	Avatar   string `json:"avatar"`
	Role     string `json:"role"`
}

type Attachment struct {
	Name string `json:"name"`
	URL  string `json:"url"`
}

type Message struct {
	ID           string       `json:"id"`
	SenderID     string       `json:"sender_id"`
	SenderName   string       `json:"sender_name"`
	SenderAvatar string       `json:"sender_avatar"`
	Content      string       `json:"content"`
	Attachments  []Attachment `json:"attachments"`
	CreatedAt    int64        `json:"created_at"`
}

type Group struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	Avatar    string    `json:"avatar"`
	Kind      string    `json:"kind"`
	AllowJoin bool      `json:"allow_join"`
	Members   []Member  `json:"members"`
	Messages  []Message `json:"messages"`
}

type Repository interface {
	FindByID(ctx context.Context, id string) (Group, error)
	Save(ctx context.Context, group Group) error
	Delete(ctx context.Context, id string) error
	List(ctx context.Context) ([]Group, error)
	ListDiscoverable(ctx context.Context) ([]Group, error)
	ListForUser(ctx context.Context, userID string) ([]Group, error)
}
