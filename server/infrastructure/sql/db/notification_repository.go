package db

import (
	"context"
	"gin_server/domain/notification"
	"gin_server/domain/shared"
	"time"
)

type notificationRepository struct {
	store *Store
}

func NewNotificationRepository(store *Store) notification.Repository {
	return notificationRepository{store: store}
}

func (s *Store) NotificationRepository() notification.Repository {
	return notificationRepository{store: s}
}

func (r notificationRepository) Create(ctx context.Context, value notification.Notification) error {
	record := newNotificationRecord(value)
	return normalizeError(r.store.databaseFor(ctx).Create(&record).Error)
}

func (r notificationRepository) FindByID(ctx context.Context, id string) (notification.Notification, error) {
	var record notificationRecord
	if err := r.store.queryFor(ctx).Where("id = ?", id).First(&record).Error; err != nil {
		return notification.Notification{}, normalizeError(err)
	}
	return record.toDomain(), nil
}

func (r notificationRepository) Update(ctx context.Context, value notification.Notification) error {
	values := map[string]any{
		"status":        value.Status,
		"handled_by_id": value.HandledByID,
		"handled_at":    nil,
	}
	if value.Status != notification.StatusPending {
		values["request_key"] = notificationRequestKey(value) + ":" + value.ID
	}
	if value.HandledAt > 0 {
		values["handled_at"] = newNotificationRecord(value).HandledAt
	}
	result := r.store.databaseFor(ctx).Model(&notificationRecord{}).Where("id = ?", value.ID).Updates(values)
	if result.Error != nil {
		return normalizeError(result.Error)
	}
	if result.RowsAffected == 0 {
		return shared.ErrNotFound
	}
	return nil
}

func (r notificationRepository) ListForRecipient(ctx context.Context, recipientID string) ([]notification.Notification, error) {
	var records []notificationRecord
	if err := r.store.databaseFor(ctx).
		Where("recipient_id = ?", recipientID).
		Order("created_at DESC").
		Find(&records).Error; err != nil {
		return nil, normalizeError(err)
	}
	values := make([]notification.Notification, 0, len(records))
	for _, record := range records {
		values = append(values, record.toDomain())
	}
	return values, nil
}

func (r notificationRepository) CancelPendingFriendRequests(ctx context.Context, firstUserID string, secondUserID string, handledByID string, handledAt int64) error {
	resolvedAt := time.UnixMilli(handledAt)
	database := r.store.databaseFor(ctx)
	var records []notificationRecord
	if err := database.
		Where("type = ? AND status = ?", notification.TypeFriendRequest, notification.StatusPending).
		Where("(sender_id = ? AND recipient_id = ?) OR (sender_id = ? AND recipient_id = ?)", firstUserID, secondUserID, secondUserID, firstUserID).
		Find(&records).Error; err != nil {
		return normalizeError(err)
	}
	for _, record := range records {
		result := database.Model(&notificationRecord{}).Where("id = ?", record.ID).Updates(map[string]any{
			"status":        notification.StatusCancelled,
			"handled_by_id": handledByID,
			"handled_at":    &resolvedAt,
			"request_key":   record.RequestKey + ":resolved:" + record.ID,
		})
		if result.Error != nil {
			return normalizeError(result.Error)
		}
	}
	return nil
}

func (r notificationRepository) ResolvePendingGroupJoinRequests(ctx context.Context, groupID string, senderID string, handledByID string, handledAt int64, status string) error {
	resolvedAt := time.UnixMilli(handledAt)
	database := r.store.databaseFor(ctx)
	var records []notificationRecord
	if err := database.
		Where("type = ? AND status = ?", notification.TypeGroupJoinRequest, notification.StatusPending).
		Where("group_id = ? AND sender_id = ?", groupID, senderID).
		Find(&records).Error; err != nil {
		return normalizeError(err)
	}
	for _, record := range records {
		result := database.Model(&notificationRecord{}).Where("id = ?", record.ID).Updates(map[string]any{
			"status":        status,
			"handled_by_id": handledByID,
			"handled_at":    &resolvedAt,
			"request_key":   record.RequestKey + ":resolved:" + record.ID,
		})
		if result.Error != nil {
			return normalizeError(result.Error)
		}
	}
	return nil
}
