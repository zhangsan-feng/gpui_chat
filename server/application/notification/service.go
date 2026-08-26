package notification

import (
	"context"
	"errors"
	"gin_server/domain/conversation"
	"gin_server/domain/identity"
	domainnotification "gin_server/domain/notification"
	"gin_server/domain/shared"
)

type Service struct {
	notifications domainnotification.Repository
	users         identity.Repository
	groups        conversation.Repository
	transactions  shared.TransactionManager
}

func NewService(
	notifications domainnotification.Repository,
	users identity.Repository,
	groups conversation.Repository,
	transactions shared.TransactionManager,
) *Service {
	return &Service{
		notifications: notifications,
		users:         users,
		groups:        groups,
		transactions:  transactions,
	}
}

func (s *Service) ListForUser(ctx context.Context, userID string) ([]domainnotification.Notification, error) {
	if userID == "" {
		return nil, shared.ErrInvalidInput
	}
	var values []domainnotification.Notification
	err := s.transactions.Read(ctx, func(txCtx context.Context) error {
		var err error
		values, err = s.notifications.ListForRecipient(txCtx, userID)
		if err != nil {
			return err
		}
		for index := range values {
			if err := s.enrich(txCtx, &values[index]); err != nil {
				return err
			}
		}
		return nil
	})
	return values, err
}

func (s *Service) enrich(ctx context.Context, value *domainnotification.Notification) error {
	if value.SenderID != "" {
		user, err := s.users.FindByID(ctx, value.SenderID)
		if err == nil {
			value.SenderName = user.Username
			value.SenderAvatar = user.Avatar
		} else if !errors.Is(err, shared.ErrNotFound) {
			return err
		}
	}
	if value.RecipientID != "" {
		user, err := s.users.FindByID(ctx, value.RecipientID)
		if err == nil {
			value.RecipientName = user.Username
			value.RecipientAvatar = user.Avatar
		} else if !errors.Is(err, shared.ErrNotFound) {
			return err
		}
	}
	if value.GroupID != "" {
		group, err := s.groups.FindByID(ctx, value.GroupID)
		if err == nil {
			value.GroupName = group.Name
			value.GroupAvatar = group.Avatar
		} else if !errors.Is(err, shared.ErrNotFound) {
			return err
		}
	}
	return nil
}
