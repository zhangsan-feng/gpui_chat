package safety

import (
	"log"
	"runtime/debug"
)

func Go(name string, task func()) {
	go func() {
		defer func() {
			if recovered := recover(); recovered != nil {
				log.Printf("goroutine %s panic: %v\n%s", name, recovered, debug.Stack())
			}
		}()

		task()
	}()
}
