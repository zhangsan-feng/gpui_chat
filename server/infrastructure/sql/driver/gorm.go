package driver

import (
	"fmt"

	"github.com/glebarez/sqlite"
	"gorm.io/driver/mysql"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

func Open(name string, dsn string) (*gorm.DB, error) {
	switch name {
	case "sqlite":
		return gorm.Open(sqlite.Open(dsn), &gorm.Config{TranslateError: true})
	case "mysql":
		return gorm.Open(mysql.Open(dsn), &gorm.Config{TranslateError: true})
	case "postgres":
		return gorm.Open(postgres.Open(dsn), &gorm.Config{TranslateError: true})
	default:
		return nil, fmt.Errorf("unsupported SQL driver: %s", name)
	}
}
