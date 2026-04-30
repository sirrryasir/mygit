package builtin

import (
	"fmt"
	"mygit-go/core"
)

func HashObject(file string, write bool) {
	fmt.Println(core.HashObject(file, write))
}
