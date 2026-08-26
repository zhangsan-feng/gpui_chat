package db

import (
	"context"
	"errors"
	"gin_server/domain/conversation"
	"gin_server/domain/shared"

	"github.com/google/uuid"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"
)

type groupRepository struct {
	store *Store
}

func NewGroupRepository(store *Store) conversation.Repository {
	return groupRepository{store: store}
}

func (s *Store) GroupRepository() conversation.Repository {
	return groupRepository{store: s}
}

func (r groupRepository) FindByID(ctx context.Context, id string) (conversation.Group, error) {
	return r.store.loadGroup(ctx, id)
}

func (r groupRepository) Save(ctx context.Context, group conversation.Group) error {
	if group.ID == "" || group.Kind == "" {
		return shared.ErrInvalidInput
	}
	database := r.store.databaseFor(ctx)
	room := newRoomRecord(group)
	if err := database.Clauses(clause.OnConflict{UpdateAll: true}).Create(&room).Error; err != nil {
		return normalizeError(err)
	}
	if group.Kind == "group" {
		if groupOwnerID(group.Members) == "" {
			return shared.ErrInvalidInput
		}
		groupRecord := newGroupRecord(group)
		if err := database.Clauses(clause.OnConflict{UpdateAll: true}).Create(&groupRecord).Error; err != nil {
			return normalizeError(err)
		}
	}
	if err := r.syncMembers(ctx, group); err != nil {
		return err
	}
	if err := r.syncMessages(ctx, group); err != nil {
		return err
	}
	return r.syncConversations(ctx, group)
}

func (r groupRepository) Delete(ctx context.Context, id string) error {
	if id == "" {
		return shared.ErrInvalidInput
	}
	database := r.store.databaseFor(ctx)
	var messageIDs []string
	if err := database.Model(&messageRecord{}).Where("room_id = ?", id).Pluck("id", &messageIDs).Error; err != nil {
		return normalizeError(err)
	}
	if len(messageIDs) > 0 {
		if err := database.Where("message_id IN ?", messageIDs).Delete(&attachmentRecord{}).Error; err != nil {
			return normalizeError(err)
		}
	}
	for _, record := range []any{&messageRecord{}, &memberRecord{}, &conversationRecord{}, &groupRecord{}} {
		if err := database.Where("room_id = ?", id).Delete(record).Error; err != nil {
			return normalizeError(err)
		}
	}
	if err := database.Where("group_id = ?", id).Delete(&notificationRecord{}).Error; err != nil {
		return normalizeError(err)
	}
	if err := database.Where("id = ?", id).Delete(&roomRecord{}).Error; err != nil {
		return normalizeError(err)
	}
	return nil
}

func (r groupRepository) List(ctx context.Context) ([]conversation.Group, error) {
	var rooms []roomRecord
	if err := r.store.databaseFor(ctx).Order("name ASC").Find(&rooms).Error; err != nil {
		return nil, normalizeError(err)
	}
	return r.loadGroups(ctx, rooms)
}

func (r groupRepository) ListDiscoverable(ctx context.Context) ([]conversation.Group, error) {
	var rooms []roomRecord
	if err := r.store.databaseFor(ctx).Where("kind = ?", "group").Order("name ASC").Find(&rooms).Error; err != nil {
		return nil, normalizeError(err)
	}
	groups := make([]conversation.Group, 0, len(rooms))
	for _, room := range rooms {
		var config groupRecord
		if err := r.store.databaseFor(ctx).Where("room_id = ?", room.ID).First(&config).Error; err != nil {
			if !errors.Is(err, gorm.ErrRecordNotFound) {
				return nil, normalizeError(err)
			}
			config.AllowJoin = true
		}
		if !config.AllowJoin {
			continue
		}
		groups = append(groups, conversation.Group{ID: room.ID, Name: room.Name, Avatar: room.Avatar, Kind: room.Kind, AllowJoin: true, Members: []conversation.Member{}, Messages: []conversation.Message{}})
	}
	return groups, nil
}

func (r groupRepository) ListForUser(ctx context.Context, userID string) ([]conversation.Group, error) {
	var conversations []conversationRecord
	if err := r.store.databaseFor(ctx).
		Where("user_id = ?", userID).
		Order("last_message_at DESC, created_at ASC").
		Find(&conversations).Error; err != nil {
		return nil, normalizeError(err)
	}
	groups := make([]conversation.Group, 0, len(conversations))
	for _, item := range conversations {
		group, err := r.store.loadGroup(ctx, item.RoomID)
		if err == shared.ErrNotFound {
			continue
		}
		if err != nil {
			return nil, err
		}
		groups = append(groups, group)
	}
	return groups, nil
}

func (r groupRepository) loadGroups(ctx context.Context, rooms []roomRecord) ([]conversation.Group, error) {
	groups := make([]conversation.Group, 0, len(rooms))
	for _, room := range rooms {
		group, err := r.store.loadGroup(ctx, room.ID)
		if err != nil {
			return nil, err
		}
		groups = append(groups, group)
	}
	return groups, nil
}

