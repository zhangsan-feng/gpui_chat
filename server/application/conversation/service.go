package conversation

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

const messageHistoryLimit = 500
const maxMessageAttachmentCount = 4

type Service struct {
	users         identity.Repository
	groups        conversation.Repository
	notifications notification.Repository
	notifier      shared.Notifier
	transactions  shared.TransactionManager
}

type CreateGroupCommand struct {
	Name    string
	Avatar  string
	Kind    string
	OwnerID string
}

type UpdateGroupCommand struct {
	GroupID    string
	OperatorID string
	Name       string
	Avatar     string
	AllowJoin  bool
}

type SendMessageCommand struct {
	SenderID    string
	Content     string
	Attachments []conversation.Attachment
}

type MessageCreatedEvent struct {
	conversation.Message
	GroupID string `json:"group_id"`
}

type GroupMemberLeftEvent struct {
	GroupID string `json:"group_id"`
	UserID  string `json:"user_id"`
}

type GroupDisbandedEvent struct {
	GroupID string `json:"group_id"`
}

type GroupMemberRoleChangedEvent struct {
	GroupID string `json:"group_id"`
	UserID  string `json:"user_id"`
	Role    string `json:"role"`
}

type GroupMemberAddedEvent struct {
	GroupID string              `json:"group_id"`
	Member  conversation.Member `json:"member"`
}

type GroupUpdatedEvent struct {
	GroupID   string `json:"group_id"`
	Name      string `json:"name"`
	Avatar    string `json:"avatar"`
	AllowJoin bool   `json:"allow_join"`
}

type GroupJoinRequestResolvedEvent struct {
	GroupID     string `json:"group_id"`
	SenderID    string `json:"sender_id"`
	Status      string `json:"status"`
	HandledByID string `json:"handled_by_id"`
	HandledAt   int64  `json:"handled_at"`
}

func NewService(users identity.Repository, groups conversation.Repository, notifications notification.Repository, notifier shared.Notifier, transactions shared.TransactionManager) *Service {
	return &Service{users: users, groups: groups, notifications: notifications, notifier: notifier, transactions: transactions}
}

func (s *Service) CreateGroup(ctx context.Context, command CreateGroupCommand) (conversation.Group, error) {
	var group conversation.Group
	var recipients []string
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		name := strings.TrimSpace(command.Name)
		if name == "" || command.OwnerID == "" {
			return shared.ErrInvalidInput
		}
		recipients = []string{command.OwnerID}
		members := make([]conversation.Member, 0, len(recipients))
		for _, memberID := range recipients {
			user, err := s.users.FindByID(txCtx, memberID)
			if err != nil {
				return err
			}
			role := conversation.RoleMember
			if user.ID == command.OwnerID {
				role = conversation.RoleOwner
			}
			members = append(members, conversation.Member{UserID: user.ID, Username: user.Username, Avatar: user.Avatar, Role: role})
		}
		kind := command.Kind
		if kind == "" {
			kind = "group"
		}
		if kind != "group" {
			return shared.ErrInvalidInput
		}
		group = conversation.Group{
			ID: uuid.NewString(), Name: name, Avatar: command.Avatar, Kind: kind, AllowJoin: true,
			Members: members, Messages: []conversation.Message{},
		}
		return s.groups.Save(txCtx, group)
	})
	if err != nil {
		return conversation.Group{}, err
	}
	s.notify(ctx, recipients, "conversation.created", group)
	return group, nil
}

