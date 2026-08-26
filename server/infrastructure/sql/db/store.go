package db

import (
	"context"
	stdsql "database/sql"
	"errors"
	"gin_server/domain/shared"

	"gorm.io/gorm"
	"gorm.io/gorm/clause"
)

type Store struct {
	database *gorm.DB
}

type transactionContextKey struct{}
type transactionModeKey struct{}

const writeTransaction = "write"

func NewStore(database *gorm.DB) (*Store, error) {
	if database == nil {
		return nil, errors.New("gorm database is required")
	}
	if err := database.AutoMigrate(
		&userRecord{}, &friendshipRecord{}, &roomRecord{}, &groupRecord{}, &memberRecord{},
		&conversationRecord{}, &messageRecord{}, &attachmentRecord{}, &notificationRecord{},
	); err != nil {
		return nil, err
	}
	return &Store{database: database}, nil
}

func (s *Store) Execute(ctx context.Context, operation func(context.Context) error) error {
	if _, exists := ctx.Value(transactionContextKey{}).(*gorm.DB); exists {
		return operation(ctx)
	}
	return s.database.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		txCtx := context.WithValue(ctx, transactionContextKey{}, tx)
		txCtx = context.WithValue(txCtx, transactionModeKey{}, writeTransaction)
		return operation(txCtx)
	}, &stdsql.TxOptions{Isolation: stdsql.LevelReadCommitted})
}

func (s *Store) Read(ctx context.Context, operation func(context.Context) error) error {
	if _, exists := ctx.Value(transactionContextKey{}).(*gorm.DB); exists {
		return operation(ctx)
	}
	return s.database.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		txCtx := context.WithValue(ctx, transactionContextKey{}, tx)
		return operation(txCtx)
	}, &stdsql.TxOptions{Isolation: stdsql.LevelRepeatableRead, ReadOnly: true})
}

func (s *Store) Close() error {
	pool, err := s.database.DB()
	if err != nil {
		return err
	}
	return pool.Close()
}

func (s *Store) databaseFor(ctx context.Context) *gorm.DB {
	if transaction, exists := ctx.Value(transactionContextKey{}).(*gorm.DB); exists {
		return transaction
	}
	return s.database.WithContext(ctx)
}

func (s *Store) queryFor(ctx context.Context) *gorm.DB {
	database := s.databaseFor(ctx)
	if ctx.Value(transactionModeKey{}) == writeTransaction {
		return database.Clauses(clause.Locking{Strength: "UPDATE"})
	}
	return database
}

func normalizeError(err error) error {
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return shared.ErrNotFound
	}
	if errors.Is(err, gorm.ErrDuplicatedKey) {
		return shared.ErrConflict
	}
	return err
}
