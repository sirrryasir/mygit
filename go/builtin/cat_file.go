package builtin

import (
	"fmt"
	"os"
	"mygit-go/core"
)

func CatFile(sha string) {
	_, data, err := core.ReadObject(sha)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s\n", err)
		return
	}
	fmt.Print(string(data))
}