func (s *Service) RequestJoinGroup(ctx context.Context, groupID string, userID string) (notification.Notification, error) {
	if groupID == "" || userID == "" {
		return notification.Notification{}, shared.ErrInvalidInput
	}
	var request notification.Notification
	var requests []notification.Notification
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		group, err := s.groups.FindByID(txCtx, groupID)
		if err != nil {
			return err
		}
		if group.Kind != "group" || !group.AllowJoin || groupHasMember(group, userID) {
			return shared.ErrConflict
		}
		user, err := s.users.FindByID(txCtx, userID)
		if err != nil {
			return err
		}
		approverIDs := groupApproverIDs(group)
		if len(approverIDs) == 0 {
			return shared.ErrInvalidInput
		}
		requests = make([]notification.Notification, 0, len(approverIDs))
		for _, approverID := range approverIDs {
			value := notification.Notification{
				ID: uuid.NewString(), Type: notification.TypeGroupJoinRequest, Status: notification.StatusPending,
				RecipientID: approverID, SenderID: userID, SenderName: user.Username, SenderAvatar: user.Avatar,
				GroupID: group.ID, GroupName: group.Name, GroupAvatar: group.Avatar,
				CreatedAt: time.Now().UnixMilli(),
			}
			if request.ID == "" {
				request = value
			}
			if err := s.notifications.Create(txCtx, value); err != nil {
				return err
			}
			requests = append(requests, value)
		}
		return nil
	})
	if err != nil {
		return notification.Notification{}, err
	}
	for _, value := range requests {
		s.notify(ctx, []string{value.RecipientID}, "group.join_requested", value)
	}
	return request, nil
}

func (s *Service) UpdateGroup(ctx context.Context, command UpdateGroupCommand) (conversation.Group, error) {
	if command.GroupID == "" || command.OperatorID == "" || strings.TrimSpace(command.Name) == "" {
		return conversation.Group{}, shared.ErrInvalidInput
	}

	var group conversation.Group
	var recipients []string
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		var err error
		group, err = s.groups.FindByID(txCtx, command.GroupID)
		if err != nil {
			return err
		}
		if group.Kind != "group" {
			return shared.ErrConflict
		}
		if groupOwnerID(group) != command.OperatorID {
			return shared.ErrForbidden
		}

		group.Name = strings.TrimSpace(command.Name)
		if command.Avatar != "" {
			group.Avatar = command.Avatar
		}
		group.AllowJoin = command.AllowJoin
		recipients = memberIDs(group.Members)
		return s.groups.Save(txCtx, group)
	})
	if err != nil {
		return conversation.Group{}, err
	}

	s.notify(ctx, recipients, "group.updated", GroupUpdatedEvent{
		GroupID: group.ID, Name: group.Name, Avatar: group.Avatar, AllowJoin: group.AllowJoin,
	})
	return group, nil
}

func (s *Service) ReviewJoinRequest(ctx context.Context, requestID string, reviewerID string, approved bool) (notification.Notification, error) {
	if requestID == "" || reviewerID == "" {
		return notification.Notification{}, shared.ErrInvalidInput
	}
	preview, err := s.notifications.FindByID(ctx, requestID)
	if err != nil {
		return notification.Notification{}, err
	}
	var request notification.Notification
	var result notification.Notification
	var joinedGroup conversation.Group
	var groupRecipients []string
	var approverRecipients []string
	err = s.transactions.Execute(ctx, func(txCtx context.Context) error {
		group, err := s.groups.FindByID(txCtx, preview.GroupID)
		if err != nil {
			return err
		}
		user, err := s.users.FindByID(txCtx, preview.SenderID)
		if err != nil {
			return err
		}
		request, err = s.notifications.FindByID(txCtx, requestID)
		if err != nil {
			return err
		}
		if request.Type != notification.TypeGroupJoinRequest || request.Status != notification.StatusPending {
			return shared.ErrConflict
		}
		if group.Kind != "group" {
			return shared.ErrConflict
		}
		approverRecipients = groupApproverIDs(group)
		reviewer, exists := findGroupMember(group, reviewerID)
		if !exists || (reviewer.Role != conversation.RoleOwner && reviewer.Role != conversation.RoleAdmin) {
			return shared.ErrForbidden
		}
		request.Status = notification.StatusRejected
		request.HandledByID = reviewerID
		request.HandledAt = time.Now().UnixMilli()
		if approved {
			if groupHasMember(group, request.SenderID) {
				return shared.ErrConflict
			}
			group.Members = append(group.Members, conversation.Member{UserID: user.ID, Username: user.Username, Avatar: user.Avatar, Role: conversation.RoleMember})
			if saveErr := s.groups.Save(txCtx, group); saveErr != nil {
				return saveErr
			}
			joinedGroup = group
			groupRecipients = memberIDs(group.Members)
			request.Status = notification.StatusApproved
		}
		if err := s.notifications.ResolvePendingGroupJoinRequests(
			txCtx, request.GroupID, request.SenderID, reviewerID, request.HandledAt, request.Status,
		); err != nil {
			return err
		}
		result = notification.Notification{
			ID: uuid.NewString(), Type: notification.TypeGroupJoinResult, Status: request.Status,
			RecipientID: request.SenderID, SenderID: reviewerID, GroupID: request.GroupID,
			SenderName: reviewer.Username, SenderAvatar: reviewer.Avatar,
			GroupName: group.Name, GroupAvatar: group.Avatar,
			HandledByID: reviewerID, CreatedAt: request.HandledAt, HandledAt: request.HandledAt,
		}
		return s.notifications.Create(txCtx, result)
	})
	if err != nil {
		return notification.Notification{}, err
	}
	s.notify(ctx, approverRecipients, "group.join_request_resolved", GroupJoinRequestResolvedEvent{
		GroupID:     request.GroupID,
		SenderID:    request.SenderID,
		Status:      request.Status,
		HandledByID: reviewerID,
		HandledAt:   request.HandledAt,
	})
	s.notify(ctx, []string{request.SenderID}, "group.join_request_reviewed", result)
	if joinedGroup.ID != "" {
		s.notify(ctx, []string{request.SenderID}, "conversation.created", joinedGroup)
		member, _ := findGroupMember(joinedGroup, request.SenderID)
		s.notify(ctx, groupRecipients, "group.member_added", GroupMemberAddedEvent{
			GroupID: request.GroupID,
			Member:  member,
		})
	}
	return request, nil
}

