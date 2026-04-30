package builtin

import (
	"bytes"
	"fmt"
	"os"
	"strings"
	"mygit-go/core"
)

func Log() {
	head, err := os.ReadFile(".git/refs/heads/main")
	if err != nil {
		fmt.Println("No commits yet.")
		return
	}
	sha := string(bytes.TrimSpace(head))
	for sha != "" {
		t, data, err := core.ReadObject(sha)
		if err != nil || t != "commit" { break }
		fmt.Printf("\033[33mcommit %s\033[0m\n", sha)
		lines := strings.Split(string(data), "\n")
		var parent string
		for _, l := range lines {
			if strings.HasPrefix(l, "author") { fmt.Println(l) }
			if strings.HasPrefix(l, "parent") { parent = strings.TrimPrefix(l, "parent ") }
			if l == "" { break }
		}
		sha = parent
		fmt.Println()
	}
}
