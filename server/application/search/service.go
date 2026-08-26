package search

import (
	"context"
	"gin_server/domain/conversation"
	"gin_server/domain/identity"
	"gin_server/domain/shared"
	"strings"
)

type Result struct {
	Users  []identity.User      `json:"users"`
	Groups []conversation.Group `json:"groups"`
}

type Service struct {
	users        identity.Repository
	groups       conversation.Repository
	transactions shared.TransactionManager
}

func NewService(users identity.Repository, groups conversation.Repository, transactions shared.TransactionManager) *Service {
	return &Service{users: users, groups: groups, transactions: transactions}
}

func (s *Service) Search(ctx context.Context, userID string, keyword string) (Result, error) {
	needle := strings.ToLower(strings.TrimSpace(keyword))
	if needle == "" {
		return Result{}, shared.ErrInvalidInput
	}

	var result Result
	err := s.transactions.Read(ctx, func(txCtx context.Context) error {
		users, listUsersErr := s.users.List(txCtx)
		if listUsersErr != nil {
			return listUsersErr
		}
		groups, listGroupsErr := s.groups.ListDiscoverable(txCtx)
		if listGroupsErr != nil {
			return listGroupsErr
		}
		joinedGroups, listJoinedGroupsErr := s.groups.ListForUser(txCtx, userID)
		if listJoinedGroupsErr != nil {
			return listJoinedGroupsErr
		}
		joinedGroupIDs := make(map[string]struct{}, len(joinedGroups))
		for _, group := range joinedGroups {
			joinedGroupIDs[group.ID] = struct{}{}
		}
		result = Result{Users: []identity.User{}, Groups: []conversation.Group{}}
		for _, user := range users {
			if user.ID == userID {
				continue
			}
			if matches(needle, user.ID, user.Username, user.LoginName) {
				result.Users = append(result.Users, user)
			}
		}
		for _, group := range groups {
			if _, joined := joinedGroupIDs[group.ID]; joined {
				continue
			}
			if matches(needle, group.ID, group.Name) {
				result.Groups = append(result.Groups, group)
			}
		}
		return nil
	})
	return result, err
}

func matches(needle string, values ...string) bool {
	for _, value := range values {
		if strings.Contains(strings.ToLower(value), needle) {
			return true
		}
	}
	return false
}
