package social

import (
	"context"
	"gin_server/domain/conversation"
	"gin_server/domain/identity"
	"gin_server/domain/notification"
	"gin_server/domain/shared"
	"strings"
	"time"

	"github.com/google/uuid"
)

type Service struct {
	users         identity.Repository
	groups        conversation.Repository
	notifications notification.Repository
	notifier      shared.Notifier
	transactions  shared.TransactionManager
}

type ConversationDeletedEvent struct {
	GroupID string `json:"group_id"`
}

func NewService(users identity.Repository, groups conversation.Repository, notifications notification.Repository, notifier shared.Notifier, transactions shared.TransactionManager) *Service {
	return &Service{users: users, groups: groups, notifications: notifications, notifier: notifier, transactions: transactions}
}

func (s *Service) RequestFriend(ctx context.Context, userID string, friendID string) (notification.Notification, error) {
	if userID == "" || friendID == "" || userID == friendID {
		return notification.Notification{}, shared.ErrInvalidInput
	}
	var request notification.Notification
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		user, friend, err := s.findUsers(txCtx, userID, friendID)
		if err != nil {
			return err
		}
		areFriends, err := s.users.AreFriends(txCtx, user.ID, friend.ID)
		if err != nil {
			return err
		}
		if areFriends {
			return shared.ErrConflict
		}
		request = notification.Notification{
			ID: uuid.NewString(), Type: notification.TypeFriendRequest, Status: notification.StatusPending,
			RecipientID: friend.ID, RecipientName: friend.Username, RecipientAvatar: friend.Avatar,
			SenderID: user.ID, SenderName: user.Username, SenderAvatar: user.Avatar,
			CreatedAt: time.Now().UnixMilli(),
		}
		return s.notifications.Create(txCtx, request)
	})
	if err != nil {
		return notification.Notification{}, err
	}
	s.notify(ctx, []string{friendID}, "friend.requested", request)
	return request, nil
}

func (s *Service) RemoveFriend(ctx context.Context, userID string, friendID string) error {
	if userID == "" || friendID == "" || userID == friendID {
		return shared.ErrInvalidInput
	}
	deletedConversationIDs := make([]string, 0)
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		user, friend, err := s.findUsers(txCtx, userID, friendID)
		if err != nil {
			return err
		}
		areFriends, err := s.users.AreFriends(txCtx, user.ID, friend.ID)
		if err != nil {
			return err
		}
		if !areFriends {
			return shared.ErrNotFound
		}
		conversations, err := s.groups.ListForUser(txCtx, user.ID)
		if err != nil {
			return err
		}
		for _, conversation := range conversations {
			if conversation.Kind != "direct" || !conversationHasMember(conversation, friend.ID) {
				continue
			}
			if err := s.groups.Delete(txCtx, conversation.ID); err != nil {
				return err
			}
			deletedConversationIDs = append(deletedConversationIDs, conversation.ID)
		}
		return s.users.RemoveFriendship(txCtx, user.ID, friend.ID)
	})
	if err != nil {
		return err
	}
	s.notify(ctx, []string{userID}, "friend.removed", identity.Friend{ID: friendID})
	s.notify(ctx, []string{friendID}, "friend.removed", identity.Friend{ID: userID})
	for _, groupID := range deletedConversationIDs {
		event := ConversationDeletedEvent{GroupID: groupID}
		s.notify(ctx, []string{userID, friendID}, "conversation.deleted", event)
	}
	return nil
}

