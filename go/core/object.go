package core

import (
	"bytes"
	"compress/zlib"
	"crypto/sha1"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

func ReadObject(sha string) (string, []byte, error) {
	// 1. Loose
	path := filepath.Join(".git", "objects", sha[:2], sha[2:])
	if f, err := os.Open(path); err == nil {
		defer f.Close()
		r, err := zlib.NewReader(f)
		if err != nil { return "", nil, err }
		data, _ := io.ReadAll(r)
		parts := bytes.SplitN(data, []byte{0}, 2)
		header := strings.Split(string(parts[0]), " ")
		return header[0], parts[1], nil
	}

	// 2. Packfiles
	packs, _ := filepath.Glob(".git/objects/pack/*.pack")
	for _, p := range packs {
		if t, d, err := SearchPackfile(p, sha); err == nil {
			return t, d, nil
		}
	}

	return "", nil, fmt.Errorf("object not found: %s", sha)
}

func SearchPackfile(packPath, targetSha string) (string, []byte, error) {
	idxPath := packPath[:len(packPath)-5] + ".idx"
	idxData, err := os.ReadFile(idxPath)
	if err != nil { return "", nil, err }
	packData, err := os.ReadFile(packPath)
	if err != nil { return "", nil, err }

	if !bytes.Equal(idxData[:4], []byte("\xfftOc")) { return "", nil, fmt.Errorf("unsupported idx version") }

	numObjects := binary.BigEndian.Uint32(idxData[255*4+4 : 255*4+8])
	target, _ := hex.DecodeString(targetSha)
	
	idx := sort.Search(int(numObjects), func(i int) bool {
		start := 8 + 1024 + i*20
		return bytes.Compare(idxData[start:start+20], target) >= 0
	})
	
	start := 8 + 1024 + idx*20
	if idx >= int(numObjects) || !bytes.Equal(idxData[start:start+20], target) {
		return "", nil, fmt.Errorf("not found in pack")
	}

	offsetStart := 8 + 1024 + int(numObjects)*20 + int(numObjects)*4 + idx*4
	offset := binary.BigEndian.Uint32(idxData[offsetStart : offsetStart+4])

	return UnpackObject(packData, int(offset), packPath)
}

func UnpackObject(packData []byte, offset int, packPath string) (string, []byte, error) {
	cursor := offset
	byteVal := packData[cursor]
	cursor++
	objType := (byteVal >> 4) & 0x7
	
	for byteVal&0x80 != 0 {
		byteVal = packData[cursor]
		cursor++
	}

	switch objType {
	case 1, 2, 3, 4: // Commit, Tree, Blob, Tag
		r, err := zlib.NewReader(bytes.NewReader(packData[cursor:]))
		if err != nil { return "", nil, err }
		data, _ := io.ReadAll(r)
		types := map[uint8]string{1: "commit", 2: "tree", 3: "blob", 4: "tag"}
		return types[objType], data, nil
	case 6: // OFS_DELTA
		byteVal = packData[cursor]
		cursor++
		relOffset := int(byteVal & 0x7F)
		for byteVal&0x80 != 0 {
			relOffset++
			byteVal = packData[cursor]
			cursor++
			relOffset = (relOffset << 7) | int(byteVal&0x7F)
		}
		baseOffset := offset - relOffset
		baseType, baseData, err := UnpackObject(packData, baseOffset, packPath)
		if err != nil { return "", nil, err }

		r, err := zlib.NewReader(bytes.NewReader(packData[cursor:]))
		if err != nil { return "", nil, err }
		deltaData, _ := io.ReadAll(r)

		return baseType, ApplyDelta(baseData, deltaData), nil
	case 7: // REF_DELTA
		baseSha := hex.EncodeToString(packData[cursor : cursor+20])
		cursor += 20
		baseType, baseData, err := ReadObject(baseSha)
		if err != nil { return "", nil, err }

		r, err := zlib.NewReader(bytes.NewReader(packData[cursor:]))
		if err != nil { return "", nil, err }
		deltaData, _ := io.ReadAll(r)

		return baseType, ApplyDelta(baseData, deltaData), nil
	}

	return "", nil, fmt.Errorf("unknown object type %d", objType)
}

func ApplyDelta(base, delta []byte) []byte {
	cursor := 0
	readSize := func() int {
		size := 0
		shift := 0
		for {
			b := delta[cursor]
			cursor++
			size |= int(b&0x7F) << shift
			shift += 7
			if b&0x80 == 0 { break }
		}
		return size
	}

	_ = readSize() // base size
	resultSize := readSize()
	result := make([]byte, 0, resultSize)

	for cursor < len(delta) {
		cmd := delta[cursor]
		cursor++
		if cmd&0x80 != 0 { // Copy
			offset := 0
			if cmd&0x01 != 0 { offset |= int(delta[cursor]); cursor++ }
			if cmd&0x02 != 0 { offset |= int(delta[cursor]) << 8; cursor++ }
			if cmd&0x04 != 0 { offset |= int(delta[cursor]) << 16; cursor++ }
			if cmd&0x08 != 0 { offset |= int(delta[cursor]) << 24; cursor++ }
			
			size := 0
			if cmd&0x10 != 0 { size |= int(delta[cursor]); cursor++ }
			if cmd&0x20 != 0 { size |= int(delta[cursor]) << 8; cursor++ }
			if cmd&0x40 != 0 { size |= int(delta[cursor]) << 16; cursor++ }
			if size == 0 { size = 0x10000 }
			
			result = append(result, base[offset:offset+size]...)
		} else if cmd > 0 { // Insert
			size := int(cmd)
			result = append(result, delta[cursor:cursor+size]...)
			cursor += size
		}
	}
	return result
}

func WriteObject(sha string, data []byte) {
	dir := filepath.Join(".git", "objects", sha[:2])
	os.MkdirAll(dir, 0755)
	f, _ := os.Create(filepath.Join(dir, sha[2:]))
	defer f.Close()
	w := zlib.NewWriter(f)
	w.Write(data)
	w.Close()
}

func HashObject(file string, write bool) string {
	content, err := os.ReadFile(file)
	if err != nil { return "" }
	header := fmt.Sprintf("blob %d\x00", len(content))
	full := append([]byte(header), content...)
	h := sha1.New()
	h.Write(full)
	sha := hex.EncodeToString(h.Sum(nil))
	if write { WriteObject(sha, full) }
	return sha
}
