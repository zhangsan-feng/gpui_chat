package logging

import (
	"gin_server/infrastructure/safety"
	"io"
	"log"
	"os"
	"path/filepath"
	"sync"
	"sync/atomic"
	"time"
)

func init() {
	log.SetFlags(log.LstdFlags | log.Lshortfile)
}

type FileHook struct {
	logPath          string
	ch               chan []byte
	closed           atomic.Bool
	once             sync.Once
	wg               sync.WaitGroup
	writeMu          sync.RWMutex
	currentFile      *os.File
	nextRotationUnix int64
}

var bufferPool = sync.Pool{
	New: func() any {
		return make([]byte, 0, 1024*8)
	},
}

func NewFileHook(logPath string) *FileHook {
	hook := &FileHook{
		ch:      make(chan []byte, 1024*8),
		logPath: logPath,
	}
	hook.wg.Add(1)
	safety.Go("file-log-processor", hook.processLogs)
	return hook
}

func (w *FileHook) Write(p []byte) (n int, err error) {
	if len(p) == 0 {
		return 0, nil
	}

	w.writeMu.RLock()
	defer w.writeMu.RUnlock()
	if w.closed.Load() {
		return 0, io.ErrClosedPipe
	}

	buf := bufferPool.Get().([]byte)
	if cap(buf) < len(p) {
		buf = make([]byte, len(p))
	} else {
		buf = buf[:len(p)]
	}
	copy(buf, p)

	select {
	case w.ch <- buf:
	default:
		bufferPool.Put(buf[:0])
	}

	return len(p), nil
}

func (w *FileHook) processLogs() {
	defer w.wg.Done()
	defer func() {
		if w.currentFile != nil {
			_ = w.currentFile.Close()
		}
	}()

	closeFile := func() {
		if w.currentFile != nil {
			_ = w.currentFile.Close()
			w.currentFile = nil
		}
	}

	openFileForNow := func(now time.Time) error {
		closeFile()

		today := now.Format("2006_01_02")
		filename := w.logPath + today + ".log"
		if directory := filepath.Dir(filename); directory != "." {
			if err := os.MkdirAll(directory, 0755); err != nil {
				return err
			}
		}

		file, err := os.OpenFile(filename, os.O_WRONLY|os.O_CREATE|os.O_APPEND, 0644)
		if err != nil {
			return err
		}

		w.currentFile = file
		nextDay := time.Date(now.Year(), now.Month(), now.Day()+1, 0, 0, 0, 0, now.Location())
		w.nextRotationUnix = nextDay.Unix()
		return nil
	}

	if err := openFileForNow(time.Now()); err != nil {
		_, _ = os.Stderr.WriteString("failed to open log file: " + err.Error() + "\n")
	}

	for p := range w.ch {
		now := time.Now()
		if w.currentFile == nil || now.Unix() >= w.nextRotationUnix {
			if err := openFileForNow(now); err != nil {
				_, _ = os.Stderr.WriteString("failed to open log file: " + err.Error() + "\n")
				bufferPool.Put(p[:0])
				continue
			}
		}

		if _, err := w.currentFile.Write(p); err != nil {
			_, _ = os.Stderr.WriteString("failed to write log file: " + err.Error() + "\n")
		}
		bufferPool.Put(p[:0])
	}
}

func (w *FileHook) Close() error {
	w.once.Do(func() {
		w.writeMu.Lock()
		w.closed.Store(true)
		close(w.ch)
		w.writeMu.Unlock()
		w.wg.Wait()
	})
	return nil
}
