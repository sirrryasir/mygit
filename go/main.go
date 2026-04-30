package main

import (
	"fmt"
	"os"
	"mygit-go/builtin"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: mygit-go <command> [<args>]")
		return
	}

	command := os.Args[1]
	switch command {
	case "init":
		builtin.InitRepo()
	case "cat-file":
		if len(os.Args) < 4 {
			fmt.Println("Usage: mygit-go cat-file -p <sha>")
			return
		}
		builtin.CatFile(os.Args[3])
	case "hash-object":
		write := false
		file := ""
		if len(os.Args) == 4 && os.Args[2] == "-w" {
			write = true
			file = os.Args[3]
		} else {
			file = os.Args[2]
		}
		builtin.HashObject(file, write)
	case "add":
		builtin.AddFiles(os.Args[2:])
	case "status":
		builtin.Status()
	case "log":
		builtin.Log()
	case "-v", "--version":
		fmt.Println("mygit-go version 0.1.0")
	default:
		fmt.Printf("Unknown command: %s\n", command)
	}
}
