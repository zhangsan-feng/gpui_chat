package identity

import (
	"context"
	"errors"
	"gin_server/domain/identity"
	"gin_server/domain/shared"
	"strings"

	"github.com/google/uuid"
)

type TokenIssuer interface {
	Issue(ctx context.Context, user identity.User) (identity.TokenPair, error)
	Refresh(rawToken string) (identity.RefreshTokenClaims, error)
}

type AvatarProvider interface {
	Next() string
}

type PasswordHasher interface {
	Hash(value string) (string, error)
	Verify(hash string, value string) error
}

type Service struct {
	users        identity.Repository
	tokens       TokenIssuer
	avatars      AvatarProvider
	passwords    PasswordHasher
	transactions shared.TransactionManager
}

type LoginCommand struct {
	LoginName string
	Password  string
}

type RegisterCommand struct {
	Username  string
	LoginName string
	Password  string
}

type UpdateProfileCommand struct {
	UserID          string
	Username        string
	LoginName       string
	CurrentPassword string
	NewPassword     string
	Avatar          string
}

type LoginResult struct {
	User identity.User `json:"user"`
	identity.TokenPair
}

type RefreshCommand struct {
	RefreshToken string
}

const bootstrapRootID = "root"

func NewService(users identity.Repository, tokens TokenIssuer, avatars AvatarProvider, passwords PasswordHasher, transactions shared.TransactionManager) *Service {
	return &Service{users: users, tokens: tokens, avatars: avatars, passwords: passwords, transactions: transactions}
}

func (s *Service) Login(ctx context.Context, command LoginCommand) (LoginResult, error) {
	var result LoginResult
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		loginName := strings.TrimSpace(command.LoginName)
		if !validLoginName(loginName) || command.Password == "" || s.passwords == nil {
			return shared.ErrInvalidInput
		}

		user, findErr := s.users.FindByLoginName(txCtx, loginName)
		if errors.Is(findErr, shared.ErrNotFound) {
			return shared.ErrUnauthorized
		}
		if findErr != nil {
			return findErr
		} else if user.PasswordHash == "" || s.passwords.Verify(user.PasswordHash, command.Password) != nil {
			return shared.ErrUnauthorized
		}

		tokens, issueErr := s.tokens.Issue(txCtx, user)
		if issueErr != nil {
			return issueErr
		}
		result = LoginResult{User: user, TokenPair: tokens}
		return nil
	})
	return result, err
}

func (s *Service) Register(ctx context.Context, command RegisterCommand) (LoginResult, error) {
	var result LoginResult
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		username := strings.TrimSpace(command.Username)
		loginName := strings.TrimSpace(command.LoginName)
		if !validLoginName(loginName) || command.Password == "" || s.passwords == nil {
			return shared.ErrInvalidInput
		}
		if username == "" {
			username = uuid.NewString()
		}

		_, findErr := s.users.FindByLoginName(txCtx, loginName)
		switch {
		case findErr == nil:
			return shared.ErrConflict
		case !errors.Is(findErr, shared.ErrNotFound):
			return findErr
		}

		user, createErr := s.newUser(username, loginName, command.Password)
		if createErr != nil {
			return createErr
		}
		if saveErr := s.users.Save(txCtx, user); saveErr != nil {
			return saveErr
		}

		tokens, issueErr := s.tokens.Issue(txCtx, user)
		if issueErr != nil {
			return issueErr
		}
		result = LoginResult{User: user, TokenPair: tokens}
		return nil
	})
	return result, err
}

