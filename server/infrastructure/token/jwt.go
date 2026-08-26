package token

import (
	"context"
	"errors"
	"gin_server/domain/identity"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
)

type Issuer struct {
	secret []byte
}

type Claims struct {
	UserID    string `json:"user_id"`
	Username  string `json:"username"`
	TokenType string `json:"token_type"`
	jwt.RegisteredClaims
}

const (
	accessTokenType  = "access"
	refreshTokenType = "refresh"
	accessTokenTTL   = 15 * time.Minute
	refreshTokenTTL  = 30 * 24 * time.Hour
)

func NewIssuer(secret string) (*Issuer, error) {
	if len(secret) < 32 {
		return nil, errors.New("CHAT_JWT_SECRET must contain at least 32 characters")
	}
	return &Issuer{secret: []byte(secret)}, nil
}

func (i *Issuer) Issue(_ context.Context, user identity.User) (identity.TokenPair, error) {
	now := time.Now()
	accessExpiresAt := now.Add(accessTokenTTL)
	refreshExpiresAt := now.Add(refreshTokenTTL)
	accessToken, err := i.issue(user, accessTokenType, accessExpiresAt)
	if err != nil {
		return identity.TokenPair{}, err
	}
	refreshToken, err := i.issue(user, refreshTokenType, refreshExpiresAt)
	if err != nil {
		return identity.TokenPair{}, err
	}
	return identity.TokenPair{
		AccessToken:           accessToken,
		RefreshToken:          refreshToken,
		AccessTokenExpiresAt:  accessExpiresAt.Unix(),
		RefreshTokenExpiresAt: refreshExpiresAt.Unix(),
	}, nil
}

func (i *Issuer) Authenticate(rawToken string) (string, error) {
	claims, err := i.parse(rawToken)
	if err != nil || claims.UserID == "" || (claims.TokenType != "" && claims.TokenType != accessTokenType) {
		return "", errors.New("invalid token")
	}
	return claims.UserID, nil
}

func (i *Issuer) Refresh(rawToken string) (identity.RefreshTokenClaims, error) {
	claims, err := i.parse(rawToken)
	if err != nil || claims.UserID == "" || claims.TokenType != refreshTokenType {
		return identity.RefreshTokenClaims{}, errors.New("invalid refresh token")
	}
	return identity.RefreshTokenClaims{UserID: claims.UserID, Username: claims.Username}, nil
}

func (i *Issuer) issue(user identity.User, tokenType string, expiresAt time.Time) (string, error) {
	claims := Claims{
		UserID: user.ID, Username: user.Username, TokenType: tokenType,
		RegisteredClaims: jwt.RegisteredClaims{
			ID: uuid.NewString(), ExpiresAt: jwt.NewNumericDate(expiresAt),
		},
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS512, claims).SignedString(i.secret)
}

func (i *Issuer) parse(rawToken string) (Claims, error) {
	claims := Claims{}
	parsed, err := jwt.ParseWithClaims(rawToken, &claims, func(token *jwt.Token) (any, error) {
		if token.Method.Alg() != jwt.SigningMethodHS512.Alg() {
			return nil, errors.New("unexpected signing method")
		}
		return i.secret, nil
	})
	if err != nil || !parsed.Valid {
		return Claims{}, errors.New("invalid token")
	}
	return claims, nil
}
