package identity

import "context"

type Friend struct {
	ID       string `json:"id"`
	Username string `json:"username"`
	Avatar   string `json:"avatar"`
}

type User struct {
	ID              string   `json:"id"`
	Username        string   `json:"username"`
	LoginName       string   `json:"login_name"`
	Avatar          string   `json:"avatar"`
	Status          string   `json:"status"`
	PasswordHash    string   `json:"-"`
	ConversationIDs []string `json:"conversation_ids"`
	Friends         []Friend `json:"friends"`
}

type Repository interface {
	FindByID(ctx context.Context, id string) (User, error)
	FindByLoginName(ctx context.Context, loginName string) (User, error)
	Save(ctx context.Context, user User) error
	List(ctx context.Context) ([]User, error)
	AreFriends(ctx context.Context, userID string, friendID string) (bool, error)
	CreateFriendship(ctx context.Context, userID string, friendID string) error
	RemoveFriendship(ctx context.Context, userID string, friendID string) error
}
