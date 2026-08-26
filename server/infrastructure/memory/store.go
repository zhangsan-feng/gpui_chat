package memory

import (
	"context"
	"gin_server/domain/conversation"
	"gin_server/domain/identity"
	"gin_server/domain/notification"
	"gin_server/domain/shared"
	"sort"
	"sync"
)

type Store struct {
	mu            sync.RWMutex
	transactionMu sync.Mutex
	users         map[string]identity.User
	groups        map[string]conversation.Group
	friendships   map[string]struct{}
	notifications map[string]notification.Notification
}

type transactionContextKey struct{}

func (s *Store) Execute(ctx context.Context, operation func(context.Context) error) error {
	s.transactionMu.Lock()
	defer s.transactionMu.Unlock()

	s.mu.RLock()
	usersSnapshot := cloneUsers(s.users)
	groupsSnapshot := cloneGroups(s.groups)
	friendshipsSnapshot := cloneFriendships(s.friendships)
	notificationsSnapshot := cloneNotifications(s.notifications)
	s.mu.RUnlock()

	txCtx := context.WithValue(ctx, transactionContextKey{}, true)
	if err := operation(txCtx); err != nil {
		s.mu.Lock()
		s.users = usersSnapshot
		s.groups = groupsSnapshot
		s.friendships = friendshipsSnapshot
		s.notifications = notificationsSnapshot
		s.mu.Unlock()
		return err
	}
	return nil
}

func (s *Store) Read(ctx context.Context, operation func(context.Context) error) error {
	s.transactionMu.Lock()
	defer s.transactionMu.Unlock()
	return operation(context.WithValue(ctx, transactionContextKey{}, true))
}

func (s *Store) lockOperation(ctx context.Context) func() {
	if inTransaction, _ := ctx.Value(transactionContextKey{}).(bool); inTransaction {
		return func() {}
	}
	s.transactionMu.Lock()
	return s.transactionMu.Unlock
}

func NewStore() *Store {
	return &Store{
		users:         make(map[string]identity.User),
		groups:        make(map[string]conversation.Group),
		friendships:   make(map[string]struct{}),
		notifications: make(map[string]notification.Notification),
	}
}

func (s *Store) FindByID(ctx context.Context, id string) (identity.User, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	user, exists := s.users[id]
	if !exists {
		return identity.User{}, shared.ErrNotFound
	}
	return s.hydrateUserLocked(user), nil
}

func (s *Store) FindByLoginName(ctx context.Context, loginName string) (identity.User, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	for _, user := range s.users {
		if user.LoginName == loginName {
			return s.hydrateUserLocked(user), nil
		}
	}
	return identity.User{}, shared.ErrNotFound
}

func (s *Store) Save(ctx context.Context, user identity.User) error {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.Lock()
	defer s.mu.Unlock()
	s.users[user.ID] = cloneUser(user)
	return nil
}

func (s *Store) List(ctx context.Context) ([]identity.User, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	users := make([]identity.User, 0, len(s.users))
	for _, user := range s.users {
		users = append(users, s.hydrateUserLocked(user))
	}
	return users, nil
}

func (s *Store) AreFriends(ctx context.Context, userID string, friendID string) (bool, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	_, exists := s.friendships[friendshipKey(userID, friendID)]
	return exists, nil
}

func (s *Store) CreateFriendship(ctx context.Context, userID string, friendID string) error {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.Lock()
	defer s.mu.Unlock()
	s.friendships[friendshipKey(userID, friendID)] = struct{}{}
	return nil
}

func (s *Store) RemoveFriendship(ctx context.Context, userID string, friendID string) error {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.friendships, friendshipKey(userID, friendID))
	return nil
}

func (s *Store) FindByIDGroup(ctx context.Context, id string) (conversation.Group, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	group, exists := s.groups[id]
	if !exists {
		return conversation.Group{}, shared.ErrNotFound
	}
	return cloneGroup(group), nil
}

func (s *Store) SaveGroup(ctx context.Context, group conversation.Group) error {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.Lock()
	defer s.mu.Unlock()
	s.groups[group.ID] = cloneGroup(group)
	return nil
}

func (s *Store) DeleteGroup(ctx context.Context, id string) error {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.Lock()
	defer s.mu.Unlock()
	if _, exists := s.groups[id]; !exists {
		return shared.ErrNotFound
	}
	delete(s.groups, id)
	return nil
}

func (s *Store) ListGroups(ctx context.Context) ([]conversation.Group, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	groups := make([]conversation.Group, 0, len(s.groups))
	for _, group := range s.groups {
		groups = append(groups, cloneGroup(group))
	}
	return groups, nil
}

