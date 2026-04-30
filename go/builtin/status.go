package builtin

import "fmt"

func Status() {
	fmt.Println("On branch main")
	fmt.Println("No changes staged for commit (use \"mygit-go add\" to stage)")
}
