package shared

import "context"

type TransactionManager interface {
	Execute(ctx context.Context, operation func(context.Context) error) error
	Read(ctx context.Context, operation func(context.Context) error) error
}
