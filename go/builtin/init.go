package builtin

import (
	"fmt"
	"os"
)

func InitRepo() {
	for _, dir := range []string{".git", ".git/objects", ".git/refs", ".git/objects/pack"} {
		os.MkdirAll(dir, 0755)
	}
	os.WriteFile(".git/HEAD", []byte("ref: refs/heads/main\n"), 0644)
	fmt.Println("Initialized empty Git repository")
}
