package memory

import (
	"context"
	"fmt"
	"gin_server/infrastructure/safety"
	"log"
	"math"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/shirou/gopsutil/v4/mem"
)

const (
	defaultMemoryThresholdPercent = 80.0
	defaultPurgeRatio             = 0.5
	defaultMemoryCheckInterval    = 15 * time.Second
)

type CapacityGuardConfig struct {
	Enabled          bool
	ThresholdPercent float64
	PurgeRatio       float64
	CheckInterval    time.Duration
}

func LoadCapacityGuardConfig() (CapacityGuardConfig, error) {
	config := CapacityGuardConfig{
		Enabled:          true,
		ThresholdPercent: defaultMemoryThresholdPercent,
		PurgeRatio:       defaultPurgeRatio,
		CheckInterval:    defaultMemoryCheckInterval,
	}
	if value := strings.TrimSpace(os.Getenv("CHAT_MEMORY_GUARD_ENABLED")); value != "" {
		enabled, err := strconv.ParseBool(value)
		if err != nil {
			return CapacityGuardConfig{}, fmt.Errorf("invalid CHAT_MEMORY_GUARD_ENABLED: %q", value)
		}
		config.Enabled = enabled
	}
	if err := applyFloat(&config.ThresholdPercent, "CHAT_MEMORY_THRESHOLD_PERCENT"); err != nil {
		return CapacityGuardConfig{}, err
	}
	if err := applyFloat(&config.PurgeRatio, "CHAT_MEMORY_PURGE_RATIO"); err != nil {
		return CapacityGuardConfig{}, err
	}
	if value := strings.TrimSpace(os.Getenv("CHAT_MEMORY_CHECK_INTERVAL")); value != "" {
		interval, err := time.ParseDuration(value)
		if err != nil || interval <= 0 {
			return CapacityGuardConfig{}, fmt.Errorf("invalid CHAT_MEMORY_CHECK_INTERVAL: %q", value)
		}
		config.CheckInterval = interval
	}
	if !isFinite(config.ThresholdPercent) || config.ThresholdPercent <= 0 || config.ThresholdPercent > 100 {
		return CapacityGuardConfig{}, fmt.Errorf("CHAT_MEMORY_THRESHOLD_PERCENT must be between 0 and 100")
	}
	if !isFinite(config.PurgeRatio) || config.PurgeRatio <= 0 || config.PurgeRatio > 1 {
		return CapacityGuardConfig{}, fmt.Errorf("CHAT_MEMORY_PURGE_RATIO must be between 0 and 1")
	}
	return config, nil
}

func (s *Store) StartCapacityGuard(ctx context.Context, config CapacityGuardConfig) {
	if !config.Enabled {
		return
	}
	safety.Go("memory-capacity-guard", func() {
		s.runCapacityGuard(ctx, config)
	})
}

func (s *Store) runCapacityGuard(ctx context.Context, config CapacityGuardConfig) {
	ticker := time.NewTicker(config.CheckInterval)
	defer ticker.Stop()
	thresholdExceeded := false
	for {
		thresholdExceeded = s.enforceCapacity(config, thresholdExceeded)
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
	}
}

func (s *Store) enforceCapacity(config CapacityGuardConfig, thresholdExceeded bool) bool {
	usage, err := mem.VirtualMemory()
	if err != nil {
		log.Printf("memory guard failed to read machine memory: %v", err)
		return thresholdExceeded
	}
	if usage.UsedPercent < config.ThresholdPercent {
		return false
	}
	if thresholdExceeded {
		return true
	}
	result, err := s.PruneOldestMessages(config.PurgeRatio)
	if err != nil {
		log.Printf("memory guard failed to prune messages: %v", err)
		return false
	}
	if result.DeletedMessages > 0 {
		log.Printf(
			"memory guard pruned %d of %d messages at machine memory %.2f%%",
			result.DeletedMessages,
			result.TotalMessages,
			usage.UsedPercent,
		)
	}
	return true
}

func applyFloat(target *float64, environmentName string) error {
	value := strings.TrimSpace(os.Getenv(environmentName))
	if value == "" {
		return nil
	}
	parsed, err := strconv.ParseFloat(value, 64)
	if err != nil {
		return fmt.Errorf("invalid %s: %q", environmentName, value)
	}
	*target = parsed
	return nil
}

func isFinite(value float64) bool {
	return !math.IsNaN(value) && !math.IsInf(value, 0)
}