func (s *Service) UpdateMemberRole(ctx context.Context, groupID string, operatorID string, memberID string, role string) error {
	if groupID == "" || operatorID == "" || memberID == "" || !conversation.IsValidRole(role) || role == conversation.RoleOwner {
		return shared.ErrInvalidInput
	}

	var recipients []string
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		group, err := s.groups.FindByID(txCtx, groupID)
		if err != nil {
			return err
		}
		if group.Kind != "group" {
			return shared.ErrConflict
		}

		operator, exists := findGroupMember(group, operatorID)
		if !exists {
			return shared.ErrForbidden
		}
		if operator.Role != conversation.RoleOwner {
			return shared.ErrForbidden
		}

		memberIndex := -1
		for index, member := range group.Members {
			if member.UserID == memberID {
				if member.Role == conversation.RoleOwner {
					return shared.ErrForbidden
				}
				memberIndex = index
				break
			}
		}
		if memberIndex < 0 {
			return shared.ErrNotFound
		}
		if group.Members[memberIndex].Role == role {
			return nil
		}

		group.Members[memberIndex].Role = role
		recipients = memberIDs(group.Members)
		return s.groups.Save(txCtx, group)
	})
	if err != nil {
		return err
	}
	if len(recipients) > 0 {
		s.notify(ctx, recipients, "group.member_role_changed", GroupMemberRoleChangedEvent{
			GroupID: groupID,
			UserID:  memberID,
			Role:    role,
		})
	}
	return nil
}

func (s *Service) LeaveGroup(ctx context.Context, groupID string, userID string) error {
	if groupID == "" || userID == "" {
		return shared.ErrInvalidInput
	}
	var recipients []string
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		group, err := s.groups.FindByID(txCtx, groupID)
		if err != nil {
			return err
		}
		if group.Kind != "group" {
			return shared.ErrConflict
		}
		member, exists := findGroupMember(group, userID)
		if !exists {
			return shared.ErrNotFound
		}
		if member.Role == conversation.RoleOwner {
			return shared.ErrForbidden
		}
		members := make([]conversation.Member, 0, len(group.Members)-1)
		for _, current := range group.Members {
			if current.UserID != userID {
				members = append(members, current)
			}
		}
		group.Members = members
		recipients = memberIDs(group.Members)
		return s.groups.Save(txCtx, group)
	})
	if err != nil {
		return err
	}
	s.notify(ctx, recipients, "group.member_left", GroupMemberLeftEvent{GroupID: groupID, UserID: userID})
	return nil
}

