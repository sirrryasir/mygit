import * as fs from 'fs';
import * as crypto from 'crypto';
import * as zlib from 'zlib';

export interface IndexEntry {
  ctime: { sec: number; nsec: number };
  mtime: { sec: number; nsec: number };
  dev: number;
  ino: number;
  mode: number;
  uid: number;
  gid: number;
  size: number;
  sha1: Buffer;
  flags: number;
  name: string;
}

export class IndexManager {
  public entries: IndexEntry[] = [];
  private indexPath = '.git/index';

  constructor() {
    this.readIndex();
  }

  private readIndex() {
    if (!fs.existsSync(this.indexPath)) {
      this.entries = [];
      return;
    }

    const buffer = fs.readFileSync(this.indexPath);
    
    // Check signature
    const signature = buffer.subarray(0, 4).toString('ascii');
    if (signature !== 'DIRC') {
      throw new Error(`Invalid index signature: ${signature}`);
    }

    const version = buffer.readUInt32BE(4);
    if (version !== 2) {
      throw new Error(`Unsupported index version: ${version}`);
    }

    const numEntries = buffer.readUInt32BE(8);
    let offset = 12;

    this.entries = [];

    for (let i = 0; i < numEntries; i++) {
      const entry: Partial<IndexEntry> = {};
      
      entry.ctime = {
        sec: buffer.readUInt32BE(offset),
        nsec: buffer.readUInt32BE(offset + 4),
      };
      offset += 8;
      
      entry.mtime = {
        sec: buffer.readUInt32BE(offset),
        nsec: buffer.readUInt32BE(offset + 4),
      };
      offset += 8;
      
      entry.dev = buffer.readUInt32BE(offset); offset += 4;
      entry.ino = buffer.readUInt32BE(offset); offset += 4;
      entry.mode = buffer.readUInt32BE(offset); offset += 4;
      entry.uid = buffer.readUInt32BE(offset); offset += 4;
      entry.gid = buffer.readUInt32BE(offset); offset += 4;
      entry.size = buffer.readUInt32BE(offset); offset += 4;
      
      entry.sha1 = buffer.subarray(offset, offset + 20); offset += 20;
      
      entry.flags = buffer.readUInt16BE(offset); offset += 2;
      
      // Read name (null-terminated)
      let nameEnd = offset;
      while (buffer[nameEnd] !== 0) {
        nameEnd++;
      }
      
      entry.name = buffer.subarray(offset, nameEnd).toString('utf8');
      
      // Calculate padding: entry size must be a multiple of 8
      const entrySizeBeforePadding = 62 + Buffer.byteLength(entry.name) + 1; // 62 bytes fixed + name + 1 null byte
      let padding = 8 - (entrySizeBeforePadding % 8);
      if (padding === 0) padding = 8; // Git always adds 1-8 null bytes
      
      offset = nameEnd + padding; // Skip name and padding
      
      this.entries.push(entry as IndexEntry);
    }
  }

  public writeIndex() {
    // Sort entries alphabetically by name before writing
    this.entries.sort((a, b) => a.name < b.name ? -1 : (a.name > b.name ? 1 : 0));

    const header = Buffer.alloc(12);
    header.write('DIRC', 0, 'ascii');
    header.writeUInt32BE(2, 4); // Version 2
    header.writeUInt32BE(this.entries.length, 8);

    const entryBuffers: Buffer[] = [];

    for (const entry of this.entries) {
      const fixedBuffer = Buffer.alloc(62);
      let offset = 0;
      
      fixedBuffer.writeUInt32BE(entry.ctime.sec, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.ctime.nsec, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.mtime.sec, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.mtime.nsec, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.dev, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.ino, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.mode, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.uid, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.gid, offset); offset += 4;
      fixedBuffer.writeUInt32BE(entry.size, offset); offset += 4;
      
      entry.sha1.copy(fixedBuffer, offset); offset += 20;
      
      fixedBuffer.writeUInt16BE(entry.flags, offset); offset += 2;
      
      const nameBuffer = Buffer.from(entry.name, 'utf8');
      
      // Calculate padding
      const entrySizeWithoutPadding = 62 + nameBuffer.length + 1;
      let paddingSize = 8 - (entrySizeWithoutPadding % 8);
      if (paddingSize === 0) paddingSize = 8;
      
      const paddingBuffer = Buffer.alloc(paddingSize, 0);
      
      entryBuffers.push(Buffer.concat([fixedBuffer, nameBuffer, paddingBuffer]));
    }

    const contentBuffer = Buffer.concat([header, ...entryBuffers]);
    const sha1 = crypto.createHash('sha1').update(contentBuffer).digest();
    
    const finalBuffer = Buffer.concat([contentBuffer, sha1]);
    
    fs.writeFileSync(this.indexPath, finalBuffer);
  }

