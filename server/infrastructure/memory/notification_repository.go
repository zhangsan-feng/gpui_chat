package memory

import (
	"context"
	"gin_server/domain/notification"
	"gin_server/domain/shared"
	"sort"
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
	unlock := r.store.lockOperation(ctx)
	defer unlock()
	r.store.mu.Lock()
	defer r.store.mu.Unlock()
	for _, existing := range r.store.notifications {
		if existing.Status == notification.StatusPending && notificationKey(existing) == notificationKey(value) {
			return shared.ErrConflict
		}
	}
	r.store.notifications[value.ID] = value
	return nil
}

func (r notificationRepository) FindByID(ctx context.Context, id string) (notification.Notification, error) {
	unlock := r.store.lockOperation(ctx)
	defer unlock()
	r.store.mu.RLock()
	defer r.store.mu.RUnlock()
	value, exists := r.store.notifications[id]
	if !exists {
		return notification.Notification{}, shared.ErrNotFound
	}
	return value, nil
}

func (r notificationRepository) Update(ctx context.Context, value notification.Notification) error {
	unlock := r.store.lockOperation(ctx)
	defer unlock()
	r.store.mu.Lock()
	defer r.store.mu.Unlock()
	if _, exists := r.store.notifications[value.ID]; !exists {
		return shared.ErrNotFound
	}
	r.store.notifications[value.ID] = value
	return nil
}

func (r notificationRepository) ListForRecipient(ctx context.Context, recipientID string) ([]notification.Notification, error) {
	unlock := r.store.lockOperation(ctx)
	defer unlock()
	r.store.mu.RLock()
	defer r.store.mu.RUnlock()
	items := make([]notification.Notification, 0)
	for _, value := range r.store.notifications {
		if value.RecipientID == recipientID {
			items = append(items, value)
		}
	}
	sort.Slice(items, func(left int, right int) bool {
		return items[left].CreatedAt > items[right].CreatedAt
	})
	return items, nil
}

func (r notificationRepository) CancelPendingFriendRequests(ctx context.Context, firstUserID string, secondUserID string, handledByID string, handledAt int64) error {
	unlock := r.store.lockOperation(ctx)
	defer unlock()
	r.store.mu.Lock()
	defer r.store.mu.Unlock()
	for id, value := range r.store.notifications {
		if value.Type != notification.TypeFriendRequest || value.Status != notification.StatusPending {
			continue
		}
		if !sameFriendPair(value, firstUserID, secondUserID) {
			continue
		}
		value.Status = notification.StatusCancelled
		value.HandledByID = handledByID
		value.HandledAt = handledAt
		r.store.notifications[id] = value
	}
	return nil
}

func (r notificationRepository) ResolvePendingGroupJoinRequests(ctx context.Context, groupID string, senderID string, handledByID string, handledAt int64, status string) error {
	unlock := r.store.lockOperation(ctx)
	defer unlock()
	r.store.mu.Lock()
	defer r.store.mu.Unlock()
	for id, value := range r.store.notifications {
		if value.Type != notification.TypeGroupJoinRequest || value.Status != notification.StatusPending {
			continue
		}
		if value.GroupID != groupID || value.SenderID != senderID {
			continue
		}
		value.Status = status
		value.HandledByID = handledByID
		value.HandledAt = handledAt
		r.store.notifications[id] = value
	}
	return nil
}

func notificationKey(value notification.Notification) string {
	return value.Type + "\x00" + value.RecipientID + "\x00" + value.SenderID + "\x00" + value.GroupID
}

func sameFriendPair(value notification.Notification, firstUserID string, secondUserID string) bool {
	return (value.SenderID == firstUserID && value.RecipientID == secondUserID) ||
		(value.SenderID == secondUserID && value.RecipientID == firstUserID)
}
