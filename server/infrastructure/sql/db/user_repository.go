package db

import (
	"context"
	"errors"
	"gin_server/domain/identity"
	"sort"

	"gorm.io/gorm"
	"gorm.io/gorm/clause"
)

func (s *Store) FindByID(ctx context.Context, id string) (identity.User, error) {
	var record userRecord
	if err := s.queryFor(ctx).Where("id = ?", id).First(&record).Error; err != nil {
		return identity.User{}, normalizeError(err)
	}
	return s.hydrateUser(ctx, record)
}

func (s *Store) FindByLoginName(ctx context.Context, loginName string) (identity.User, error) {
	var record userRecord
	if err := s.queryFor(ctx).Where("login_name = ?", loginName).First(&record).Error; err != nil {
		return identity.User{}, normalizeError(err)
	}
	return s.hydrateUser(ctx, record)
}

func (s *Store) Save(ctx context.Context, user identity.User) error {
	record := newUserRecord(user)
	database := s.databaseFor(ctx)
	result := database.Model(&userRecord{}).Where("id = ?", record.ID).Updates(map[string]any{
		"username":      record.Username,
		"login_name":    record.LoginName,
		"avatar":        record.Avatar,
		"status":        record.Status,
		"password_hash": record.PasswordHash,
	})
	if result.Error != nil {
		return normalizeError(result.Error)
	}
	if result.RowsAffected > 0 {
		return nil
	}
	return normalizeError(database.Create(&record).Error)
}

func (s *Store) List(ctx context.Context) ([]identity.User, error) {
	var records []userRecord
	if err := s.databaseFor(ctx).Order("username ASC").Find(&records).Error; err != nil {
		return nil, normalizeError(err)
	}
	users := make([]identity.User, 0, len(records))
	for _, record := range records {
		user, err := s.hydrateUser(ctx, record)
		if err != nil {
			return nil, err
		}
		users = append(users, user)
	}
	return users, nil
}

func (s *Store) AreFriends(ctx context.Context, userID string, friendID string) (bool, error) {
	leftID, rightID := friendshipIDs(userID, friendID)
	var record friendshipRecord
	err := s.queryFor(ctx).Where("user_id = ? AND friend_id = ?", leftID, rightID).First(&record).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return false, nil
	}
	if err != nil {
		return false, normalizeError(err)
	}
	return true, nil
}

func (s *Store) CreateFriendship(ctx context.Context, userID string, friendID string) error {
	leftID, rightID := friendshipIDs(userID, friendID)
	record := friendshipRecord{UserID: leftID, FriendID: rightID}
	return normalizeError(s.databaseFor(ctx).Clauses(clause.OnConflict{DoNothing: true}).Create(&record).Error)
}

func (s *Store) RemoveFriendship(ctx context.Context, userID string, friendID string) error {
	leftID, rightID := friendshipIDs(userID, friendID)
	return normalizeError(s.databaseFor(ctx).
		Where("user_id = ? AND friend_id = ?", leftID, rightID).
		Delete(&friendshipRecord{}).Error)
}

func (s *Store) hydrateUser(ctx context.Context, record userRecord) (identity.User, error) {
	user := record.toDomain()
	friends, err := s.listFriends(ctx, user.ID)
	if err != nil {
		return identity.User{}, err
	}
	user.Friends = friends

	var conversations []conversationRecord
	if err := s.databaseFor(ctx).
		Where("user_id = ?", user.ID).
		Order("last_message_at DESC, created_at ASC").
		Find(&conversations).Error; err != nil {
		return identity.User{}, normalizeError(err)
	}
	user.ConversationIDs = make([]string, 0, len(conversations))
	for _, conversation := range conversations {
		user.ConversationIDs = append(user.ConversationIDs, conversation.RoomID)
	}
	return user, nil
}

func (s *Store) listFriends(ctx context.Context, userID string) ([]identity.Friend, error) {
	var relations []friendshipRecord
	if err := s.databaseFor(ctx).Where("user_id = ? OR friend_id = ?", userID, userID).Find(&relations).Error; err != nil {
		return nil, normalizeError(err)
	}
	friendIDs := make([]string, 0, len(relations))
	for _, relation := range relations {
		if relation.UserID == userID {
			friendIDs = append(friendIDs, relation.FriendID)
		} else {
			friendIDs = append(friendIDs, relation.UserID)
		}
	}
	if len(friendIDs) == 0 {
		return []identity.Friend{}, nil
	}
	var records []userRecord
	if err := s.databaseFor(ctx).Where("id IN ?", friendIDs).Find(&records).Error; err != nil {
		return nil, normalizeError(err)
	}
	friends := make([]identity.Friend, 0, len(records))
	for _, record := range records {
		friends = append(friends, identity.Friend{ID: record.ID, Username: record.Username, Avatar: record.Avatar})
	}
	sort.Slice(friends, func(left int, right int) bool {
		return friends[left].Username < friends[right].Username
	})
	return friends, nil
}

func friendshipIDs(userID string, friendID string) (string, string) {
	if userID < friendID {
		return userID, friendID
	}
	return friendID, userID
}
