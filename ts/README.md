# My Own Git Implementation

This is a standalone Git implementation written from scratch in TypeScript, capable of initializing a repository, reading/writing Git objects (blobs, trees, commits), and more.

## Prerequisites

Ensure you have `bun` installed locally to run this project.

## Usage

You can use the provided `mygit` executable just like the real `git` CLI.

```sh
# Initialize an empty git repository
./mygit init

# Read a blob
./mygit cat-file -p <blob_sha>

# Hash and write a file object
./mygit hash-object -w <file_path>

# Read a tree object
./mygit ls-tree --name-only <tree_sha>

# Write current working directory to a tree
./mygit write-tree

# Create a commit
./mygit commit-tree <tree_sha> -p <parent_sha> -m "Commit message"
```

## Testing Locally

We suggest executing `./mygit` in a different folder when testing locally to avoid overwriting your actual repository's `.git` folder.

```sh
mkdir -p /tmp/testing && cd /tmp/testing
/path/to/your/repo/mygit init
```

To make this easier to type out, you could add an alias in your shell:
```sh
alias mygit=/path/to/your/repo/mygit
```
