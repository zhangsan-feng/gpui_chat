package db

import (
	"gin_server/domain/conversation"
	"gin_server/domain/identity"
	"gin_server/domain/notification"
	"time"
)

type userRecord struct {
	ID           string  `gorm:"primaryKey;size:36"`
	Username     string  `gorm:"not null;size:255"`
	LoginName    *string `gorm:"uniqueIndex;size:255"`
	Avatar       string  `gorm:"type:text"`
	Status       string  `gorm:"type:text"`
	PasswordHash string  `gorm:"type:text"`
	CreatedAt    time.Time
	UpdatedAt    time.Time
}

func (userRecord) TableName() string {
	return "chat_users"
}

type friendshipRecord struct {
	UserID    string `gorm:"primaryKey;size:36"`
	FriendID  string `gorm:"primaryKey;size:36"`
	CreatedAt time.Time
}

func (friendshipRecord) TableName() string {
	return "friendships"
}

type roomRecord struct {
	ID        string `gorm:"primaryKey;size:36"`
	Kind      string `gorm:"not null;index;size:32"`
	Name      string `gorm:"not null;size:255"`
	Avatar    string `gorm:"type:text"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

func (roomRecord) TableName() string {
	return "chat_rooms"
}

type groupRecord struct {
	RoomID    string `gorm:"primaryKey;size:36"`
	OwnerID   string `gorm:"not null;index;size:36"`
	AllowJoin bool   `gorm:"not null;default:true"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

func (groupRecord) TableName() string {
	return "group_chats"
}

type memberRecord struct {
	RoomID    string `gorm:"primaryKey;size:36"`
	UserID    string `gorm:"primaryKey;size:36"`
	Role      string `gorm:"not null;size:32"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

func (memberRecord) TableName() string {
	return "chat_members"
}

type conversationRecord struct {
	ID            string `gorm:"primaryKey;size:36"`
	UserID        string `gorm:"not null;uniqueIndex:idx_conversation_user_room;index;size:36"`
	RoomID        string `gorm:"not null;uniqueIndex:idx_conversation_user_room;index;size:36"`
	LastMessageID string `gorm:"size:36"`
	LastMessageAt int64  `gorm:"not null;index"`
	UnreadCount   int64  `gorm:"not null;default:0"`
	CreatedAt     time.Time
	UpdatedAt     time.Time
}

func (conversationRecord) TableName() string {
	return "conversations"
}

type messageRecord struct {
	ID           string `gorm:"primaryKey;size:36"`
	RoomID       string `gorm:"not null;index;size:36"`
	SenderID     string `gorm:"not null;index;size:36"`
	SenderName   string `gorm:"not null;size:255"`
	SenderAvatar string `gorm:"type:text"`
	Content      string `gorm:"type:text"`
	CreatedAt    int64  `gorm:"not null;index"`
}

func (messageRecord) TableName() string {
	return "chat_messages"
}

type attachmentRecord struct {
	ID        uint64 `gorm:"primaryKey"`
	MessageID string `gorm:"not null;index;size:36"`
	Name      string `gorm:"not null;size:255"`
	URL       string `gorm:"not null;type:text"`
	Position  int    `gorm:"not null"`
}

func (attachmentRecord) TableName() string {
	return "message_attachments"
}

type notificationRecord struct {
	ID          string     `gorm:"primaryKey;size:36"`
	RequestKey  string     `gorm:"not null;uniqueIndex;size:255"`
	Type        string     `gorm:"not null;index;size:32"`
	Status      string     `gorm:"not null;index;size:32"`
	RecipientID string     `gorm:"not null;index;size:36"`
	SenderID    string     `gorm:"not null;index;size:36"`
	GroupID     string     `gorm:"not null;default:'';size:36"`
	HandledByID string     `gorm:"size:36"`
	HandledAt   *time.Time `gorm:"index"`
	CreatedAt   time.Time  `gorm:"index"`
	UpdatedAt   time.Time
}

func (notificationRecord) TableName() string {
	return "notifications"
}

func newUserRecord(user identity.User) userRecord {
	var loginName *string
	if user.LoginName != "" {
		value := user.LoginName
		loginName = &value
	}
	return userRecord{ID: user.ID, Username: user.Username, LoginName: loginName, Avatar: user.Avatar, Status: user.Status, PasswordHash: user.PasswordHash}
}

func (record userRecord) toDomain() identity.User {
	loginName := ""
	if record.LoginName != nil {
		loginName = *record.LoginName
	}
	return identity.User{ID: record.ID, Username: record.Username, LoginName: loginName, Avatar: record.Avatar, Status: record.Status, PasswordHash: record.PasswordHash}
}

func newRoomRecord(group conversation.Group) roomRecord {
	return roomRecord{ID: group.ID, Kind: group.Kind, Name: group.Name, Avatar: group.Avatar}
}

func newGroupRecord(group conversation.Group) groupRecord {
	return groupRecord{RoomID: group.ID, OwnerID: groupOwnerID(group.Members), AllowJoin: group.AllowJoin}
}

func groupOwnerID(members []conversation.Member) string {
	for _, member := range members {
		if member.Role == "owner" {
			return member.UserID
		}
	}
	return ""
}

func newNotificationRecord(value notification.Notification) notificationRecord {
	var handledAt *time.Time
	if value.HandledAt > 0 {
		parsed := time.UnixMilli(value.HandledAt)
		handledAt = &parsed
	}
	requestKey := notificationRequestKey(value)
	if value.Status != notification.StatusPending {
		requestKey += ":" + value.ID
	}
	return notificationRecord{
		ID: value.ID, RequestKey: requestKey, Type: value.Type, Status: value.Status,
		RecipientID: value.RecipientID, SenderID: value.SenderID, GroupID: value.GroupID,
		HandledByID: value.HandledByID, HandledAt: handledAt,
	}
}

func (record notificationRecord) toDomain() notification.Notification {
	value := notification.Notification{
		ID: record.ID, Type: record.Type, Status: record.Status, RecipientID: record.RecipientID,
		SenderID: record.SenderID, GroupID: record.GroupID, HandledByID: record.HandledByID,
		CreatedAt: record.CreatedAt.UnixMilli(),
	}
	if record.HandledAt != nil {
		value.HandledAt = record.HandledAt.UnixMilli()
	}
	return value
}

func notificationRequestKey(value notification.Notification) string {
	return value.Type + ":" + value.RecipientID + ":" + value.SenderID + ":" + value.GroupID
}