func (s *Service) ReviewFriendRequest(ctx context.Context, requestID string, reviewerID string, approved bool) (notification.Notification, error) {
	if requestID == "" || reviewerID == "" {
		return notification.Notification{}, shared.ErrInvalidInput
	}
	preview, err := s.notifications.FindByID(ctx, requestID)
	if err != nil {
		return notification.Notification{}, err
	}
	var request notification.Notification
	var requester identity.User
	var recipient identity.User
	var direct conversation.Group
	err = s.transactions.Execute(ctx, func(txCtx context.Context) error {
		var err error
		requester, recipient, err = s.findUsers(txCtx, preview.SenderID, preview.RecipientID)
		if err != nil {
			return err
		}
		request, err = s.notifications.FindByID(txCtx, requestID)
		if err != nil {
			return err
		}
		if request.Type != notification.TypeFriendRequest || request.Status != notification.StatusPending {
			return shared.ErrConflict
		}
		if request.RecipientID != reviewerID {
			return shared.ErrForbidden
		}
		request.Status = notification.StatusRejected
		request.HandledByID = reviewerID
		request.HandledAt = time.Now().UnixMilli()
		if approved {
			areFriends, friendErr := s.users.AreFriends(txCtx, requester.ID, recipient.ID)
			if friendErr != nil {
				return friendErr
			}
			if areFriends {
				return shared.ErrConflict
			}
			if createErr := s.users.CreateFriendship(txCtx, requester.ID, recipient.ID); createErr != nil {
				return createErr
			}
			direct = conversation.Group{
				ID: uuid.NewString(), Name: directConversationName(requester.Username, recipient.Username), Kind: "direct",
				Members: []conversation.Member{
					{UserID: requester.ID, Username: requester.Username, Avatar: requester.Avatar, Role: "member"},
					{UserID: recipient.ID, Username: recipient.Username, Avatar: recipient.Avatar, Role: "member"},
				},
				Messages: []conversation.Message{},
			}
			if saveErr := s.groups.Save(txCtx, direct); saveErr != nil {
				return saveErr
			}
			request.Status = notification.StatusApproved
		}
		if err := s.notifications.Update(txCtx, request); err != nil {
			return err
		}
		if request.Status != notification.StatusApproved {
			return nil
		}
		return s.notifications.CancelPendingFriendRequests(txCtx, requester.ID, recipient.ID, reviewerID, request.HandledAt)
	})
	if err != nil {
		return notification.Notification{}, err
	}
	request.SenderName = requester.Username
	request.SenderAvatar = requester.Avatar
	request.RecipientName = recipient.Username
	request.RecipientAvatar = recipient.Avatar
	s.notify(ctx, []string{request.SenderID}, "friend.request_reviewed", request)
	if request.Status == notification.StatusApproved {
		s.notify(ctx, []string{requester.ID}, "add_friend", identity.Friend{
			ID: recipient.ID, Username: recipient.Username, Avatar: recipient.Avatar,
		})
		s.notify(ctx, []string{recipient.ID}, "add_friend", identity.Friend{
			ID: requester.ID, Username: requester.Username, Avatar: requester.Avatar,
		})
		s.notify(ctx, []string{requester.ID, recipient.ID}, "conversation.created", direct)
	}
	return request, nil
}

func (s *Service) findUsers(ctx context.Context, firstID string, secondID string) (identity.User, identity.User, error) {
	if firstID < secondID {
		first, err := s.users.FindByID(ctx, firstID)
		if err != nil {
			return identity.User{}, identity.User{}, err
		}
		second, err := s.users.FindByID(ctx, secondID)
		if err != nil {
			return identity.User{}, identity.User{}, err
		}
		return first, second, nil
	}
	second, err := s.users.FindByID(ctx, secondID)
	if err != nil {
		return identity.User{}, identity.User{}, err
	}
	first, err := s.users.FindByID(ctx, firstID)
	if err != nil {
		return identity.User{}, identity.User{}, err
	}
	return first, second, nil
}

func (s *Service) notify(ctx context.Context, userIDs []string, notificationType string, data any) {
	if s.notifier == nil {
		return
	}
	_ = s.notifier.Notify(ctx, userIDs, shared.Notification{Type: notificationType, Data: data})
}

func directConversationName(first string, second string) string {
	names := []string{first, second}
	if names[0] > names[1] {
		names[0], names[1] = names[1], names[0]
	}
	return strings.Join(names, " / ")
}

func conversationHasMember(group conversation.Group, userID string) bool {
	for _, member := range group.Members {
		if member.UserID == userID {
			return true
		}
	}
	return false
}
