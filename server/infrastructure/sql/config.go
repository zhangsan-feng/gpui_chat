package sql

import (
	"fmt"
	"gin_server/infrastructure/sql/db"
	"gin_server/infrastructure/sql/driver"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

const (
	DriverMemory   = "memory"
	DriverSQLite   = "sqlite"
	DriverMySQL    = "mysql"
	DriverPostgres = "postgres"
)

type Config struct {
	Driver             string
	DSN                string
	MaxOpenConnections int
	MaxIdleConnections int
	ConnectionLifetime time.Duration
}

func LoadConfigFromEnvironment(projectDirectory string) (Config, error) {
	if strings.TrimSpace(projectDirectory) == "" {
		return Config{}, fmt.Errorf("project directory is required")
	}
	projectDirectory = filepath.Clean(projectDirectory)
	config := Config{
		Driver:             normalizeDriver(os.Getenv("CHAT_DATABASE_DRIVER")),
		DSN:                strings.TrimSpace(os.Getenv("CHAT_DATABASE_DSN")),
		MaxOpenConnections: 20,
		MaxIdleConnections: 5,
		ConnectionLifetime: 30 * time.Minute,
	}
	if config.Driver == DriverSQLite && config.DSN == "" {
		config.DSN = filepath.Join(projectDirectory, "data", "server", "chat.sqlite3")
	} else if config.Driver == DriverSQLite && !filepath.IsAbs(config.DSN) {
		config.DSN = filepath.Join(projectDirectory, config.DSN)
	}
	if err := applyPositiveInteger(&config.MaxOpenConnections, "CHAT_DATABASE_MAX_OPEN_CONNECTIONS"); err != nil {
		return Config{}, err
	}
	if err := applyPositiveInteger(&config.MaxIdleConnections, "CHAT_DATABASE_MAX_IDLE_CONNECTIONS"); err != nil {
		return Config{}, err
	}
	if value := strings.TrimSpace(os.Getenv("CHAT_DATABASE_CONNECTION_LIFETIME")); value != "" {
		lifetime, err := time.ParseDuration(value)
		if err != nil || lifetime <= 0 {
			return Config{}, fmt.Errorf("invalid CHAT_DATABASE_CONNECTION_LIFETIME: %q", value)
		}
		config.ConnectionLifetime = lifetime
	}
	if !config.UsesMemory() && config.DSN == "" {
		return Config{}, fmt.Errorf("CHAT_DATABASE_DSN is required for %s", config.Driver)
	}
	return config, nil
}

func (c Config) UsesMemory() bool {
	return c.Driver == DriverMemory
}

func NewStore(config Config) (*db.Store, error) {
	if config.UsesMemory() {
		return nil, fmt.Errorf("memory does not use a SQL store")
	}
	if config.Driver == DriverSQLite {
		if directory := filepath.Dir(config.DSN); directory != "." {
			if err := os.MkdirAll(directory, 0755); err != nil {
				return nil, fmt.Errorf("create sqlite directory: %w", err)
			}
		}
	}
	database, err := driver.Open(config.Driver, config.DSN)
	if err != nil {
		return nil, err
	}
	pool, err := database.DB()
	if err != nil {
		return nil, err
	}
	pool.SetMaxOpenConns(config.MaxOpenConnections)
	pool.SetMaxIdleConns(config.MaxIdleConnections)
	pool.SetConnMaxLifetime(config.ConnectionLifetime)
	return db.NewStore(database)
}

func normalizeDriver(value string) string {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "", DriverSQLite:
		return DriverSQLite
	case DriverMemory:
		return DriverMemory
	case DriverMySQL:
		return DriverMySQL
	case DriverPostgres, "postgresql", "pg":
		return DriverPostgres
	default:
		return strings.ToLower(strings.TrimSpace(value))
	}
}

func applyPositiveInteger(target *int, environmentName string) error {
	value := strings.TrimSpace(os.Getenv(environmentName))
	if value == "" {
		return nil
	}
	parsed, err := strconv.Atoi(value)
	if err != nil || parsed <= 0 {
		return fmt.Errorf("invalid %s: %q", environmentName, value)
	}
	*target = parsed
	return nil
}