func (s *Service) DisbandGroup(ctx context.Context, groupID string, userID string) error {
	if groupID == "" || userID == "" {
		return shared.ErrInvalidInput
	}
	var recipients []string
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		group, err := s.groups.FindByID(txCtx, groupID)
		if err != nil {
			return err
		}
		if group.Kind != "group" {
			return shared.ErrConflict
		}
		if groupOwnerID(group) != userID {
			return shared.ErrForbidden
		}
		recipients = memberIDs(group.Members)
		return s.groups.Delete(txCtx, groupID)
	})
	if err != nil {
		return err
	}
	s.notify(ctx, recipients, "group.disbanded", GroupDisbandedEvent{GroupID: groupID})
	return nil
}

func (s *Service) SendMessage(ctx context.Context, groupID string, command SendMessageCommand) (conversation.Message, error) {
	if groupID == "" || command.SenderID == "" || (strings.TrimSpace(command.Content) == "" && len(command.Attachments) == 0) {
		return conversation.Message{}, shared.ErrInvalidInput
	}
	if len(command.Attachments) > maxMessageAttachmentCount {
		return conversation.Message{}, shared.ErrInvalidInput
	}
	var message conversation.Message
	var recipients []string
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		group, err := s.groups.FindByID(txCtx, groupID)
		if err != nil {
			return err
		}
		if !groupHasMember(group, command.SenderID) {
			return shared.ErrNotFound
		}
		sender, err := s.users.FindByID(txCtx, command.SenderID)
		if err != nil {
			return err
		}
		message = conversation.Message{
			ID: uuid.NewString(), SenderID: sender.ID, SenderName: sender.Username, SenderAvatar: sender.Avatar,
			Content: strings.TrimSpace(command.Content), Attachments: command.Attachments, CreatedAt: time.Now().UnixMilli(),
		}
		group.Messages = append(group.Messages, message)
		if len(group.Messages) > messageHistoryLimit {
			group.Messages = group.Messages[len(group.Messages)-messageHistoryLimit:]
		}
		if err := s.groups.Save(txCtx, group); err != nil {
			return err
		}
		recipients = memberIDs(group.Members)
		return nil
	})
	if err != nil {
		return conversation.Message{}, err
	}
	s.notify(ctx, recipients, "message.created", MessageCreatedEvent{Message: message, GroupID: groupID})
	return message, nil
}

func (s *Service) ListForUser(ctx context.Context, userID string) ([]conversation.Group, error) {
	var groups []conversation.Group
	err := s.transactions.Read(ctx, func(txCtx context.Context) error {
		if _, err := s.users.FindByID(txCtx, userID); err != nil {
			return err
		}
		var err error
		groups, err = s.groups.ListForUser(txCtx, userID)
		return err
	})
	return groups, err
}

func (s *Service) notify(ctx context.Context, userIDs []string, notificationType string, data any) {
	if s.notifier == nil {
		return
	}
	_ = s.notifier.Notify(ctx, userIDs, shared.Notification{Type: notificationType, Data: data})
}

func groupHasMember(group conversation.Group, userID string) bool {
	for _, member := range group.Members {
		if member.UserID == userID {
			return true
		}
	}
	return false
}

func findGroupMember(group conversation.Group, userID string) (conversation.Member, bool) {
	for _, member := range group.Members {
		if member.UserID == userID {
			return member, true
		}
	}
	return conversation.Member{}, false
}

func groupOwnerID(group conversation.Group) string {
	for _, member := range group.Members {
		if member.Role == conversation.RoleOwner {
			return member.UserID
		}
	}
	return ""
}

func groupApproverIDs(group conversation.Group) []string {
	approverIDs := make([]string, 0, len(group.Members))
	for _, member := range group.Members {
		if member.Role == conversation.RoleOwner || member.Role == conversation.RoleAdmin {
			approverIDs = append(approverIDs, member.UserID)
		}
	}
	return approverIDs
}

func memberIDs(members []conversation.Member) []string {
	ids := make([]string, 0, len(members))
	for _, member := range members {
		ids = append(ids, member.UserID)
	}
	return ids
}
