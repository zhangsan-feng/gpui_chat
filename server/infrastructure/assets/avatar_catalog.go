package assets

import (
	"os"
	"path/filepath"
	"sync"
)

type AvatarCatalog struct {
	mu      sync.Mutex
	avatars []string
	next    int
}

func NewAvatarCatalog(directory string, baseURL string) *AvatarCatalog {
	entries, err := os.ReadDir(directory)
	if err != nil {
		return &AvatarCatalog{avatars: []string{}}
	}
	avatars := make([]string, 0, len(entries))
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		avatars = append(avatars, baseURL+"/"+filepath.ToSlash(entry.Name()))
	}
	return &AvatarCatalog{avatars: avatars}
}

func (c *AvatarCatalog) Next() string {
	c.mu.Lock()
	defer c.mu.Unlock()
	if len(c.avatars) == 0 {
		return ""
	}
	avatar := c.avatars[c.next%len(c.avatars)]
	c.next++
	return avatar
}