func (s *Store) loadGroup(ctx context.Context, id string) (conversation.Group, error) {
	var room roomRecord
	if err := s.queryFor(ctx).Where("id = ?", id).First(&room).Error; err != nil {
		return conversation.Group{}, normalizeError(err)
	}
	var members []memberRecord
	if err := s.databaseFor(ctx).Where("room_id = ?", room.ID).Order("created_at ASC").Find(&members).Error; err != nil {
		return conversation.Group{}, normalizeError(err)
	}
	allowJoin := true
	if room.Kind == "group" {
		var config groupRecord
		if err := s.databaseFor(ctx).Where("room_id = ?", room.ID).First(&config).Error; err != nil {
			if !errors.Is(err, gorm.ErrRecordNotFound) {
				return conversation.Group{}, normalizeError(err)
			}
			config.AllowJoin = true
		}
		allowJoin = config.AllowJoin
	}
	group := conversation.Group{ID: room.ID, Name: room.Name, Avatar: room.Avatar, Kind: room.Kind, AllowJoin: allowJoin}
	group.Members = make([]conversation.Member, 0, len(members))
	for _, member := range members {
		var user userRecord
		if err := s.databaseFor(ctx).Where("id = ?", member.UserID).First(&user).Error; err != nil {
			return conversation.Group{}, normalizeError(err)
		}
		group.Members = append(group.Members, conversation.Member{
			UserID: user.ID, Username: user.Username, Avatar: user.Avatar, Role: member.Role,
		})
	}
	var messages []messageRecord
	if err := s.databaseFor(ctx).Where("room_id = ?", room.ID).Order("created_at ASC").Find(&messages).Error; err != nil {
		return conversation.Group{}, normalizeError(err)
	}
	group.Messages = make([]conversation.Message, 0, len(messages))
	for _, message := range messages {
		var attachments []attachmentRecord
		if err := s.databaseFor(ctx).Where("message_id = ?", message.ID).Order("position ASC").Find(&attachments).Error; err != nil {
			return conversation.Group{}, normalizeError(err)
		}
		item := conversation.Message{
			ID: message.ID, SenderID: message.SenderID, SenderName: message.SenderName, SenderAvatar: message.SenderAvatar,
			Content: message.Content, CreatedAt: message.CreatedAt, Attachments: make([]conversation.Attachment, 0, len(attachments)),
		}
		for _, attachment := range attachments {
			item.Attachments = append(item.Attachments, conversation.Attachment{Name: attachment.Name, URL: attachment.URL})
		}
		group.Messages = append(group.Messages, item)
	}
	return group, nil
}

func (r groupRepository) syncMembers(ctx context.Context, group conversation.Group) error {
	database := r.store.databaseFor(ctx)
	if err := database.Where("room_id = ?", group.ID).Delete(&memberRecord{}).Error; err != nil {
		return normalizeError(err)
	}
	if len(group.Members) == 0 {
		return nil
	}
	records := make([]memberRecord, 0, len(group.Members))
	for _, member := range group.Members {
		records = append(records, memberRecord{RoomID: group.ID, UserID: member.UserID, Role: member.Role})
	}
	return normalizeError(database.Create(&records).Error)
}

func (r groupRepository) syncMessages(ctx context.Context, group conversation.Group) error {
	database := r.store.databaseFor(ctx)
	var existing []messageRecord
	if err := database.Where("room_id = ?", group.ID).Find(&existing).Error; err != nil {
		return normalizeError(err)
	}
	if len(existing) > 0 {
		messageIDs := make([]string, 0, len(existing))
		for _, message := range existing {
			messageIDs = append(messageIDs, message.ID)
		}
		if err := database.Where("message_id IN ?", messageIDs).Delete(&attachmentRecord{}).Error; err != nil {
			return normalizeError(err)
		}
	}
	if err := database.Where("room_id = ?", group.ID).Delete(&messageRecord{}).Error; err != nil {
		return normalizeError(err)
	}
	for _, message := range group.Messages {
		record := messageRecord{
			ID: message.ID, RoomID: group.ID, SenderID: message.SenderID, SenderName: message.SenderName,
			SenderAvatar: message.SenderAvatar, Content: message.Content, CreatedAt: message.CreatedAt,
		}
		if err := database.Create(&record).Error; err != nil {
			return normalizeError(err)
		}
		if len(message.Attachments) == 0 {
			continue
		}
		attachments := make([]attachmentRecord, 0, len(message.Attachments))
		for position, attachment := range message.Attachments {
			attachments = append(attachments, attachmentRecord{MessageID: message.ID, Name: attachment.Name, URL: attachment.URL, Position: position})
		}
		if err := database.Create(&attachments).Error; err != nil {
			return normalizeError(err)
		}
	}
	return nil
}

func (r groupRepository) syncConversations(ctx context.Context, group conversation.Group) error {
	database := r.store.databaseFor(ctx)
	lastMessageID := ""
	lastMessageAt := int64(0)
	if len(group.Messages) > 0 {
		last := group.Messages[len(group.Messages)-1]
		lastMessageID, lastMessageAt = last.ID, last.CreatedAt
	}
	memberIDs := make([]string, 0, len(group.Members))
	for _, member := range group.Members {
		memberIDs = append(memberIDs, member.UserID)
	}
	if len(memberIDs) == 0 {
		return nil
	}
	if err := database.Where("room_id = ? AND user_id NOT IN ?", group.ID, memberIDs).Delete(&conversationRecord{}).Error; err != nil {
		return normalizeError(err)
	}
	for _, userID := range memberIDs {
		var existing conversationRecord
		err := database.Where("user_id = ? AND room_id = ?", userID, group.ID).First(&existing).Error
		if err != nil && !errors.Is(err, gorm.ErrRecordNotFound) {
			return normalizeError(err)
		}
		if err == nil {
			existing.LastMessageID, existing.LastMessageAt = lastMessageID, lastMessageAt
			if updateErr := database.Save(&existing).Error; updateErr != nil {
				return normalizeError(updateErr)
			}
			continue
		}
		record := conversationRecord{ID: uuid.NewString(), UserID: userID, RoomID: group.ID, LastMessageID: lastMessageID, LastMessageAt: lastMessageAt}
		if createErr := database.Create(&record).Error; createErr != nil {
			return normalizeError(createErr)
		}
	}
	return nil
}