  public addEntry(filePath: string, sha1Hex: string, stat: fs.Stats) {
    const existingIndex = this.entries.findIndex(e => e.name === filePath);
    
    const nameLength = Buffer.byteLength(filePath);
    const flags = nameLength > 0xFFF ? 0xFFF : nameLength;
    
    const mode = (stat.mode & 0o111) !== 0 ? 0o100755 : 0o100644;

    const newEntry: IndexEntry = {
      ctime: { sec: Math.floor(stat.ctimeMs / 1000), nsec: Math.floor((stat.ctimeMs % 1000) * 1000000) },
      mtime: { sec: Math.floor(stat.mtimeMs / 1000), nsec: Math.floor((stat.mtimeMs % 1000) * 1000000) },
      dev: stat.dev,
      ino: stat.ino,
      mode: mode,
      uid: stat.uid,
      gid: stat.gid,
      size: stat.size,
      sha1: Buffer.from(sha1Hex, 'hex'),
      flags: flags,
      name: filePath
    };

    if (existingIndex !== -1) {
      this.entries[existingIndex] = newEntry;
    } else {
      this.entries.push(newEntry);
    }
  }

  public writeTreeFromIndex(): string {

    
    interface TreeNode {
      name: string;
      mode: string;
      sha1?: Buffer;
      children?: Map<string, TreeNode>;
    }

    const root: TreeNode = { name: '', mode: '40000', children: new Map() };

    for (const entry of this.entries) {
      const parts = entry.name.split('/');
      let current = root;
      
      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isFile = i === parts.length - 1;
        
        if (isFile) {
          current.children!.set(part, {
            name: part,
            mode: entry.mode.toString(8), // Convert numeric mode to octal string like "100644"
            sha1: entry.sha1
          });
        } else {
          if (!current.children!.has(part)) {
            current.children!.set(part, {
              name: part,
              mode: '40000',
              children: new Map()
            });
          }
          current = current.children!.get(part)!;
        }
      }
    }

    function writeTreeNode(node: TreeNode): Buffer {
      if (node.sha1) {
        return node.sha1;
      }
      
      const treeEntries: { mode: string; name: string; sha: Buffer }[] = [];
      for (const child of node.children!.values()) {
        const sha = writeTreeNode(child);
        treeEntries.push({ mode: child.mode, name: child.name, sha });
      }
      
      treeEntries.sort((a, b) => a.name < b.name ? -1 : (a.name > b.name ? 1 : 0));
      
      const treeBuffers: Buffer[] = [];
      for (const entry of treeEntries) {
        treeBuffers.push(Buffer.from(`${entry.mode} ${entry.name}\0`));
        treeBuffers.push(entry.sha);
      }
      
      const treeContent = Buffer.concat(treeBuffers);
      const treeHeader = `tree ${treeContent.length}\0`;
      const treeStore = Buffer.concat([Buffer.from(treeHeader), treeContent]);
      
      const treeShaStr = crypto.createHash('sha1').update(treeStore).digest('hex');
      const treeSha = Buffer.from(treeShaStr, 'hex');
      
      const compressedTree = zlib.deflateSync(treeStore);
      const dir = treeShaStr.slice(0, 2);
      const filename = treeShaStr.slice(2);
      const dirPath = `.git/objects/${dir}`;
      if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath, { recursive: true });
      }
      fs.writeFileSync(`${dirPath}/${filename}`, compressedTree);
      
      return treeSha;
    }

    const rootSha = writeTreeNode(root);
    return rootSha.toString('hex');
  }
}
