package main

import (
	"context"
	"errors"
	applicationconversation "gin_server/application/conversation"
	applicationidentity "gin_server/application/identity"
	applicationnotification "gin_server/application/notification"
	"gin_server/application/search"
	"gin_server/application/social"
	"gin_server/config"
	"gin_server/domain/conversation"
	"gin_server/domain/identity"
	domainnotification "gin_server/domain/notification"
	"gin_server/domain/shared"
	"gin_server/infrastructure/assets"
	"gin_server/infrastructure/logging"
	"gin_server/infrastructure/memory"
	"gin_server/infrastructure/password"
	"gin_server/infrastructure/realtime"
	persistence "gin_server/infrastructure/sql"
	"gin_server/infrastructure/token"
	transport "gin_server/interfaces/http"
	transportwebsocket "gin_server/interfaces/websocket"
	"io"
	"log"
	"os"
	"path/filepath"
)

const (
	defaultAddress = "0.0.0.0:34332"
	defaultBaseURL = "http://127.0.0.1:34332"
	jwtSecret      = "gpui-chat-local-development-secret-2026"
)

func main() {
	workingDirectory, err := os.Getwd()
	if err != nil {
		log.Fatal(err)
	}
	config, err := config.Load(workingDirectory)
	if err != nil {
		log.Fatal(err)
	}
	fileHook := logging.NewFileHook(config.LoggerPath)
	defer func() {
		if closeErr := fileHook.Close(); closeErr != nil && !errors.Is(closeErr, io.ErrClosedPipe) {
			log.Println("close file hook failed:", closeErr)
		}
	}()

	stdWrite := io.MultiWriter(os.Stdout, fileHook)
	log.SetOutput(stdWrite)
	log.Printf("file logging enabled with prefix: %s", config.LoggerPath)

	staticDirectory := filepath.Join(config.ServerDirectory, "static")
	databaseConfig, err := persistence.LoadConfigFromEnvironment(config.ProjectDirectory)
	if err != nil {
		log.Fatal(err)
	}

	var users identity.Repository
	var groups conversation.Repository
	var notifications domainnotification.Repository
	var transactions shared.TransactionManager
	if databaseConfig.UsesMemory() {
		store := memory.NewStore()
		guardConfig, guardErr := memory.LoadCapacityGuardConfig()
		if guardErr != nil {
			log.Fatal(guardErr)
		}
		guardContext, cancelGuard := context.WithCancel(context.Background())
		defer cancelGuard()
		store.StartCapacityGuard(guardContext, guardConfig)
		users = store
		groups = memory.NewGroupRepository(store)
		notifications = memory.NewNotificationRepository(store)
		transactions = store
	} else {
		store, openErr := persistence.NewStore(databaseConfig)
		if openErr != nil {
			log.Fatal(openErr)
		}
		defer func() {
			if closeErr := store.Close(); closeErr != nil {
				log.Printf("close database failed: %v", closeErr)
			}
		}()
		users = store
		groups = store.GroupRepository()
		notifications = store.NotificationRepository()
		transactions = store
	}

	pubSub := realtime.NewPubSub()
	hub := realtime.NewHub(pubSub)
	notifier := realtime.NewNotifier(pubSub)
	avatars := assets.NewAvatarCatalog(filepath.Join(staticDirectory, "user_avatar"), defaultBaseURL+"/assets/user_avatar")
	issuer, issuerErr := token.NewIssuer(jwtSecret)
	if issuerErr != nil {
		log.Fatal(issuerErr)
	}
	passwords := password.NewHasher()
	identityService := applicationidentity.NewService(users, issuer, avatars, passwords, transactions)
	if databaseConfig.Driver == persistence.DriverSQLite && config.BootstrapRootEnabled {
		created, rootErr := identityService.EnsureBootstrapRoot(context.Background(), config.BootstrapRootUsername, config.BootstrapRootPassword)
		if rootErr != nil {
			log.Fatal(rootErr)
		}
		log.Printf("development bootstrap root account: username=%s password=%s created=%t", config.BootstrapRootUsername, config.BootstrapRootPassword, created)
	}
	conversationService := applicationconversation.NewService(users, groups, notifications, notifier, transactions)
	notificationService := applicationnotification.NewService(notifications, users, groups, transactions)
	searchService := search.NewService(users, groups, transactions)
	socialService := social.NewService(users, groups, notifications, notifier, transactions)
	socketHandler := transportwebsocket.NewHandler(issuer, hub)

	log.Printf("chat server listening on %s with %s storage", defaultAddress, databaseConfig.Driver)
	if err := transport.NewRouter(identityService, conversationService, notificationService, searchService, socialService, issuer, socketHandler, staticDirectory).Run(defaultAddress); err != nil {
		log.Fatal(err)
	}
}