func (s *Store) ListDiscoverableGroups(ctx context.Context) ([]conversation.Group, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	groups := make([]conversation.Group, 0)
	for _, group := range s.groups {
		if group.Kind != "group" || !group.AllowJoin {
			continue
		}
		groups = append(groups, conversation.Group{ID: group.ID, Name: group.Name, Avatar: group.Avatar, Kind: group.Kind, AllowJoin: true, Members: []conversation.Member{}, Messages: []conversation.Message{}})
	}
	sort.Slice(groups, func(left int, right int) bool {
		return groups[left].Name < groups[right].Name
	})
	return groups, nil
}

func (s *Store) ListGroupsForUser(ctx context.Context, userID string) ([]conversation.Group, error) {
	unlock := s.lockOperation(ctx)
	defer unlock()
	s.mu.RLock()
	defer s.mu.RUnlock()
	groups := make([]conversation.Group, 0)
	for _, group := range s.groups {
		if !groupHasMember(group, userID) {
			continue
		}
		groups = append(groups, cloneGroup(group))
	}
	sort.Slice(groups, func(left int, right int) bool {
		return groups[left].Name < groups[right].Name
	})
	return groups, nil
}

type groupRepository struct {
	store *Store
}

func NewGroupRepository(store *Store) conversation.Repository {
	return groupRepository{store: store}
}

func (r groupRepository) FindByID(ctx context.Context, id string) (conversation.Group, error) {
	return r.store.FindByIDGroup(ctx, id)
}

func (r groupRepository) Save(ctx context.Context, group conversation.Group) error {
	return r.store.SaveGroup(ctx, group)
}

func (r groupRepository) Delete(ctx context.Context, id string) error {
	return r.store.DeleteGroup(ctx, id)
}

func (r groupRepository) List(ctx context.Context) ([]conversation.Group, error) {
	return r.store.ListGroups(ctx)
}

func (r groupRepository) ListDiscoverable(ctx context.Context) ([]conversation.Group, error) {
	return r.store.ListDiscoverableGroups(ctx)
}

func (r groupRepository) ListForUser(ctx context.Context, userID string) ([]conversation.Group, error) {
	return r.store.ListGroupsForUser(ctx, userID)
}

func cloneUser(user identity.User) identity.User {
	clone := user
	clone.ConversationIDs = append([]string{}, user.ConversationIDs...)
	clone.Friends = append([]identity.Friend{}, user.Friends...)
	return clone
}

func cloneGroup(group conversation.Group) conversation.Group {
	clone := group
	clone.Members = append([]conversation.Member{}, group.Members...)
	clone.Messages = make([]conversation.Message, len(group.Messages))
	for index, message := range group.Messages {
		clone.Messages[index] = message
		clone.Messages[index].Attachments = append([]conversation.Attachment{}, message.Attachments...)
	}
	return clone
}

func cloneUsers(users map[string]identity.User) map[string]identity.User {
	clone := make(map[string]identity.User, len(users))
	for id, user := range users {
		clone[id] = cloneUser(user)
	}
	return clone
}

func cloneGroups(groups map[string]conversation.Group) map[string]conversation.Group {
	clone := make(map[string]conversation.Group, len(groups))
	for id, group := range groups {
		clone[id] = cloneGroup(group)
	}
	return clone
}

func (s *Store) hydrateUserLocked(user identity.User) identity.User {
	clone := cloneUser(user)
	clone.Friends = make([]identity.Friend, 0)
	for _, candidate := range s.users {
		if candidate.ID == user.ID {
			continue
		}
		if _, exists := s.friendships[friendshipKey(user.ID, candidate.ID)]; !exists {
			continue
		}
		clone.Friends = append(clone.Friends, identity.Friend{ID: candidate.ID, Username: candidate.Username, Avatar: candidate.Avatar})
	}
	sort.Slice(clone.Friends, func(left int, right int) bool {
		return clone.Friends[left].Username < clone.Friends[right].Username
	})
	clone.ConversationIDs = make([]string, 0)
	for _, group := range s.groups {
		if groupHasMember(group, user.ID) {
			clone.ConversationIDs = append(clone.ConversationIDs, group.ID)
		}
	}
	sort.Strings(clone.ConversationIDs)
	return clone
}

func groupHasMember(group conversation.Group, userID string) bool {
	for _, member := range group.Members {
		if member.UserID == userID {
			return true
		}
	}
	return false
}

func friendshipKey(userID string, friendID string) string {
	if userID < friendID {
		return userID + "\x00" + friendID
	}
	return friendID + "\x00" + userID
}

func cloneFriendships(friendships map[string]struct{}) map[string]struct{} {
	clone := make(map[string]struct{}, len(friendships))
	for key := range friendships {
		clone[key] = struct{}{}
	}
	return clone
}

func cloneNotifications(notifications map[string]notification.Notification) map[string]notification.Notification {
	clone := make(map[string]notification.Notification, len(notifications))
	for id, item := range notifications {
		clone[id] = item
	}
	return clone
}
