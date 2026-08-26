package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type Config struct {
	ProjectDirectory      string
	ServerDirectory       string
	LoggerPath            string
	BootstrapRootEnabled  bool
	BootstrapRootUsername string
	BootstrapRootPassword string
}

func Load(workingDirectory string) (Config, error) {
	if strings.TrimSpace(workingDirectory) == "" {
		return Config{}, fmt.Errorf("working directory is required")
	}

	workingDirectory = filepath.Clean(workingDirectory)
	projectDirectory := findProjectDirectory(workingDirectory)
	serverDirectory := filepath.Join(projectDirectory, "server")
	if !directoryExists(serverDirectory) {
		serverDirectory = workingDirectory
	}

	return Config{
		ProjectDirectory:      projectDirectory,
		ServerDirectory:       serverDirectory,
		LoggerPath:            filepath.Join(projectDirectory, "logs", "server", "server_"),
		BootstrapRootEnabled:  true,
		BootstrapRootUsername: "root",
		BootstrapRootPassword: "root123456",
	}, nil
}

func findProjectDirectory(startDirectory string) string {
	currentDirectory := startDirectory
	for {
		if directoryExists(filepath.Join(currentDirectory, "server")) &&
			directoryExists(filepath.Join(currentDirectory, "gui")) {
			return currentDirectory
		}

		parentDirectory := filepath.Dir(currentDirectory)
		if parentDirectory == currentDirectory {
			return startDirectory
		}
		currentDirectory = parentDirectory
	}
}

func directoryExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.IsDir()
}
