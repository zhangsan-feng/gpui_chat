package http

import (
	"fmt"
	"image"
	_ "image/gif"
	_ "image/jpeg"
	"image/png"
	"mime/multipart"
	"os"
	"path/filepath"
	"strings"

	"github.com/google/uuid"
)

func saveUserAvatar(fileHeader *multipart.FileHeader, directory string) (string, string, error) {
	if fileHeader.Size > 5*1024*1024 {
		return "", "", fmt.Errorf("avatar file exceeds 5 MB")
	}

	extension := strings.ToLower(filepath.Ext(fileHeader.Filename))
	if !supportedAvatarExtension(extension) {
		return "", "", fmt.Errorf("unsupported avatar file format")
	}

	file, err := fileHeader.Open()
	if err != nil {
		return "", "", fmt.Errorf("open avatar file: %w", err)
	}
	defer file.Close()

	avatar, _, err := image.Decode(file)
	if err != nil {
		return "", "", fmt.Errorf("invalid avatar image: %w", err)
	}

	if err := os.MkdirAll(directory, 0o755); err != nil {
		return "", "", fmt.Errorf("create avatar directory: %w", err)
	}

	filename := uuid.NewString() + ".png"
	path := filepath.Join(directory, filename)
	output, err := os.Create(path)
	if err != nil {
		return "", "", fmt.Errorf("create avatar file: %w", err)
	}

	if err := png.Encode(output, avatar); err != nil {
		_ = output.Close()
		_ = os.Remove(path)
		return "", "", fmt.Errorf("save avatar image: %w", err)
	}
	if err := output.Close(); err != nil {
		_ = os.Remove(path)
		return "", "", fmt.Errorf("close avatar file: %w", err)
	}

	return path, filename, nil
}

func supportedAvatarExtension(extension string) bool {
	switch extension {
	case ".png", ".jpg", ".jpeg", ".gif", ".webp":
		return true
	default:
		return false
	}
}