func (s *Service) UpdateProfile(ctx context.Context, command UpdateProfileCommand) (LoginResult, error) {
	var result LoginResult
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		username := strings.TrimSpace(command.Username)
		if command.UserID == "" || username == "" || s.passwords == nil {
			return shared.ErrInvalidInput
		}

		user, findErr := s.users.FindByID(txCtx, command.UserID)
		if errors.Is(findErr, shared.ErrNotFound) {
			return shared.ErrUnauthorized
		}
		if findErr != nil {
			return findErr
		}

		if command.NewPassword != "" {
			if command.CurrentPassword == "" || user.PasswordHash == "" ||
				s.passwords.Verify(user.PasswordHash, command.CurrentPassword) != nil {
				return shared.ErrForbidden
			}
			if len(command.NewPassword) < 6 {
				return shared.ErrInvalidInput
			}
			passwordHash, hashErr := s.passwords.Hash(command.NewPassword)
			if hashErr != nil {
				return hashErr
			}
			user.PasswordHash = passwordHash
		} else if command.CurrentPassword != "" {
			return shared.ErrInvalidInput
		}

		user.Username = username
		if user.LoginName == "" && validLoginName(command.LoginName) {
			user.LoginName = strings.TrimSpace(command.LoginName)
		}
		if command.Avatar != "" {
			user.Avatar = command.Avatar
		}
		if saveErr := s.users.Save(txCtx, user); saveErr != nil {
			return saveErr
		}

		tokens, issueErr := s.tokens.Issue(txCtx, user)
		if issueErr != nil {
			return issueErr
		}
		result = LoginResult{User: user, TokenPair: tokens}
		return nil
	})
	return result, err
}

func (s *Service) Refresh(ctx context.Context, command RefreshCommand) (LoginResult, error) {
	if strings.TrimSpace(command.RefreshToken) == "" {
		return LoginResult{}, shared.ErrInvalidInput
	}

	claims, err := s.tokens.Refresh(command.RefreshToken)
	if err != nil || claims.UserID == "" {
		return LoginResult{}, shared.ErrUnauthorized
	}

	var result LoginResult
	err = s.transactions.Execute(ctx, func(txCtx context.Context) error {
		user, findErr := s.users.FindByID(txCtx, claims.UserID)
		if errors.Is(findErr, shared.ErrNotFound) {
			return shared.ErrUnauthorized
		}
		if findErr != nil {
			return findErr
		}

		tokens, issueErr := s.tokens.Issue(txCtx, user)
		if issueErr != nil {
			return issueErr
		}
		result = LoginResult{User: user, TokenPair: tokens}
		return nil
	})
	return result, err
}

func (s *Service) EnsureBootstrapRoot(ctx context.Context, username string, password string) (bool, error) {
	username = strings.TrimSpace(username)
	if username == "" || password == "" || s.passwords == nil {
		return false, shared.ErrInvalidInput
	}

	created := false
	err := s.transactions.Execute(ctx, func(txCtx context.Context) error {
		if _, err := s.users.FindByID(txCtx, bootstrapRootID); err == nil {
			return nil
		} else if !errors.Is(err, shared.ErrNotFound) {
			return err
		}

		if _, err := s.users.FindByLoginName(txCtx, username); err == nil {
			return nil
		} else if !errors.Is(err, shared.ErrNotFound) {
			return err
		}

		passwordHash, err := s.passwords.Hash(password)
		if err != nil {
			return err
		}
		user := identity.User{
			ID:              bootstrapRootID,
			Username:        username,
			LoginName:       username,
			Status:          "online",
			PasswordHash:    passwordHash,
			ConversationIDs: []string{},
			Friends:         []identity.Friend{},
		}
		if err := s.users.Save(txCtx, user); err != nil {
			return err
		}
		created = true
		return nil
	})
	return created, err
}

func (s *Service) GetUser(ctx context.Context, id string) (identity.User, error) {
	return s.users.FindByID(ctx, id)
}

func (s *Service) newUser(username string, loginName string, password string) (identity.User, error) {
	avatar := ""
	if s.avatars != nil {
		avatar = s.avatars.Next()
	}
	passwordHash, err := s.passwords.Hash(password)
	if err != nil {
		return identity.User{}, err
	}
	return identity.User{
		ID:              uuid.NewString(),
		Username:        username,
		LoginName:       loginName,
		Avatar:          avatar,
		Status:          "online",
		PasswordHash:    passwordHash,
		ConversationIDs: []string{},
		Friends:         []identity.Friend{},
	}, nil
}

func validLoginName(loginName string) bool {
	if loginName == "" || strings.TrimSpace(loginName) != loginName || loginName == "." || loginName == ".." {
		return false
	}
	for _, character := range loginName {
		switch character {
		case '<', '>', ':', '"', '/', '\\', '|', '?', '*':
			return false
		}
	}
	return true
}
