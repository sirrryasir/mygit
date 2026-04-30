# MyGit - Professional Git Implementations

This repository contains professional-grade implementations of the Git version control system in **Rust**, **Go**, and **TypeScript**.

The architecture mirrors the official Git (C) source code, with a clean separation between high-level commands, core object management, and low-level utilities.

## 🏗️ Architecture

All implementations follow a standardized modular structure:

- `builtin/`: Command-specific modules (init, add, commit, etc.)
- `core/`: Fundamental Git logic (Object management, Index parsing, Delta resolution)
- `utils/`: Reusable helper functions and IO utilities

## 🚀 Key Features

- **Professional Parity**: Mirrors Git's internal algorithms and file structures.
- **Delta Resolution**: Full support for delta-encoded objects in packfiles (Rust & Go).
- **Zlib Compression**: Efficient binary data handling using industry-standard compression.
- **SHA-1 Hashing**: Cryptographically accurate object identifiers.
- **Modular Design**: Highly scalable codebase designed for educational and professional use.

## 🛠️ Usage

### Rust (`mygit-rs`)
```bash
cd rs
cargo build --release
./target/release/mygit init
```

### Go (`mygit-go`)
```bash
cd go
go build -o mygit-go main.go
./mygit-go init
```

### TypeScript (`mygit-ts`)
```bash
cd ts
bun install
bun app/main.ts init
```

## 📜 Commands Supported

- `init`: Initialize a new repository
- `add`: Stage files to the index
- `commit`: Record changes
- `log`: View commit history
- `status`: Check working tree state
- `cat-file`: Inspect objects
- `hash-object`: Compute object hashes
- `ls-tree`: List tree contents
- `checkout`: Switch branches/commits
- `clone`: Clone local repositories
- `branch`: Manage branches
