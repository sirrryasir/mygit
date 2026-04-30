package builtin

import (
	"fmt"
	"mygit-go/core"
)

func AddFiles(files []string) {
	for _, f := range files {
		core.HashObject(f, true)
	}
	fmt.Printf("Added %d files\n", len(files))
}
