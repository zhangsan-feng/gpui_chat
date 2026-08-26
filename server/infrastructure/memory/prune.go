package memory

import (
	"fmt"
	"gin_server/domain/conversation"
	"math"
	"sort"
)

type PruneResult struct {
	TotalMessages   int
	DeletedMessages int
	Remaining       int
}

type messageReference struct {
	GroupID   string
	Index     int
	CreatedAt int64
	ID        string
}

func (s *Store) PruneOldestMessages(ratio float64) (PruneResult, error) {
	if math.IsNaN(ratio) || math.IsInf(ratio, 0) || ratio <= 0 || ratio > 1 {
		return PruneResult{}, fmt.Errorf("message purge ratio must be between 0 and 1")
	}
	s.transactionMu.Lock()
	defer s.transactionMu.Unlock()
	s.mu.Lock()
	defer s.mu.Unlock()

	references := collectMessageReferences(s.groups)
	result := PruneResult{TotalMessages: len(references), Remaining: len(references)}
	if len(references) == 0 {
		return result, nil
	}
	sort.Slice(references, func(left int, right int) bool {
		if references[left].CreatedAt != references[right].CreatedAt {
			return references[left].CreatedAt < references[right].CreatedAt
		}
		if references[left].GroupID != references[right].GroupID {
			return references[left].GroupID < references[right].GroupID
		}
		return references[left].ID < references[right].ID
	})
	deleteCount := int(float64(len(references)) * ratio)
	if deleteCount == 0 {
		return result, nil
	}
	toDelete := make(map[string]map[int]struct{}, deleteCount)
	for _, reference := range references[:deleteCount] {
		if toDelete[reference.GroupID] == nil {
			toDelete[reference.GroupID] = make(map[int]struct{})
		}
		toDelete[reference.GroupID][reference.Index] = struct{}{}
	}
	for groupID, indexes := range toDelete {
		group := s.groups[groupID]
		group.Messages = removeMessageIndexes(group.Messages, indexes)
		s.groups[groupID] = group
	}
	result.DeletedMessages = deleteCount
	result.Remaining = result.TotalMessages - deleteCount
	return result, nil
}

func collectMessageReferences(groups map[string]conversation.Group) []messageReference {
	references := make([]messageReference, 0)
	for groupID, group := range groups {
		for index, message := range group.Messages {
			references = append(references, messageReference{
				GroupID: groupID, Index: index, CreatedAt: message.CreatedAt, ID: message.ID,
			})
		}
	}
	return references
}

func removeMessageIndexes(messages []conversation.Message, indexes map[int]struct{}) []conversation.Message {
	remaining := make([]conversation.Message, 0, len(messages)-len(indexes))
	for index, message := range messages {
		if _, remove := indexes[index]; remove {
			continue
		}
		remaining = append(remaining, message)
	}
	return remaining
}
