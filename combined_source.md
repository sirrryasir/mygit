### File: .gitignore ** **
node_modules/
target/
dist/
*.bin
*.exe
mygit
mygit-linux
.DS_Store

** **

### File: ts/app/config.ts ** **
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

const GLOBAL_CONFIG = path.join(os.homedir(), ".mygitconfig");
const LOCAL_CONFIG = ".git/config";

export function getConfig(key: string): string | undefined {
  // Try local first, then global
  const local = readConfig(LOCAL_CONFIG);
  if (local[key]) return local[key];
  
  const global = readConfig(GLOBAL_CONFIG);
  return global[key];
}

export function setConfig(key: string, value: string, global: boolean = false) {
  const filePath = global ? GLOBAL_CONFIG : LOCAL_CONFIG;
  const config = readConfig(filePath);
  config[key] = value;
  writeConfig(filePath, config);
}

function readConfig(filePath: string): Record<string, string> {
  const result: Record<string, string> = {};
  if (!fs.existsSync(filePath)) return result;
  
  const content = fs.readFileSync(filePath, "utf-8");
  const lines = content.split("\n");
  let section = "";
  
  for (const line of lines) {
    const sMatch = line.match(/^\[(.+)\]$/);
    if (sMatch) {
      section = sMatch[1].replace(/"/g, "");
      continue;
    }
    
    const kvMatch = line.match(/^\t?([^=]+) = (.+)$/);
    if (kvMatch && section) {
      const key = kvMatch[1].trim();
      const val = kvMatch[2].trim();
      result[`${section}.${key}`] = val;
    }
  }
  return result;
}

function writeConfig(filePath: string, config: Record<string, string>) {
  const sections: Record<string, Record<string, string>> = {};
  
  for (const [fullKey, val] of Object.entries(config)) {
    const lastDot = fullKey.lastIndexOf(".");
    const section = fullKey.substring(0, lastDot);
    const key = fullKey.substring(lastDot + 1);
    
    if (!sections[section]) sections[section] = {};
    sections[section][key] = val;
  }
  
  let content = "";
  for (const [section, kvs] of Object.entries(sections)) {
    content += `[${section}]\n`;
    for (const [k, v] of Object.entries(kvs)) {
      content += `\t${k} = ${v}\n`;
    }
  }
  
  fs.writeFileSync(filePath, content);
}

** **

### File: ts/app/git-helpers.ts ** **
import * as fs from "fs";
import * as crypto from "crypto";
import * as zlib from "zlib";
import * as path from "path";

// ─── Object Store ───────────────────────────────────────────

export function readObject(sha: string): { type: string; content: Buffer } {
  const objPath = `.git/objects/${sha.slice(0, 2)}/${sha.slice(2)}`;
  if (!fs.existsSync(objPath)) throw new Error(`fatal: Not a valid object name ${sha}`);
  const compressed = fs.readFileSync(objPath);
  const decompressed = zlib.inflateSync(compressed);
  const nullIdx = decompressed.indexOf(0);
  const header = decompressed.subarray(0, nullIdx).toString("utf-8");
  const [type] = header.split(" ");
  const content = decompressed.subarray(nullIdx + 1);
  return { type, content };
}

export function writeObject(type: string, content: Buffer): string {
  const header = `${type} ${content.length}\0`;
  const store = Buffer.concat([Buffer.from(header), content]);
  const sha = crypto.createHash("sha1").update(store).digest("hex");
  const dir = `.git/objects/${sha.slice(0, 2)}`;
  const filePath = `${dir}/${sha.slice(2)}`;
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  if (!fs.existsSync(filePath)) {
    fs.writeFileSync(filePath, zlib.deflateSync(store));
  }
  return sha;
}

export function createBlob(filePath: string): string {
  const content = fs.readFileSync(filePath);
  return writeObject("blob", content);
}

// ─── HEAD / Refs ────────────────────────────────────────────

export function resolveHead(): string {
  if (!fs.existsSync(".git/HEAD")) return "";
  const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
  if (headContent.startsWith("ref: ")) {
    const refPath = `.git/${headContent.slice(5)}`;
    if (fs.existsSync(refPath)) {
      return fs.readFileSync(refPath, "utf-8").trim();
    }
    return "";
  }
  return headContent;
}

export function currentBranch(): string {
  if (!fs.existsSync(".git/HEAD")) return "";
  const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
  if (headContent.startsWith("ref: refs/heads/")) {
    return headContent.slice("ref: refs/heads/".length);
  }
  return "";
}

export function updateRef(refName: string, sha: string) {
  const refPath = `.git/${refName}`;
  const refDir = path.dirname(refPath);
  if (!fs.existsSync(refDir)) fs.mkdirSync(refDir, { recursive: true });
  fs.writeFileSync(refPath, sha + "\n");
}

export function updateHead(sha: string) {
  const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
  if (headContent.startsWith("ref: ")) {
    updateRef(headContent.slice(5), sha);
  } else {
    fs.writeFileSync(".git/HEAD", sha + "\n");
  }
}

// ─── Tree Parsing ───────────────────────────────────────────

export function readTreeFlat(treeSha: string, prefix: string = ""): Map<string, string> {
  const files = new Map<string, string>();
  const { content } = readObject(treeSha);
  let offset = 0;

  while (offset < content.length) {
    let spaceIdx = offset;
    while (content[spaceIdx] !== 0x20) spaceIdx++;
    const mode = content.subarray(offset, spaceIdx).toString("utf-8");

    let nameEnd = spaceIdx + 1;
    while (content[nameEnd] !== 0) nameEnd++;
    const name = content.subarray(spaceIdx + 1, nameEnd).toString("utf-8");

    const sha = content.subarray(nameEnd + 1, nameEnd + 21);
    const shaHex = Buffer.from(sha).toString("hex");
    offset = nameEnd + 21;

    const fullPath = prefix ? `${prefix}/${name}` : name;
    if (mode === "40000") {
      const subFiles = readTreeFlat(shaHex, fullPath);
      for (const [k, v] of subFiles) files.set(k, v);
    } else {
      files.set(fullPath, shaHex);
    }
  }
  return files;
}

export function getCommitTree(commitSha: string): string {
  const { content } = readObject(commitSha);
  const text = content.toString("utf-8");
  const match = text.match(/^tree ([a-f0-9]{40})/);
  return match ? match[1] : "";
}

export function getCommitParents(commitSha: string): string[] {
  const { content } = readObject(commitSha);
  const text = content.toString("utf-8");
  const parents: string[] = [];
  for (const line of text.split("\n")) {
    if (line.startsWith("parent ")) parents.push(line.slice(7));
    if (line === "") break;
  }
  return parents;
}

// ─── Commit Creation ────────────────────────────────────────

import { getConfig } from "./config.js";

export function createCommit(treeSha: string, parentShas: string[], message: string): string {
  const timestamp = Math.floor(Date.now() / 1000);
  const timezone = "+0000";
  const authorName = getConfig("user.name") || "Yasir";
  const authorEmail = getConfig("user.email") || "yasir@example.com";

  let commitContent = `tree ${treeSha}\n`;
  for (const parent of parentShas) {
    if (parent) commitContent += `parent ${parent}\n`;
  }
  commitContent += `author ${authorName} <${authorEmail}> ${timestamp} ${timezone}\n`;
  commitContent += `committer ${authorName} <${authorEmail}> ${timestamp} ${timezone}\n\n`;
  commitContent += `${message}\n`;

  return writeObject("commit", Buffer.from(commitContent));
}

// ─── Ancestor Check ─────────────────────────────────────────

export function isAncestor(potentialAncestor: string, commitSha: string): boolean {
  // BFS through parents
  const visited = new Set<string>();
  const queue = [commitSha];
  while (queue.length > 0) {
    const current = queue.shift()!;
    if (current === potentialAncestor) return true;
    if (visited.has(current)) continue;
    visited.add(current);
    const parents = getCommitParents(current);
    queue.push(...parents);
  }
  return false;
}

** **

### File: ts/app/ignore.ts ** **
import * as fs from "fs";
import * as path from "path";

export class IgnoreManager {
  private patterns: string[] = [];

  constructor() {
    this.loadPatterns(".gitignore");
    this.loadPatterns(".mygitignore");
    // Standard git ignores
    this.patterns.push(".git/");
  }

  private loadPatterns(file: string) {
    if (fs.existsSync(file)) {
      const content = fs.readFileSync(file, "utf-8");
      const lines = content.split("\n");
      for (let line of lines) {
        line = line.trim();
        if (line && !line.startsWith("#")) {
          this.patterns.push(line);
        }
      }
    }
  }

  public isIgnored(filePath: string): boolean {
    for (const pattern of this.patterns) {
      if (this.match(pattern, filePath)) return true;
    }
    return false;
  }

  private match(pattern: string, filePath: string): boolean {
    // Simple glob to regex conversion
    // handle directory-only patterns (ending in /)
    const isDirPattern = pattern.endsWith("/");
    const cleanPattern = isDirPattern ? pattern.slice(0, -1) : pattern;
    
    // Convert glob stars to regex
    let regexStr = cleanPattern
      .replace(/\./g, "\\.")
      .replace(/\*\*/g, "(.+)")
      .replace(/\*/g, "([^/]+)")
      .replace(/\?/g, "(.)");
      
    if (isDirPattern) {
      regexStr = `^${regexStr}/|^${regexStr}$`;
    } else {
      regexStr = `^${regexStr}$|^${regexStr}/`;
    }
    
    const regex = new RegExp(regexStr);
    
    // Check path parts
    const parts = filePath.split("/");
    let current = "";
    for (const part of parts) {
      current = current ? `${current}/${part}` : part;
      if (regex.test(current)) return true;
      // Also match basename if pattern doesn't have /
      if (!pattern.includes("/") && regex.test(part)) return true;
    }
    
    return false;
  }
}

** **

### File: ts/app/index-manager.ts ** **
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

** **

### File: ts/app/main.ts ** **
import * as fs from "fs";
import * as crypto from "crypto";
import * as zlib from "zlib";
import * as path from "path";
import { IndexManager } from "./index-manager.js";
import { resolveHead, currentBranch, updateHead, updateRef, readObject, writeObject, createBlob, readTreeFlat, getCommitTree, getCommitParents, createCommit, isAncestor } from "./git-helpers.js";
import { addRemote, removeRemote, listRemotes, getRemoteUrl } from "./remote.js";
import { discoverRefs, fetchPack, parsePackfile, pushPack } from "./packfile.js";
import { setConfig, getConfig } from "./config.js";
import { IgnoreManager } from "./ignore.js";

const args = process.argv.slice(2);
const command = args[0];

function writeTree(dirPath: string): string {

  
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  const treeEntries: { mode: string; name: string; rawSha: Buffer }[] = [];
  
  for (const entry of entries) {
    if (entry.name === ".git") continue;
    
    const fullPath = `${dirPath}/${entry.name}`;
    if (entry.isDirectory()) {
      const sha = writeTree(fullPath);
      treeEntries.push({
        mode: "40000",
        name: entry.name,
        rawSha: Buffer.from(sha, "hex")
      });
    } else if (entry.isFile()) {
      const stat = fs.statSync(fullPath);
      const isExecutable = (stat.mode & 0o111) !== 0;
      const mode = isExecutable ? "100755" : "100644";
      
      const fileContent = fs.readFileSync(fullPath);
      const header = `blob ${fileContent.length}\0`;
      const store = Buffer.concat([Buffer.from(header), fileContent]);
      const sha = crypto.createHash("sha1").update(store).digest("hex");
      
      const compressed = zlib.deflateSync(store);
      const objectDir = sha.slice(0, 2);
      const filename = sha.slice(2);
      const dirPathObject = `.git/objects/${objectDir}`;
      if (!fs.existsSync(dirPathObject)) {
        fs.mkdirSync(dirPathObject, { recursive: true });
      }
      fs.writeFileSync(`${dirPathObject}/${filename}`, compressed);
      
      treeEntries.push({
        mode,
        name: entry.name,
        rawSha: Buffer.from(sha, "hex")
      });
    }
  }
  
  treeEntries.sort((a, b) => a.name < b.name ? -1 : (a.name > b.name ? 1 : 0));
  
  const treeBuffers: Buffer[] = [];
  for (const entry of treeEntries) {
    treeBuffers.push(Buffer.from(`${entry.mode} ${entry.name}\0`));
    treeBuffers.push(entry.rawSha);
  }
  
  const treeContent = Buffer.concat(treeBuffers);
  const treeHeader = `tree ${treeContent.length}\0`;
  const treeStore = Buffer.concat([Buffer.from(treeHeader), treeContent]);
  
  const treeSha = crypto.createHash("sha1").update(treeStore).digest("hex");
  
  const compressedTree = zlib.deflateSync(treeStore);
  const objectDirTree = treeSha.slice(0, 2);
  const filenameTree = treeSha.slice(2);
  const dirPathTree = `.git/objects/${objectDirTree}`;
  if (!fs.existsSync(dirPathTree)) {
    fs.mkdirSync(dirPathTree, { recursive: true });
  }
  fs.writeFileSync(`${dirPathTree}/${filenameTree}`, compressedTree);
  
  return treeSha;
}

switch (command) {
  case "init":
    fs.mkdirSync(".git", { recursive: true });
    fs.mkdirSync(".git/objects", { recursive: true });
    fs.mkdirSync(".git/refs", { recursive: true });
    fs.writeFileSync(".git/HEAD", "ref: refs/heads/main\n");
    console.log("Initialized empty Git repository in " + path.join(process.cwd(), ".git/"));
    break;

  case "cat-file": {
    const flag = args[1];
    const sha = args[2];
    if (flag === "-p" && sha) {
      const dir = sha.slice(0, 2);
      const file = sha.slice(2);
      const filePath = `.git/objects/${dir}/${file}`;
      
      const compressed = fs.readFileSync(filePath);
      const zlib = require("zlib");
      const decompressed = zlib.unzipSync(compressed);
      
      const nullByteIndex = decompressed.indexOf(0);
      const content = decompressed.subarray(nullByteIndex + 1);
      
      process.stdout.write(content);
    }
    break;
  }
  case "hash-object": {
    let flag = args[1];
    let file = args[2];
    let write = false;
    
    if (flag === "-w") {
      write = true;
    } else {
      file = flag;
    }
    
    if (file) {

      
      const fileContent = fs.readFileSync(file);
      const header = `blob ${fileContent.length}\0`;
      const store = Buffer.concat([Buffer.from(header), fileContent]);
      
      const hash = crypto.createHash("sha1").update(store).digest("hex");
      
      if (write) {
        const compressed = zlib.deflateSync(store);
        const dir = hash.slice(0, 2);
        const filename = hash.slice(2);
        const dirPath = `.git/objects/${dir}`;
        if (!fs.existsSync(dirPath)) {
          fs.mkdirSync(dirPath, { recursive: true });
        }
        fs.writeFileSync(`${dirPath}/${filename}`, compressed);
      }
      
      process.stdout.write(`${hash}\n`);
    }
    break;
  }
  case "ls-tree": {
    let flag = args[1];
    let sha = args[2];
    let nameOnly = false;
    
    if (flag === "--name-only") {
      nameOnly = true;
    } else {
      sha = flag;
    }
    
    if (sha) {
      const zlib = require("zlib");
      const dir = sha.slice(0, 2);
      const file = sha.slice(2);
      const filePath = `.git/objects/${dir}/${file}`;
      
      const compressed = fs.readFileSync(filePath);
      const decompressed = zlib.unzipSync(compressed);
      
      const nullByteIndex = decompressed.indexOf(0);
      const entriesContent = decompressed.subarray(nullByteIndex + 1);
      
      let cursor = 0;
      const names: string[] = [];
      
      while (cursor < entriesContent.length) {
        const spaceIndex = entriesContent.indexOf(32, cursor);
        const nullIndex = entriesContent.indexOf(0, spaceIndex);
        
        const name = entriesContent.subarray(spaceIndex + 1, nullIndex).toString();
        names.push(name);
        
        cursor = nullIndex + 21;
      }
      
      if (nameOnly) {
        names.sort().forEach(name => console.log(name));
      }
    }
    break;
  }
  case "write-tree": {
    const cwd = process.cwd();
    const treeSha = writeTree(cwd);
    process.stdout.write(`${treeSha}\n`);
    break;
  }
  case "commit-tree": {
    const treeSha = args[1];
    let parentSha = "";
    let message = "";
    
    for (let i = 2; i < args.length; i++) {
      if (args[i] === "-p") {
        parentSha = args[i + 1];
        i++;
      } else if (args[i] === "-m") {
        message = args[i + 1];
        i++;
      }
    }
    
    const timestamp = Math.floor(Date.now() / 1000);
    const timezone = "+0000";
    const authorName = "Yasir";
    const authorEmail = "yasir@example.com";
    
    let commitContent = `tree ${treeSha}\n`;
    if (parentSha) {
      commitContent += `parent ${parentSha}\n`;
    }
    commitContent += `author ${authorName} <${authorEmail}> ${timestamp} ${timezone}\n`;
    commitContent += `committer ${authorName} <${authorEmail}> ${timestamp} ${timezone}\n\n`;
    commitContent += `${message}\n`;
    

    
    const store = Buffer.from(`commit ${Buffer.byteLength(commitContent)}\0${commitContent}`);
    const hash = crypto.createHash("sha1").update(store).digest("hex");
    
    const compressed = zlib.deflateSync(store);
    const dir = hash.slice(0, 2);
    const filename = hash.slice(2);
    const dirPath = `.git/objects/${dir}`;
    if (!fs.existsSync(dirPath)) {
      fs.mkdirSync(dirPath, { recursive: true });
    }
    fs.writeFileSync(`${dirPath}/${filename}`, compressed);
    
    process.stdout.write(`${hash}\n`);
    break;
  }
  case "add": {
    const isForce = args.includes("-f") || args.includes("--force");
    const paths = args.slice(1).filter(a => !a.startsWith("-"));
    if (paths.length === 0) throw new Error("Nothing specified, nothing added.");
    
    const idx = new IndexManager();
    const ignore = new IgnoreManager();
    
    const addPath = (p: string) => {
      const stats = fs.statSync(p);
      if (stats.isDirectory()) {
        const items = fs.readdirSync(p);
        for (const item of items) {
          const full = path.join(p, item).replace(/\\/g, "/");
          if (item === ".git") continue;
          if (!isForce && ignore.isIgnored(full)) continue;
          addPath(full);
        }
      } else {
        if (!isForce && ignore.isIgnored(p)) return;
        const sha = createBlob(p);
        idx.addEntry(p, sha, stats);
      }
    };
    
    for (const p of paths) {
      if (!fs.existsSync(p)) throw new Error(`fatal: pathspec '${p}' did not match any files`);
      addPath(p);
    }
    idx.writeIndex();
    break;
  }
  case "commit": {
    const message = args[args.indexOf("-m") + 1];
    if (!message) throw new Error("Abort commit: no message provided.");
    
    const treeSha = writeTree(".");
    const headSha = resolveHead();
    
    const commitSha = createCommit(treeSha, headSha ? [headSha] : [], message);
    updateHead(commitSha);
    
    const branch = currentBranch() || "main";
    console.log(`[${branch} ${commitSha.slice(0, 7)}] ${message}`);
    break;
  }
  case "log": {
    // Resolve HEAD to a commit hash
    let commitSha = "";
    if (fs.existsSync(".git/HEAD")) {
      const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
      if (headContent.startsWith("ref: ")) {
        const refPath = `.git/${headContent.slice(5)}`;
        if (fs.existsSync(refPath)) {
          commitSha = fs.readFileSync(refPath, "utf-8").trim();
        }
      } else {
        commitSha = headContent;
      }
    }

    if (!commitSha) {
      console.log("fatal: your current branch 'main' does not have any commits yet");
      break;
    }

    // Walk the commit chain
    while (commitSha) {
      const dir = commitSha.slice(0, 2);
      const file = commitSha.slice(2);
      const objPath = `.git/objects/${dir}/${file}`;

      if (!fs.existsSync(objPath)) break;

      const compressed = fs.readFileSync(objPath);
      const decompressed = zlib.inflateSync(compressed);
      const nullIndex = decompressed.indexOf(0);
      const content = decompressed.subarray(nullIndex + 1).toString("utf-8");

      // Parse commit fields
      const lines = content.split("\n");
      let author = "";
      let parentSha = "";
      let messageLines: string[] = [];
      let inMessage = false;

      for (const line of lines) {
        if (inMessage) {
          messageLines.push(line);
        } else if (line === "") {
          inMessage = true;
        } else if (line.startsWith("parent ")) {
          parentSha = line.slice(7);
        } else if (line.startsWith("author ")) {
          author = line.slice(7);
        }
      }

      // Format: author name <email> timestamp tz
      const authorMatch = author.match(/^(.+) <(.+)> (\d+) ([+-]\d{4})$/);
      let dateStr = "";
      if (authorMatch) {
        const ts = parseInt(authorMatch[3]) * 1000;
        dateStr = new Date(ts).toUTCString();
      }

      const message = messageLines.join("\n").trim();

      console.log(`\x1b[33mcommit ${commitSha}\x1b[0m`);
      if (authorMatch) {
        console.log(`Author: ${authorMatch[1]} <${authorMatch[2]}>`);
        console.log(`Date:   ${dateStr}`);
      }
      console.log("");
      console.log(`    ${message}`);
      console.log("");

      commitSha = parentSha;
    }
    break;
  }
  case "status": {
    const headSha = resolveHead();
    const branch = currentBranch() || (headSha ? "detached HEAD" : "main");
    console.log(`On branch ${branch}`);
    
    const idx = new IndexManager();
    const staged = idx.entries;
    
    const headFiles = headSha ? readTreeFlat(getCommitTree(headSha)) : new Map<string, string>();
    const stagedNew: string[] = [];
    const stagedModified: string[] = [];
    const stagedDeleted: string[] = [];
    
    for (const entry of staged) {
      if (!headFiles.has(entry.name)) {
        stagedNew.push(entry.name);
      } else if (headFiles.get(entry.name) !== entry.sha1.toString("hex")) {
        stagedModified.push(entry.name);
      }
    }
    for (const [name] of headFiles) {
      if (!staged.some(e => e.name === name)) stagedDeleted.push(name);
    }
    
    if (stagedNew.length || stagedModified.length || stagedDeleted.length) {
      console.log("Changes to be committed:");
      for (const f of stagedNew) console.log(`\tnew file:   ${f}`);
      for (const f of stagedModified) console.log(`\tmodified:   ${f}`);
      for (const f of stagedDeleted) console.log(`\tdeleted:    ${f}`);
      console.log("");
    }
    
    const unstagedModified: string[] = [];
    const unstagedDeleted: string[] = [];
    const untracked: string[] = [];
    const ignore = new IgnoreManager();
    
    const walk = (dir: string) => {
      const items = fs.readdirSync(dir, { withFileTypes: true });
      for (const item of items) {
        const fullPath = path.join(dir, item.name).replace(/\\/g, "/");
        const relPath = fullPath.startsWith("./") ? fullPath.slice(2) : fullPath;
        if (item.name === ".git") continue;
        if (ignore.isIgnored(relPath)) continue;
        
        if (item.isDirectory()) {
          walk(fullPath);
        } else {
          const entry = staged.find(e => e.name === relPath);
          if (entry) {
            const content = fs.readFileSync(fullPath);
            const sha = crypto.createHash("sha1").update(`blob ${content.length}\0`).update(content).digest("hex");
            if (sha !== entry.sha1.toString("hex")) {
              unstagedModified.push(relPath);
            }
          } else if (!headFiles.has(relPath)) {
            untracked.push(relPath);
          }
        }
      }
    };
    walk(".");
    
    // Check for deleted files that are in index
    for (const entry of staged) {
      if (!fs.existsSync(entry.name)) {
        if (!unstagedDeleted.includes(entry.name)) unstagedDeleted.push(entry.name);
      }
    }
    
    if (unstagedModified.length || unstagedDeleted.length) {
      console.log("Changes not staged for commit:");
      for (const f of unstagedModified) console.log(`\tmodified:   ${f}`);
      for (const f of unstagedDeleted) console.log(`\tdeleted:    ${f}`);
      console.log("");
    }
    
    if (untracked.length) {
      console.log("Untracked files:");
      for (const f of untracked) console.log(`\t${f}`);
      console.log("");
    }
    
    if (!stagedNew.length && !stagedModified.length && !stagedDeleted.length && !unstagedModified.length && !unstagedDeleted.length && !untracked.length) {
      console.log("nothing to commit, working tree clean");
    }
    break;
  }
  case "branch": {
    const branchName = args[1];
    const deleteFlag = args.includes("-d") || args.includes("-D");

    // Resolve current HEAD commit
    function resolveHead(): string {
      if (!fs.existsSync(".git/HEAD")) return "";
      const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
      if (headContent.startsWith("ref: ")) {
        const refPath = `.git/${headContent.slice(5)}`;
        if (fs.existsSync(refPath)) {
          return fs.readFileSync(refPath, "utf-8").trim();
        }
        return "";
      }
      return headContent;
    }

    function currentBranch(): string {
      if (!fs.existsSync(".git/HEAD")) return "";
      const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
      if (headContent.startsWith("ref: refs/heads/")) {
        return headContent.slice("ref: refs/heads/".length);
      }
      return "";
    }

    if (deleteFlag) {
      // Delete branch
      const target = args.find(a => a !== "-d" && a !== "-D" && a !== "branch")!;
      if (!target) throw new Error("fatal: branch name required");
      const refPath = `.git/refs/heads/${target}`;
      if (!fs.existsSync(refPath)) {
        throw new Error(`error: branch '${target}' not found.`);
      }
      if (currentBranch() === target) {
        throw new Error(`error: Cannot delete branch '${target}' checked out at '${process.cwd()}'`);
      }
      fs.unlinkSync(refPath);
      console.log(`Deleted branch ${target}`);
    } else if (!branchName) {
      // List branches
      const refsDir = ".git/refs/heads";
      if (!fs.existsSync(refsDir)) {
        break;
      }
      const branches = fs.readdirSync(refsDir).sort();
      const current = currentBranch();
      for (const branch of branches) {
        if (branch === current) {
          console.log(`* \x1b[32m${branch}\x1b[0m`);
        } else {
          console.log(`  ${branch}`);
        }
      }
    } else {
      // Create new branch
      const commitSha = resolveHead();
      if (!commitSha) {
        throw new Error("fatal: Not a valid object name: 'main'.");
      }
      const refPath = `.git/refs/heads/${branchName}`;
      if (fs.existsSync(refPath)) {
        throw new Error(`fatal: A branch named '${branchName}' already exists.`);
      }
      const refDir = path.dirname(refPath);
      if (!fs.existsSync(refDir)) {
        fs.mkdirSync(refDir, { recursive: true });
      }
      fs.writeFileSync(refPath, commitSha + "\n");
      console.log(`Created branch '${branchName}' at ${commitSha.slice(0, 7)}`);
    }
    break;
  }
  case "checkout": {
    const target = args[1];
    if (!target) throw new Error("fatal: You must specify a branch to checkout.");

    const createNew = args.includes("-b");
    const branchToCheckout = createNew ? args[args.indexOf("-b") + 1] : target;

    if (createNew) {
      // Create branch first
      let headSha = "";
      const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
      if (headContent.startsWith("ref: ")) {
        const refPath = `.git/${headContent.slice(5)}`;
        if (fs.existsSync(refPath)) {
          headSha = fs.readFileSync(refPath, "utf-8").trim();
        }
      } else {
        headSha = headContent;
      }

      const refPath = `.git/refs/heads/${branchToCheckout}`;
      if (fs.existsSync(refPath)) {
        throw new Error(`fatal: A branch named '${branchToCheckout}' already exists.`);
      }
      const refDir = path.dirname(refPath);
      if (!fs.existsSync(refDir)) fs.mkdirSync(refDir, { recursive: true });
      fs.writeFileSync(refPath, headSha + "\n");
    }

    // Check if target is a branch
    const branchRefPath = `.git/refs/heads/${branchToCheckout}`;
    if (fs.existsSync(branchRefPath)) {
      // Switch HEAD to point to the branch
      fs.writeFileSync(".git/HEAD", `ref: refs/heads/${branchToCheckout}\n`);

      // Restore working tree from the branch's commit tree
      const commitSha = fs.readFileSync(branchRefPath, "utf-8").trim();
      const commitObj = fs.readFileSync(`.git/objects/${commitSha.slice(0, 2)}/${commitSha.slice(2)}`);
      const commitData = zlib.inflateSync(commitObj);
      const commitNull = commitData.indexOf(0);
      const commitContent = commitData.subarray(commitNull + 1).toString("utf-8");
      const treeMatch = commitContent.match(/^tree ([a-f0-9]{40})/);

      if (treeMatch) {
        const treeSha = treeMatch[1];

        // Collect currently tracked files (from old index) so we can clean up
        const oldIndexManager = new IndexManager();
        const oldTrackedFiles = new Set(oldIndexManager.entries.map(e => e.name));

        // Rebuild index from tree
        const indexManager = new IndexManager();
        indexManager.entries = []; // Clear
        const newTrackedFiles = new Set<string>();

        function restoreTree(sha: string, prefix: string) {
          const treeObj = fs.readFileSync(`.git/objects/${sha.slice(0, 2)}/${sha.slice(2)}`);
          const treeData = zlib.inflateSync(treeObj);
          const treeNull = treeData.indexOf(0);
          let offset = treeNull + 1;

          while (offset < treeData.length) {
            let spaceIdx = offset;
            while (treeData[spaceIdx] !== 0x20) spaceIdx++;
            const mode = treeData.subarray(offset, spaceIdx).toString("utf-8");

            let nameEnd = spaceIdx + 1;
            while (treeData[nameEnd] !== 0) nameEnd++;
            const name = treeData.subarray(spaceIdx + 1, nameEnd).toString("utf-8");

            const entrySha = treeData.subarray(nameEnd + 1, nameEnd + 21);
            const entryShaHex = Buffer.from(entrySha).toString("hex");
            offset = nameEnd + 21;

            const fullPath = prefix ? `${prefix}/${name}` : name;

            if (mode === "40000") {
              restoreTree(entryShaHex, fullPath);
            } else {
              // Restore file to working directory
              const blobObj = fs.readFileSync(`.git/objects/${entryShaHex.slice(0, 2)}/${entryShaHex.slice(2)}`);
              const blobData = zlib.inflateSync(blobObj);
              const blobNull = blobData.indexOf(0);
              const blobContent = blobData.subarray(blobNull + 1);

              const fileDir = path.dirname(fullPath);
              if (fileDir !== "." && !fs.existsSync(fileDir)) {
                fs.mkdirSync(fileDir, { recursive: true });
              }
              fs.writeFileSync(fullPath, blobContent);

              // Add to index
              newTrackedFiles.add(fullPath);
              const stat = fs.statSync(fullPath);
              indexManager.addEntry(fullPath, entryShaHex, stat);
            }
          }
        }

        restoreTree(treeSha, "");
        indexManager.writeIndex();

        // Remove files that were tracked in the old branch but not in the new one
        for (const oldFile of oldTrackedFiles) {
          if (!newTrackedFiles.has(oldFile) && fs.existsSync(oldFile)) {
            fs.unlinkSync(oldFile);
          }
        }
      }

      console.log(`Switched to branch '${branchToCheckout}'`);
    } else {
      throw new Error(`error: pathspec '${branchToCheckout}' did not match any file(s) known to mygit.`);
    }
    break;
  }
  case "diff": {
    const indexManager = new IndexManager();

    // diff: compare working tree vs index (unstaged changes)
    for (const entry of indexManager.entries) {
      const filePath = entry.name;
      const indexSha = entry.sha1.toString("hex");

      if (!fs.existsSync(filePath)) {
        console.log(`\x1b[1mdiff --git a/${filePath} b/${filePath}\x1b[0m`);
        console.log(`deleted file mode ${entry.mode.toString(8)}`);
        console.log(`index ${indexSha.slice(0, 7)}..0000000`);
        console.log(`--- a/${filePath}`);
        console.log(`+++ /dev/null`);

        // Get old content
        const blobObj = fs.readFileSync(`.git/objects/${indexSha.slice(0, 2)}/${indexSha.slice(2)}`);
        const blobData = zlib.inflateSync(blobObj);
        const blobNull = blobData.indexOf(0);
        const oldLines = blobData.subarray(blobNull + 1).toString("utf-8").split("\n");
        console.log(`@@ -1,${oldLines.length} +0,0 @@`);
        for (const line of oldLines) {
          if (line || oldLines.indexOf(line) < oldLines.length - 1) {
            console.log(`\x1b[31m-${line}\x1b[0m`);
          }
        }
        continue;
      }

      const content = fs.readFileSync(filePath);
      const header = `blob ${content.length}\0`;
      const store = Buffer.concat([Buffer.from(header), content]);
      const workingSha = crypto.createHash("sha1").update(store).digest("hex");

      if (workingSha !== indexSha) {
        console.log(`\x1b[1mdiff --git a/${filePath} b/${filePath}\x1b[0m`);
        console.log(`index ${indexSha.slice(0, 7)}..${workingSha.slice(0, 7)} ${entry.mode.toString(8)}`);
        console.log(`--- a/${filePath}`);
        console.log(`+++ b/${filePath}`);

        // Simple line-by-line diff
        const blobObj = fs.readFileSync(`.git/objects/${indexSha.slice(0, 2)}/${indexSha.slice(2)}`);
        const blobData = zlib.inflateSync(blobObj);
        const blobNull = blobData.indexOf(0);
        const oldLines = blobData.subarray(blobNull + 1).toString("utf-8").split("\n");
        const newLines = content.toString("utf-8").split("\n");

        // Simple unified diff
        const maxLen = Math.max(oldLines.length, newLines.length);
        let hunkStart = -1;
        let hunkOld: string[] = [];
        let hunkNew: string[] = [];

        function flushHunk() {
          if (hunkOld.length === 0 && hunkNew.length === 0) return;
          console.log(`\x1b[36m@@ -${hunkStart + 1},${hunkOld.length} +${hunkStart + 1},${hunkNew.length} @@\x1b[0m`);
          for (const l of hunkOld) console.log(`\x1b[31m-${l}\x1b[0m`);
          for (const l of hunkNew) console.log(`\x1b[32m+${l}\x1b[0m`);
          hunkOld = [];
          hunkNew = [];
        }

        for (let i = 0; i < maxLen; i++) {
          const oldLine = i < oldLines.length ? oldLines[i] : undefined;
          const newLine = i < newLines.length ? newLines[i] : undefined;

          if (oldLine !== newLine) {
            if (hunkStart === -1) hunkStart = i;
            if (oldLine !== undefined) hunkOld.push(oldLine);
            if (newLine !== undefined) hunkNew.push(newLine);
          } else {
            flushHunk();
            hunkStart = -1;
          }
        }
        flushHunk();
      }
    }
    break;
  }
  case "merge": {
    const target = args[1];
    if (!target) throw new Error("fatal: No branch specified.");
    const targetRef = `.git/refs/heads/${target}`;
    if (!fs.existsSync(targetRef)) throw new Error(`merge: ${target} - not something we can merge`);
    const targetSha = fs.readFileSync(targetRef, "utf-8").trim();
    const headSha = resolveHead();
    if (!headSha) throw new Error("fatal: no commits yet");
    if (targetSha === headSha) { console.log("Already up to date."); break; }
    // Fast-forward check
    if (isAncestor(headSha, targetSha)) {
      updateHead(targetSha);
      // Restore working tree
      const treeSha = getCommitTree(targetSha);
      const files = readTreeFlat(treeSha);
      const idx = new IndexManager(); const oldFiles = new Set(idx.entries.map(e => e.name));
      idx.entries = [];
      for (const [fp, sha] of files) {
        const { content } = readObject(sha);
        const dir = path.dirname(fp);
        if (dir !== "." && !fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
        fs.writeFileSync(fp, content);
        const st = fs.statSync(fp); idx.addEntry(fp, sha, st);
      }
      idx.writeIndex();
      for (const f of oldFiles) { if (!files.has(f) && fs.existsSync(f)) fs.unlinkSync(f); }
      console.log(`Updating ${headSha.slice(0,7)}..${targetSha.slice(0,7)}`);
      console.log("Fast-forward");
    } else if (isAncestor(targetSha, headSha)) {
      console.log("Already up to date.");
    } else {
      // 3-way merge
      const headTree = readTreeFlat(getCommitTree(headSha));
      const targetTree = readTreeFlat(getCommitTree(targetSha));
      // Find common ancestor (simple: walk back from both)
      const headAncestors = new Set<string>();
      let cur = headSha;
      while (cur) { headAncestors.add(cur); const p = getCommitParents(cur); cur = p[0] || ""; }
      let baseSha = "";
      cur = targetSha;
      while (cur) { if (headAncestors.has(cur)) { baseSha = cur; break; } const p = getCommitParents(cur); cur = p[0] || ""; }
      const baseTree = baseSha ? readTreeFlat(getCommitTree(baseSha)) : new Map<string, string>();
      const allFiles = new Set([...headTree.keys(), ...targetTree.keys(), ...baseTree.keys()]);
      const mergedIdx = new IndexManager(); mergedIdx.entries = [];
      let conflicts = false;
      for (const fp of allFiles) {
        const bSha = baseTree.get(fp) || ""; const hSha = headTree.get(fp) || ""; const tSha = targetTree.get(fp) || "";
        if (hSha === tSha) {
          if (hSha) { const { content } = readObject(hSha); const d = path.dirname(fp); if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d,{recursive:true}); fs.writeFileSync(fp, content); const s = fs.statSync(fp); mergedIdx.addEntry(fp, hSha, s); }
        } else if (bSha === hSha && tSha) {
          const { content } = readObject(tSha); const d = path.dirname(fp); if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d,{recursive:true}); fs.writeFileSync(fp, content); const s = fs.statSync(fp); mergedIdx.addEntry(fp, tSha, s);
        } else if (bSha === tSha && hSha) {
          const { content } = readObject(hSha); fs.writeFileSync(fp, content); const s = fs.statSync(fp); mergedIdx.addEntry(fp, hSha, s);
        } else {
          conflicts = true;
          const hContent = hSha ? readObject(hSha).content.toString() : "";
          const tContent = tSha ? readObject(tSha).content.toString() : "";
          const merged = `<<<<<<< HEAD\n${hContent}=======\n${tContent}>>>>>>> ${target}\n`;
          fs.writeFileSync(fp, merged);
          const sha = writeObject("blob", Buffer.from(merged));
          const s = fs.statSync(fp); mergedIdx.addEntry(fp, sha, s);
          console.log(`CONFLICT (content): Merge conflict in ${fp}`);
        }
      }
      mergedIdx.writeIndex();
      if (!conflicts) {
        const treeSha = mergedIdx.writeTreeFromIndex();
        const mergeSha = createCommit(treeSha, [headSha, targetSha], `Merge branch '${target}'`);
        updateHead(mergeSha);
        console.log(`Merge made by the 'ort' strategy.`);
      } else {
        console.log("Automatic merge failed; fix conflicts and then commit the result.");
      }
    }
    break;
  }
  case "rm": {
    const file = args[1];
    if (!file) throw new Error("fatal: No pathspec given.");
    const cached = args.includes("--cached");
    const idx = new IndexManager();
    const entryIdx = idx.entries.findIndex(e => e.name === file);
    if (entryIdx === -1) throw new Error(`fatal: pathspec '${file}' did not match any files`);
    idx.entries.splice(entryIdx, 1);
    idx.writeIndex();
    if (!cached && fs.existsSync(file)) fs.unlinkSync(file);
    console.log(`rm '${file}'`);
    break;
  }
  case "restore": {
    const staged = args.includes("--staged");
    const file = args.find(a => !a.startsWith("-") && a !== "restore")!;
    if (!file) throw new Error("fatal: you must specify path(s) to restore");
    if (staged) {
      // Unstage: restore index entry from HEAD
      const headSha = resolveHead();
      if (!headSha) { throw new Error("fatal: no commits yet"); }
      const headFiles = readTreeFlat(getCommitTree(headSha));
      const idx = new IndexManager();
      if (headFiles.has(file)) {
        const sha = headFiles.get(file)!;
        const { content } = readObject(sha);
        fs.writeFileSync(file, content);
        const st = fs.statSync(file); idx.addEntry(file, sha, st);
      } else {
        idx.entries = idx.entries.filter(e => e.name !== file);
      }
      idx.writeIndex();
    } else {
      // Restore working tree from index
      const idx = new IndexManager();
      const entry = idx.entries.find(e => e.name === file);
      if (!entry) throw new Error(`error: pathspec '${file}' did not match any file(s)`);
      const { content } = readObject(entry.sha1.toString("hex"));
      fs.writeFileSync(file, content);
    }
    break;
  }
  case "reset": {
    const mode = args.includes("--soft") ? "soft" : args.includes("--hard") ? "hard" : "mixed";
    let targetSha = args.find(a => !a.startsWith("-") && a !== "reset");
    if (!targetSha) {
      // Reset to HEAD (unstage all)
      targetSha = resolveHead();
    } else if (targetSha.startsWith("HEAD~")) {
      const n = parseInt(targetSha.slice(5)) || 1;
      let sha = resolveHead();
      for (let i = 0; i < n && sha; i++) { const p = getCommitParents(sha); sha = p[0] || ""; }
      targetSha = sha;
    }
    if (!targetSha) throw new Error("fatal: Failed to resolve HEAD");
    updateHead(targetSha);
    if (mode !== "soft") {
      // Reset index
      const treeFiles = readTreeFlat(getCommitTree(targetSha));
      const idx = new IndexManager(); idx.entries = [];
      for (const [fp, sha] of treeFiles) {
        if (mode === "hard") {
          const { content } = readObject(sha);
          const d = path.dirname(fp); if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d,{recursive:true});
          fs.writeFileSync(fp, content);
        }
        if (fs.existsSync(fp)) { const st = fs.statSync(fp); idx.addEntry(fp, sha, st); }
      }
      idx.writeIndex();
    }
    console.log(`HEAD is now at ${targetSha.slice(0,7)}`);
    break;
  }
  case "tag": {
    const tagName = args[1];
    const deleteFlag = args.includes("-d");
    if (deleteFlag) {
      const name = args.find(a => a !== "-d" && a !== "tag")!;
      const tagPath = `.git/refs/tags/${name}`;
      if (!fs.existsSync(tagPath)) throw new Error(`error: tag '${name}' not found.`);
      fs.unlinkSync(tagPath);
      console.log(`Deleted tag '${name}'`);
    } else if (!tagName) {
      const tagsDir = ".git/refs/tags";
      if (fs.existsSync(tagsDir)) {
        const tags = fs.readdirSync(tagsDir).sort();
        for (const t of tags) console.log(t);
      }
    } else {
      const sha = resolveHead();
      if (!sha) throw new Error("fatal: no commits yet");
      const tagPath = `.git/refs/tags/${tagName}`;
      const tagDir = path.dirname(tagPath);
      if (!fs.existsSync(tagDir)) fs.mkdirSync(tagDir, { recursive: true });
      if (fs.existsSync(tagPath)) throw new Error(`fatal: tag '${tagName}' already exists`);
      fs.writeFileSync(tagPath, sha + "\n");
    }
    break;
  }
  case "stash": {
    const subCmd = args[1] || "push";
    const stashDir = ".git/refs/stash";
    const stashLog = ".git/stash_log";
    if (subCmd === "push" || subCmd === "save") {
      const headSha = resolveHead();
      if (!headSha) throw new Error("fatal: no commits yet");
      const idx = new IndexManager();
      if (idx.entries.length === 0) { console.log("No local changes to save"); break; }
      const treeSha = idx.writeTreeFromIndex();
      const stashSha = createCommit(treeSha, headSha ? [headSha] : [], `WIP on ${currentBranch()}`);
      // Save stash ref
      if (!fs.existsSync(path.dirname(stashDir))) fs.mkdirSync(path.dirname(stashDir), { recursive: true });
      // Append to stash log
      const stashEntries = fs.existsSync(stashLog) ? fs.readFileSync(stashLog, "utf-8").trim().split("\n").filter(Boolean) : [];
      stashEntries.unshift(stashSha);
      fs.writeFileSync(stashLog, stashEntries.join("\n") + "\n");
      // Reset working tree to HEAD
      const headTree = readTreeFlat(getCommitTree(headSha));
      idx.entries = [];
      for (const [fp, sha] of headTree) {
        const { content } = readObject(sha);
        const d = path.dirname(fp); if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d,{recursive:true});
        fs.writeFileSync(fp, content);
        const st = fs.statSync(fp); idx.addEntry(fp, sha, st);
      }
      idx.writeIndex();
      console.log(`Saved working directory and index state WIP on ${currentBranch()}: ${headSha.slice(0,7)}`);
    } else if (subCmd === "pop" || subCmd === "apply") {
      if (!fs.existsSync(stashLog)) throw new Error("No stash entries found.");
      const entries = fs.readFileSync(stashLog, "utf-8").trim().split("\n").filter(Boolean);
      if (entries.length === 0) throw new Error("No stash entries found.");
      const stashSha = entries[0];
      const stashTree = readTreeFlat(getCommitTree(stashSha));
      const idx = new IndexManager(); idx.entries = [];
      for (const [fp, sha] of stashTree) {
        const { content } = readObject(sha);
        const d = path.dirname(fp); if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d,{recursive:true});
        fs.writeFileSync(fp, content);
        const st = fs.statSync(fp); idx.addEntry(fp, sha, st);
      }
      idx.writeIndex();
      if (subCmd === "pop") { entries.shift(); fs.writeFileSync(stashLog, entries.join("\n") + "\n"); }
      console.log(`Applied stash@{0}`);
    } else if (subCmd === "list") {
      if (!fs.existsSync(stashLog)) break;
      const entries = fs.readFileSync(stashLog, "utf-8").trim().split("\n").filter(Boolean);
      entries.forEach((sha, i) => console.log(`stash@{${i}}: WIP on ${currentBranch()}: ${sha.slice(0,7)}`));
    } else if (subCmd === "drop") {
      if (!fs.existsSync(stashLog)) throw new Error("No stash entries found.");
      const entries = fs.readFileSync(stashLog, "utf-8").trim().split("\n").filter(Boolean);
      if (entries.length === 0) throw new Error("No stash entries found.");
      entries.shift();
      fs.writeFileSync(stashLog, entries.join("\n") + "\n");
      console.log("Dropped stash@{0}");
    }
    break;
  }
  case "remote": {
    const sub = args[1];
    if (sub === "add") {
      const name = args[2];
      const url = args[3];
      if (!name || !url) throw new Error("usage: mygit remote add <name> <url>");
      addRemote(name, url);
    } else if (sub === "remove" || sub === "rm") {
      const name = args[2];
      if (!name) throw new Error("usage: mygit remote remove <name>");
      removeRemote(name);
    } else {
      const verbose = args.includes("-v") || args.includes("--verbose");
      const remotes = listRemotes(verbose);
      for (const r of remotes) console.log(r);
    }
    break;
  }
  case "clone": {
    let url = args[1];
    if (!url) throw new Error("fatal: You must specify a repository to clone.");

    // Resolve local path to absolute before chdir
    if (fs.existsSync(url) && fs.statSync(url).isDirectory()) {
      url = path.resolve(url);
    }

    const dir = args[2] || path.basename(url.replace(/\.git$/, ""));


    if (fs.existsSync(dir) && fs.readdirSync(dir).length > 0) {
      console.error(`fatal: destination path '${dir}' already exists and is not an empty directory.`);
      process.exit(128);
    }
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

    // Note: We need to change CWD for clone to work in the new dir
    // But since this is a single execution, we'll just handle pathing
    process.chdir(dir);
    
    // 1. Init
    fs.mkdirSync(".git", { recursive: true });
    fs.mkdirSync(".git/objects", { recursive: true });
    fs.mkdirSync(".git/refs/heads", { recursive: true });
    fs.writeFileSync(".git/HEAD", "ref: refs/heads/main\n");
    
    // 2. Add remote
    addRemote("origin", url);
    
    // 3. Fetch
    console.log(`Cloning into '${dir}'...`);
    const { refs, symrefs } = await discoverRefs(url);
    const headRef = refs.find(r => r.name === "HEAD") || refs.find(r => r.name === "refs/heads/main") || refs[0];
    if (!headRef) throw new Error("fatal: Could not find HEAD or main branch on remote.");
    
    const packData = await fetchPack(url, [headRef.sha]);
    parsePackfile(packData);
    
    // Update local ref (avoid detached HEAD)
    const targetRef = symrefs?.get("HEAD") || "refs/heads/main";
    fs.writeFileSync(".git/HEAD", `ref: ${targetRef}\n`);
    const refPath = path.join(".git", targetRef);
    fs.mkdirSync(path.dirname(refPath), { recursive: true });
    fs.writeFileSync(refPath, headRef.sha + "\n");
    
    // 4. Checkout
    const treeSha = getCommitTree(headRef.sha);
    const files = readTreeFlat(treeSha);
    const idx = new IndexManager();
    idx.entries = [];
    for (const [fp, sha] of files) {
      const { content } = readObject(sha);
      const d = path.dirname(fp);
      if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d, { recursive: true });
      fs.writeFileSync(fp, content);
      const st = fs.statSync(fp);
      idx.addEntry(fp, sha, st);
    }
    idx.writeIndex();
    break;

  }
  case "fetch": {
    const remote = args[1] || "origin";
    const url = getRemoteUrl(remote);
    console.log(`Fetching from ${remote} (${url})`);
    
    const { refs } = await discoverRefs(url);
    const wants = refs.filter(r => r.name.startsWith("refs/heads/")).map(r => r.sha);
    if (wants.length === 0) { console.log("Already up to date."); break; }
    
    // Simplified: fetch all branch heads
    const packData = await fetchPack(url, wants);
    parsePackfile(packData);
    
    for (const ref of refs) {
      if (ref.name.startsWith("refs/heads/")) {
        const localName = ref.name.replace("refs/heads/", `refs/remotes/${remote}/`);
        updateRef(localName, ref.sha);
      }
    }
    break;
  }
  case "pull": {
    const remote = args[1] || "origin";
    const branch = args[2] || currentBranch() || "main";
    const url = getRemoteUrl(remote);
    
    // 1. Fetch
    console.log(`Fetching ${remote} ${branch}`);
    const { refs } = await discoverRefs(url);
    const targetRef = refs.find(r => r.name === `refs/heads/${branch}`);
    if (!targetRef) throw new Error(`fatal: Couldn't find remote ref refs/heads/${branch}`);
    
    const packData = await fetchPack(url, [targetRef.sha]);
    parsePackfile(packData);
    updateRef(`refs/remotes/${remote}/${branch}`, targetRef.sha);
    
    // 2. Merge (reusing our merge logic by passing the SHA)
    // We'll simulate a merge command by updating args
    args[1] = targetRef.sha; // This is a bit hacky, let's just call the logic
    
    // Re-resolve targetSha for merge
    const targetSha = targetRef.sha;
    const headSha = resolveHead();
    
    if (isAncestor(headSha, targetSha)) {
      // Fast-forward
      updateHead(targetSha);
      const treeSha = getCommitTree(targetSha);
      const files = readTreeFlat(treeSha);
      const idx = new IndexManager();
      idx.entries = [];
      for (const [fp, sha] of files) {
        const { content } = readObject(sha);
        const d = path.dirname(fp);
        if (d !== "." && !fs.existsSync(d)) fs.mkdirSync(d, { recursive: true });
        fs.writeFileSync(fp, content);
        const st = fs.statSync(fp); idx.addEntry(fp, sha, st);
      }
      idx.writeIndex();
      console.log("Fast-forward");
    } else {
      console.log("3-way merge required (non-FF). Use 'merge' manually for now.");
    }
    break;
  }
  case "mv": {
    const source = args[1];
    const destination = args[2];
    if (!source || !destination) throw new Error("usage: mygit mv <source> <destination>");
    if (!fs.existsSync(source)) throw new Error(`fatal: bad source, source=${source}`);
    
    fs.renameSync(source, destination);
    
    // Update index: remove old, add new
    const idx = new IndexManager();
    idx.entries = idx.entries.filter(e => e.name !== source);
    const sha = createBlob(destination);
    idx.addEntry(destination, sha, fs.statSync(destination));
    idx.writeIndex();
    
    console.log(`Renamed ${source} to ${destination}`);
    break;
  }
  case "clean": {
    const isForce = args.includes("-f") || args.includes("--force");
    if (!isForce) {
      console.log("fatal: clean.requireForce set and -i, -n, or -f not given; refusing to clean");
      break;
    }
    
    const ignore = new IgnoreManager();
    const headSha = resolveHead();
    const headFiles = headSha ? readTreeFlat(getCommitTree(headSha)) : new Map<string, string>();
    const idx = new IndexManager();
    const staged = idx.entries.map(e => e.name);
    
    const walk = (dir: string) => {
      const items = fs.readdirSync(dir, { withFileTypes: true });
      for (const item of items) {
        const fullPath = path.join(dir, item.name).replace(/\\/g, "/");
        const relPath = fullPath.startsWith("./") ? fullPath.slice(2) : fullPath;
        if (item.name === ".git") continue;
        if (ignore.isIgnored(relPath)) continue;
        
        if (item.isDirectory()) {
          walk(fullPath);
        } else {
          if (!staged.includes(relPath) && !headFiles.has(relPath)) {
            console.log(`Removing ${relPath}`);
            fs.unlinkSync(fullPath);
          }
        }
      }
    };
    walk(".");
    break;
  }
  case "show": {
    const target = args[1] || "HEAD";
    const sha = resolveHead(target);
    const { type, content } = readObject(sha);
    
    if (type === "commit") {
      console.log(`commit ${sha}`);
      console.log(content.toString());
      
      // Show diff if parent exists
      const parents = getCommitParents(sha);
      if (parents.length > 0) {
        const parentTree = getCommitTree(parents[0]);
        const currentTree = getCommitTree(sha);
        // Reuse diff logic here (simplified)
        console.log("\nDiff:");
        const pFiles = readTreeFlat(parentTree);
        const cFiles = readTreeFlat(currentTree);
        for (const [f, s] of cFiles) {
          if (pFiles.get(f) !== s) console.log(`modified: ${f}`);
        }
        for (const [f] of pFiles) {
          if (!cFiles.has(f)) console.log(`deleted: ${f}`);
        }
      }
    } else {
      console.log(content.toString());
    }
    break;
  }
  case "config": {
    const isGlobal = args.includes("--global");
    const key = args.find(a => a.includes(".") && !a.startsWith("-"));
    const value = args[args.indexOf(key!) + 1];
    
    if (!key) {
      console.log("usage: mygit config [--global] <key> [value]");
    } else if (value !== undefined) {
      setConfig(key, value, isGlobal);
    } else {
      const v = getConfig(key);
      if (v) console.log(v);
    }
    break;
  }
  case "push": {
    const remote = args[1] || "origin";
    const branch = args[2] || currentBranch() || "main";
    const url = getRemoteUrl(remote);
    
    console.log(`Pushing to ${remote} (${url})`);
    
    // 1. Discover remote refs to find oldSha
    const { refs } = await discoverRefs(url);
    const remoteRef = refs.find(r => r.name === `refs/heads/${branch}`);
    const oldSha = remoteRef ? remoteRef.sha : "0".repeat(40);
    const newSha = resolveHead();
    
    if (oldSha === newSha) {
      console.log("Everything up-to-date");
      break;
    }
    
    // 2. Auth (Check for GITHUB_TOKEN environment variable)
    const token = process.env.GITHUB_TOKEN;
    const username = process.env.GITHUB_USER || "git";
    const auth = token ? `${username}:${token}` : undefined;
    
    if (!auth && url.includes("github.com")) {
      console.log("Warning: No GITHUB_TOKEN found. Push might fail if authentication is required.");
    }
    
    // 3. Push
    const result = await pushPack(url, oldSha, newSha, `refs/heads/${branch}`, auth);
    console.log(result);
    break;
  }
  case "help":
  case "--help":
  case "-h": {
    console.log(`usage: mygit <command> [<args>]

These are common MyGit commands used in various situations:

start a working area
   init      Create an empty Git repository or reinitialize an existing one
   clone     Clone a repository into a new directory

work on the current change
   add       Add file contents to the index
   mv        Move or rename a file, a directory, or a symlink
   restore   Restore working tree files
   rm        Remove files from the working tree and from the index

examine the history and state
   log       Show commit logs
   status    Show the working tree status
   show      Show various types of objects
   diff      Show changes between commits, commit and working tree, etc

grow, mark and tweak your common history
   branch    List, create, or delete branches
   commit    Record changes to the repository
   merge     Join two or more development histories together
   reset     Reset current HEAD to the specified state
   tag       Create, list, delete or verify a tag object signed with GPG

collaborate
   fetch     Download objects and refs from another repository
   pull      Fetch from and integrate with another repository or a local branch
   push      Update remote refs along with associated objects
   remote    Manage set of tracked repositories
`);
    break;
  }
  case "version":
  case "-v":
  case "--version": {
    console.log("mygit version 1.0.0 (full parity)");
    break;
  }
  default: {
    if (command) {
      console.error(`mygit: '${command}' is not a mygit command. See 'mygit --help'.`);
    } else {
      console.log("usage: mygit <command> [<args>]");
    }
    process.exit(1);
  }
}


** **

### File: ts/app/packfile.ts ** **
import * as zlib from "zlib";
import * as crypto from "crypto";
import * as fs from "fs";
import * as path from "path";
import { writeObject } from "./git-helpers.js";

// ─── Packfile Parser ────────────────────────────────────────
// Git packfile format: https://git-scm.com/docs/pack-format

interface PackObject {
  type: number;      // 1=commit, 2=tree, 3=blob, 4=tag, 6=ofs_delta, 7=ref_delta
  data: Buffer;
  sha?: string;
}

const TYPE_NAMES: { [k: number]: string } = {
  1: "commit", 2: "tree", 3: "blob", 4: "tag",
};

export function parsePackfile(packData: Buffer): Map<string, { type: string; content: Buffer }> {
  const objects = new Map<string, { type: string; content: Buffer }>();
  
  // Verify header
  const sig = packData.subarray(0, 4).toString("ascii");
  if (sig !== "PACK") throw new Error(`Invalid packfile signature: ${sig}`);
  
  const version = packData.readUInt32BE(4);
  if (version !== 2 && version !== 3) throw new Error(`Unsupported packfile version: ${version}`);
  
  const numObjects = packData.readUInt32BE(8);
  let offset = 12;
  
  // First pass: extract all objects (store raw for delta resolution)
  const rawObjects: { type: number; data: Buffer; offset: number; baseRef?: Buffer; baseOfs?: number }[] = [];
  
  for (let i = 0; i < numObjects; i++) {
    const entryOffset = offset;
    
    // Read type and size (variable-length encoding)
    let byte = packData[offset++];
    const type = (byte >> 4) & 0x07;
    let size = byte & 0x0f;
    let shift = 4;
    
    while (byte & 0x80) {
      byte = packData[offset++];
      size |= (byte & 0x7f) << shift;
      shift += 7;
    }
    
    if (type === 6) {
      // OFS_DELTA: read negative offset to base
      let baseByte = packData[offset++];
      let baseOffset = baseByte & 0x7f;
      while (baseByte & 0x80) {
        baseByte = packData[offset++];
        baseOffset = ((baseOffset + 1) << 7) | (baseByte & 0x7f);
      }
      
      const result = inflateFromPack(packData, offset);
      rawObjects.push({ type, data: result.data, offset: entryOffset, baseOfs: baseOffset });
      offset = result.consumed;
    } else if (type === 7) {
      // REF_DELTA: read 20-byte base SHA
      const baseRef = packData.subarray(offset, offset + 20);
      offset += 20;
      
      const result = inflateFromPack(packData, offset);
      rawObjects.push({ type, data: result.data, offset: entryOffset, baseRef });
      offset = result.consumed;
    } else {
      // Regular object
      const result = inflateFromPack(packData, offset);
      rawObjects.push({ type, data: result.data, offset: entryOffset });
      offset = result.consumed;
    }
  }
  
  // Build offset->index map for OFS_DELTA resolution
  const offsetMap = new Map<number, number>();
  rawObjects.forEach((obj, idx) => offsetMap.set(obj.offset, idx));
  
  // Resolve objects (non-delta first, then deltas)
  const resolved: (Buffer | null)[] = new Array(rawObjects.length).fill(null);
  const resolvedTypes: (number | null)[] = new Array(rawObjects.length).fill(null);
  
  function resolveObject(idx: number): { type: number; data: Buffer } {
    if (resolved[idx] !== null) return { type: resolvedTypes[idx]!, data: resolved[idx]! };
    
    const obj = rawObjects[idx];
    
    if (obj.type !== 6 && obj.type !== 7) {
      // Regular object
      resolved[idx] = obj.data;
      resolvedTypes[idx] = obj.type;
      return { type: obj.type, data: obj.data };
    }
    
    // Delta object — resolve base first
    let baseIdx: number;
    if (obj.type === 6) {
      // OFS_DELTA
      const baseAbsOffset = obj.offset - obj.baseOfs!;
      baseIdx = offsetMap.get(baseAbsOffset)!;
      if (baseIdx === undefined) throw new Error(`OFS_DELTA base not found at offset ${baseAbsOffset}`);
    } else {
      // REF_DELTA
      const baseHex = obj.baseRef!.toString("hex");
      // Find by SHA in already-stored objects
      if (objects.has(baseHex)) {
        const base = objects.get(baseHex)!;
        const result = applyDelta(base.content, obj.data);
        const typeNum = Object.entries(TYPE_NAMES).find(([, v]) => v === base.type)?.[0];
        resolved[idx] = result;
        resolvedTypes[idx] = parseInt(typeNum || "3");
        return { type: resolvedTypes[idx]!, data: result };
      }
      // Search in rawObjects
      baseIdx = rawObjects.findIndex((_, i) => {
        if (resolved[i] === null) resolveObject(i);
        const r = resolved[i];
        if (!r) return false;
        const t = TYPE_NAMES[resolvedTypes[i]!];
        if (!t) return false;
        const header = `${t} ${r.length}\0`;
        const store = Buffer.concat([Buffer.from(header), r]);
        const sha = crypto.createHash("sha1").update(store).digest("hex");
        return sha === baseHex;
      });
      if (baseIdx === -1) throw new Error(`REF_DELTA base ${baseHex} not found`);
    }
    
    const base = resolveObject(baseIdx);
    const result = applyDelta(base.data, obj.data);
    resolved[idx] = result;
    resolvedTypes[idx] = base.type;
    return { type: base.type, data: result };
  }
  
  // Resolve all objects and store them
  process.stdout.write(`Resolving deltas: 100% (${rawObjects.length}/${rawObjects.length}), done.\n`);
  for (let i = 0; i < rawObjects.length; i++) {
    const { type, data } = resolveObject(i);

    const typeName = TYPE_NAMES[type];
    if (!typeName) continue;
    
    const sha = writeObject(typeName, data);
    objects.set(sha, { type: typeName, content: data });
  }
  
  return objects;
}

// ─── Inflate from packfile ──────────────────────────────────

function inflateFromPack(packData: Buffer, offset: number): { data: Buffer; consumed: number } {
  try {
    const result = zlib.inflateSync(packData.subarray(offset));
    
    // Binary search for the smallest input size that produces 'result'
    let low = 1;
    let high = packData.length - offset;
    let consumed = high;
    
    while (low <= high) {
      const mid = Math.floor((low + high) / 2);
      try {
        const tryResult = zlib.inflateSync(packData.subarray(offset, offset + mid));
        if (tryResult.length === result.length) {
          consumed = mid;
          high = mid - 1;
        } else {
          low = mid + 1;
        }
      } catch {
        low = mid + 1;
      }
    }
    
    return { data: result, consumed: offset + consumed };
  } catch (e: any) {
    // If inflateSync failed, try inflateRawSync as a fallback (some packfiles might vary)
    try {
      const result = zlib.inflateRawSync(packData.subarray(offset));
      let low = 1; let high = packData.length - offset; let consumed = high;
      while (low <= high) {
        const mid = Math.floor((low + high) / 2);
        try {
          const tryResult = zlib.inflateRawSync(packData.subarray(offset, offset + mid));
          if (tryResult.length === result.length) { consumed = mid; high = mid - 1; }
          else { low = mid + 1; }
        } catch { low = mid + 1; }
      }
      return { data: result, consumed: offset + consumed };
    } catch (e2) {
      throw new Error(`Inflation failed at offset ${offset}: ${e.message}`);
    }
  }
}

// ─── Delta Application ──────────────────────────────────────
// Git delta format: https://git-scm.com/docs/pack-format#_deltified_representation

export function applyDelta(base: Buffer, delta: Buffer): Buffer {
  let offset = 0;
  
  // Read base size (variable-length)
  let baseSize = 0;
  let shift = 0;
  let byte: number;
  do {
    byte = delta[offset++];
    baseSize |= (byte & 0x7f) << shift;
    shift += 7;
  } while (byte & 0x80);
  
  // Read result size (variable-length)
  let resultSize = 0;
  shift = 0;
  do {
    byte = delta[offset++];
    resultSize |= (byte & 0x7f) << shift;
    shift += 7;
  } while (byte & 0x80);
  
  const result = Buffer.alloc(resultSize);
  let resultOffset = 0;
  
  while (offset < delta.length) {
    const cmd = delta[offset++];
    
    if (cmd & 0x80) {
      // Copy from base
      let copyOffset = 0;
      let copySize = 0;
      
      if (cmd & 0x01) copyOffset = delta[offset++];
      if (cmd & 0x02) copyOffset |= delta[offset++] << 8;
      if (cmd & 0x04) copyOffset |= delta[offset++] << 16;
      if (cmd & 0x08) copyOffset |= delta[offset++] << 24;
      
      if (cmd & 0x10) copySize = delta[offset++];
      if (cmd & 0x20) copySize |= delta[offset++] << 8;
      if (cmd & 0x40) copySize |= delta[offset++] << 16;
      
      if (copySize === 0) copySize = 0x10000;
      
      base.copy(result, resultOffset, copyOffset, copyOffset + copySize);
      resultOffset += copySize;
    } else if (cmd > 0) {
      // Insert new data
      delta.copy(result, resultOffset, offset, offset + cmd);
      resultOffset += cmd;
      offset += cmd;
    } else {
      throw new Error("Unexpected delta command 0");
    }
  }
  
  return result;
}

// ─── Communication Logic ────────────────────────────────────

import { spawn } from "child_process";
import { isSsh, parseSshUrl } from "./remote.js";

export async function discoverRefs(url: string): Promise<{ refs: { sha: string; name: string }[]; caps: string[]; symrefs: Map<string, string> }> {

  // Check if it's a local path
  if (fs.existsSync(url) && fs.statSync(url).isDirectory()) {
    const gitDir = fs.existsSync(path.join(url, ".git")) ? path.join(url, ".git") : url;
    const refs: { sha: string; name: string }[] = [];
    const symrefs = new Map<string, string>();
    
    // Resolve HEAD
    if (fs.existsSync(path.join(gitDir, "HEAD"))) {
      const headContent = fs.readFileSync(path.join(gitDir, "HEAD"), "utf-8").trim();
      if (headContent.startsWith("ref: ")) {
        const target = headContent.slice(5);
        symrefs.set("HEAD", target);
        const refPath = path.join(gitDir, target);
        if (fs.existsSync(refPath)) {
          refs.push({ sha: fs.readFileSync(refPath, "utf-8").trim(), name: "HEAD" });
        }
      } else {
        refs.push({ sha: headContent, name: "HEAD" });
      }
    }

    // Walk refs/heads
    const headsDir = path.join(gitDir, "refs", "heads");
    if (fs.existsSync(headsDir)) {
      const walk = (dir: string, prefix: string) => {
        const items = fs.readdirSync(dir);
        for (const item of items) {
          const full = path.join(dir, item);
          if (fs.statSync(full).isDirectory()) {
            walk(full, `${prefix}${item}/`);
          } else {
            refs.push({ sha: fs.readFileSync(full, "utf-8").trim(), name: `refs/heads/${prefix}${item}` });
          }
        }
      };
      walk(headsDir, "");
    }
    return { refs, caps: ["multi_ack", "side-band-64k", "agent=mygit/1.0"], symrefs };

  }

  if (isSsh(url)) {
    const { host, user, path } = parseSshUrl(url);
    return new Promise((resolve, reject) => {
      const ssh = spawn("ssh", [`${user}@${host}`, `git-upload-pack '${path}'`]);
      const chunks: Buffer[] = [];
      ssh.stdout.on("data", (chunk) => chunks.push(chunk));
      ssh.on("close", () => resolve(parseRefs(Buffer.concat(chunks))));
      ssh.on("error", reject);
    });
  }
  
  url = url.replace(/\/$/, "");
  if (!url.endsWith(".git")) url += ".git";
  const resp = await fetch(`${url}/info/refs?service=git-upload-pack`);
  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${resp.statusText}`);
  return parseRefs(Buffer.from(await resp.arrayBuffer()));
}

function parseRefs(body: Buffer): { refs: { sha: string; name: string }[]; caps: string[]; symrefs: Map<string, string> } {
  const refs: { sha: string; name: string }[] = [];
  const symrefs = new Map<string, string>();

  let caps: string[] = [];
  let offset = 0;
  let firstLine = true;
  
  while (offset < body.length) {
    const lenHex = body.subarray(offset, offset + 4).toString("ascii");
    const len = parseInt(lenHex, 16);
    if (len === 0) { offset += 4; continue; }
    
    const line = body.subarray(offset + 4, offset + len).toString("utf-8").trim();
    offset += len;
    
    if (line.startsWith("# ")) continue;
    
    if (firstLine) {
      const nullIdx = line.indexOf("\0");
      if (nullIdx !== -1) {
        const refPart = line.substring(0, nullIdx);
        caps = line.substring(nullIdx + 1).split(" ");
        // Parse symrefs from caps: symref=HEAD:refs/heads/main
        for (const cap of caps) {
          if (cap.startsWith("symref=")) {
            const parts = cap.substring(7).split(":");
            if (parts.length === 2) symrefs.set(parts[0], parts[1]);
          }
        }
        const [sha, name] = refPart.split(" ");
        if (sha && name) refs.push({ sha, name });
      } else {
        const [sha, name] = line.split(" ");
        if (sha && name) refs.push({ sha, name });
      }
      firstLine = false;
    } else {
      const [sha, name] = line.split(" ");
      if (sha && name && sha.length === 40) refs.push({ sha, name });
    }
  }
  return { refs, caps, symrefs };
}


export async function fetchPack(url: string, wants: string[], haves: string[] = []): Promise<Buffer> {
  // Check if it's a local path
  if (fs.existsSync(url) && fs.statSync(url).isDirectory()) {
    const srcGit = fs.existsSync(path.join(url, ".git")) ? path.join(url, ".git") : url;
    const srcObjects = path.join(srcGit, "objects");
    const destObjects = ".git/objects";
    
    // Copy objects directory recursively
    const copyDir = (src: string, dest: string) => {
      if (!fs.existsSync(dest)) fs.mkdirSync(dest, { recursive: true });
      const entries = fs.readdirSync(src, { withFileTypes: true });
      for (const entry of entries) {
        const srcPath = path.join(src, entry.name);
        const destPath = path.join(dest, entry.name);
        if (entry.isDirectory()) {
          copyDir(srcPath, destPath);
        } else {
          if (!fs.existsSync(destPath)) fs.copyFileSync(srcPath, destPath);
        }
      }
    };
    if (fs.existsSync(srcObjects)) copyDir(srcObjects, destObjects);
    
    // Return a dummy empty packfile
    const header = Buffer.alloc(12);
    header.write("PACK", 0);
    header.writeUInt32BE(2, 4);
    header.writeUInt32BE(0, 8);
    const checksum = crypto.createHash("sha1").update(header).digest();
    return Buffer.concat([header, checksum]);
  }



  let requestBody = "";

  for (let i = 0; i < wants.length; i++) {
    const line = i === 0 ? `want ${wants[i]} no-done side-band-64k\n` : `want ${wants[i]}\n`;
    requestBody += pktLine(line);
  }
  requestBody += "0000";
  for (const have of haves) requestBody += pktLine(`have ${have}\n`);
  requestBody += pktLine("done\n");

  if (isSsh(url)) {
    const { host, user, path } = parseSshUrl(url);
    return new Promise((resolve, reject) => {
      const ssh = spawn("ssh", [`${user}@${host}`, `git-upload-pack '${path}'`]);
      const chunks: Buffer[] = [];
      ssh.stdin.write(requestBody);
      ssh.stdout.on("data", (chunk) => chunks.push(chunk));
      ssh.on("close", () => resolve(extractPackData(Buffer.concat(chunks))));
      ssh.on("error", reject);
    });
  }
  
  url = url.replace(/\/$/, "");
  if (!url.endsWith(".git")) url += ".git";
  const resp = await fetch(`${url}/git-upload-pack`, {
    method: "POST",
    headers: { "Content-Type": "application/x-git-upload-pack-request" },
    body: requestBody,
  });
  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${resp.statusText}`);
  const packData = Buffer.from(await resp.arrayBuffer());
  const sizeKiB = (packData.length / 1024).toFixed(2);
  process.stdout.write(`Receiving objects: 100% ... ${sizeKiB} KiB, done.\n`);
  return extractPackData(packData);

}


function extractPackData(data: Buffer): Buffer {
  const chunks: Buffer[] = [];
  let offset = 0;
  while (offset < data.length) {
    if (offset + 4 > data.length) break;
    const lenHex = data.subarray(offset, offset + 4).toString("ascii");
    const len = parseInt(lenHex, 16);
    if (len <= 4) { offset += 4; continue; }
    const payload = data.subarray(offset + 4, offset + len);
    offset += len;
    const band = payload[0];
    if (band === 1) {
      chunks.push(payload.subarray(1));
    } else if (band === 2) {
      // Sideband 2: Progress data (e.g., "remote: Enumerating objects: 40, done.")
      process.stderr.write(payload.subarray(1).toString());
    } else if (band === 3) {
      throw new Error(`Remote error: ${payload.subarray(1).toString()}`);
    } else if (payload.subarray(0, 4).toString("ascii") === "PACK") {
      chunks.push(payload);
    }

  }
  const result = Buffer.concat(chunks);
  const packIdx = result.indexOf("PACK");
  if (packIdx === -1) throw new Error("No packfile found in response");
  return result.subarray(packIdx);
}

function pktLine(data: string): string {
  const len = data.length + 4;
  return len.toString(16).padStart(4, "0") + data;
}

// ─── Packfile Creation ──────────────────────────────────────

export function createPackfile(objectShas: string[], sourceDir?: string): Buffer {
  const allObjects = sourceDir ? getAllReachableObjects(objectShas, sourceDir) : objectShas;
  
  const header = Buffer.alloc(12);
  header.write("PACK", 0);
  header.writeUInt32BE(2, 4);
  header.writeUInt32BE(allObjects.length, 8);
  
  const bodyChunks: Buffer[] = [header];
  for (const sha of allObjects) {
    const { type, content } = readObjectFromPath(sha, sourceDir);
    const typeNum = Object.entries(TYPE_NAMES).find(([, v]) => v === type)?.[0];
    if (!typeNum) continue;
    
    let size = content.length;
    let byte = (parseInt(typeNum) << 4) | (size & 0x0f);
    size >>= 4;
    const sizeBytes: number[] = [];
    while (size > 0) {
      sizeBytes.push((byte | 0x80));
      byte = size & 0x7f;
      size >>= 7;
    }
    sizeBytes.push(byte);
    bodyChunks.push(Buffer.from(sizeBytes));
    bodyChunks.push(zlib.deflateSync(content));
  }
  
  const packWithoutChecksum = Buffer.concat(bodyChunks);
  const checksum = crypto.createHash("sha1").update(packWithoutChecksum).digest();
  return Buffer.concat([packWithoutChecksum, checksum]);
}

function getAllReachableObjects(startShas: string[], sourceDir: string): string[] {
  const reachable = new Set<string>();
  const visited = new Set<string>();
  const queue = [...startShas];
  
  while (queue.length > 0) {
    const sha = queue.shift()!;
    if (visited.has(sha) || sha === "0".repeat(40)) continue;
    visited.add(sha);
    
    try {
      const { type, content } = readObjectFromPath(sha, sourceDir);
      reachable.add(sha);
      
      if (type === "commit") {
        const text = content.toString("utf-8");
        const treeMatch = text.match(/^tree ([a-f0-9]{40})/);
        if (treeMatch) queue.push(treeMatch[1]);
        const parents = text.split("\n").filter(l => l.startsWith("parent ")).map(l => l.slice(7));
        queue.push(...parents);
      } else if (type === "tree") {
        let offset = 0;
        while (offset < content.length) {
          let spaceIdx = offset;
          while (content[spaceIdx] !== 0x20) spaceIdx++;
          let nameEnd = spaceIdx + 1;
          while (content[nameEnd] !== 0) nameEnd++;
          const entrySha = content.subarray(nameEnd + 1, nameEnd + 21).toString("hex");
          queue.push(entrySha);
          offset = nameEnd + 21;
        }
      }
    } catch (e) {
      console.error(`Warning: could not read object ${sha} during reachability check`);
    }
  }
  return Array.from(reachable);
}


function readObjectFromPath(sha: string, sourceDir?: string): { type: string; content: Buffer } {
  const gitDir = sourceDir ? (fs.existsSync(path.join(sourceDir, ".git")) ? path.join(sourceDir, ".git") : sourceDir) : ".git";
  const objPath = path.join(gitDir, "objects", sha.slice(0, 2), sha.slice(2));
  if (!fs.existsSync(objPath)) throw new Error(`fatal: Not a valid object name ${sha} in ${gitDir}`);
  const compressed = fs.readFileSync(objPath);
  const decompressed = zlib.inflateSync(compressed);
  const nullIdx = decompressed.indexOf(0);
  const header = decompressed.subarray(0, nullIdx).toString("utf-8");
  const [type] = header.split(" ");
  const content = decompressed.subarray(nullIdx + 1);
  return { type, content };
}


export async function pushPack(url: string, oldSha: string, newSha: string, refName: string, auth?: string): Promise<string> {
  const objectsToSend = new Set<string>();
  const visited = new Set<string>();
  const queue = [newSha];
  
  while (queue.length > 0) {
    const sha = queue.shift()!;
    if (sha === oldSha || visited.has(sha) || sha === "0".repeat(40)) continue;
    visited.add(sha);
    const { type, content } = readObject(sha);
    objectsToSend.add(sha);
    if (type === "commit") {
      const treeMatch = content.toString().match(/^tree ([a-f0-9]{40})/);
      if (treeMatch) queue.push(treeMatch[1]);
      const parents = content.toString().split("\n").filter(l => l.startsWith("parent ")).map(l => l.slice(7));
      queue.push(...parents);
    } else if (type === "tree") {
      const files = readTreeFlat(sha);
      for (const [, fsha] of files) objectsToSend.add(fsha);
    }
  }
  
  const packData = createPackfile(Array.from(objectsToSend));
  const command = `${oldSha} ${newSha} ${refName}\0 report-status side-band-64k\n`;
  const body = Buffer.concat([Buffer.from(pktLine(command)), Buffer.from("0000"), packData]);

  if (isSsh(url)) {
    const { host, user, path } = parseSshUrl(url);
    return new Promise((resolve, reject) => {
      const ssh = spawn("ssh", [`${user}@${host}`, `git-receive-pack '${path}'`]);
      const chunks: Buffer[] = [];
      ssh.stdin.write(body);
      ssh.stdout.on("data", (chunk) => chunks.push(chunk));
      ssh.on("close", () => resolve(Buffer.concat(chunks).toString()));
      ssh.on("error", reject);
    });
  }
  
  url = url.replace(/\/$/, "");
  if (!url.endsWith(".git")) url += ".git";
  const headers: Record<string, string> = {
    "Content-Type": "application/x-git-receive-pack-request",
    "Accept": "application/x-git-receive-pack-result",
  };
  if (auth) headers["Authorization"] = `Basic ${Buffer.from(auth).toString("base64")}`;
  
  const resp = await fetch(`${url}/git-receive-pack`, { method: "POST", headers, body });
  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${resp.statusText}`);
  return await resp.text();
}

// Re-importing readObject and others if needed by the module
import { readObject, readTreeFlat } from "./git-helpers.js";

** **

### File: ts/app/remote.ts ** **
import * as fs from "fs";
import * as path from "path";

export function isSsh(url: string): boolean {
  return url.startsWith("ssh://") || (url.includes("@") && url.includes(":"));
}

export function parseSshUrl(url: string): { host: string; user: string; path: string } {
  if (url.startsWith("ssh://")) {
    const match = url.match(/^ssh:\/\/([^@]+)@([^/]+)\/(.+)$/);
    if (!match) throw new Error(`Invalid SSH URL: ${url}`);
    return { user: match[1], host: match[2], path: match[3] };
  } else {
    const match = url.match(/^([^@]+)@([^:]+):(.+)$/);
    if (!match) throw new Error(`Invalid SSH URL: ${url}`);
    return { user: match[1], host: match[2], path: match[3] };
  }
}

// ─── Remote Config ──────────────────────────────────────────

const configPath = ".git/config";

interface RemoteConfig {
  [name: string]: { url: string; fetch: string };
}

function readConfig(): RemoteConfig {
  const remotes: RemoteConfig = {};
  if (!fs.existsSync(configPath)) return remotes;

  const content = fs.readFileSync(configPath, "utf-8");
  const lines = content.split("\n");
  let currentRemote = "";

  for (const line of lines) {
    const sectionMatch = line.match(/^\[remote "(.+)"\]$/);
    if (sectionMatch) {
      currentRemote = sectionMatch[1];
      remotes[currentRemote] = { url: "", fetch: "" };
    } else if (currentRemote && line.trim().startsWith("url = ")) {
      remotes[currentRemote].url = line.trim().slice(6);
    } else if (currentRemote && line.trim().startsWith("fetch = ")) {
      remotes[currentRemote].fetch = line.trim().slice(8);
    } else if (line.startsWith("[")) {
      currentRemote = "";
    }
  }
  return remotes;
}

function writeConfig(remotes: RemoteConfig) {
  let content = "";
  // Preserve non-remote sections
  if (fs.existsSync(configPath)) {
    const existing = fs.readFileSync(configPath, "utf-8");
    const lines = existing.split("\n");
    let skip = false;
    for (const line of lines) {
      if (line.match(/^\[remote "/)) { skip = true; continue; }
      if (skip && line.startsWith("[")) { skip = false; }
      if (skip) continue;
      content += line + "\n";
    }
  }

  for (const [name, config] of Object.entries(remotes)) {
    content += `[remote "${name}"]\n`;
    content += `\turl = ${config.url}\n`;
    content += `\tfetch = ${config.fetch}\n`;
  }

  fs.writeFileSync(configPath, content);
}

export function addRemote(name: string, url: string) {
  const remotes = readConfig();
  if (remotes[name]) throw new Error(`fatal: remote ${name} already exists.`);
  remotes[name] = { url, fetch: `+refs/heads/*:refs/remotes/${name}/*` };
  writeConfig(remotes);
}

export function removeRemote(name: string) {
  const remotes = readConfig();
  if (!remotes[name]) throw new Error(`fatal: No such remote: '${name}'`);
  delete remotes[name];
  writeConfig(remotes);
}

export function listRemotes(verbose: boolean): string[] {
  const remotes = readConfig();
  const result: string[] = [];
  for (const [name, config] of Object.entries(remotes)) {
    if (verbose) {
      result.push(`${name}\t${config.url} (fetch)`);
      result.push(`${name}\t${config.url} (push)`);
    } else {
      result.push(name);
    }
  }
  return result;
}

export function getRemoteUrl(name: string): string {
  const remotes = readConfig();
  if (!remotes[name]) throw new Error(`fatal: '${name}' does not appear to be a git repository`);
  return remotes[name].url;
}

** **

### File: ts/package.json ** **
{
  "name": "my-own-git",
  "version": "1.0.0",
  "description": "My own Git implementation in TypeScript",
  "type": "module",
  "bin": {
    "mygit": "./mygit"
  },
  "scripts": {
    "git": "./mygit",
    "dev": "bun run app/main.ts"
  },
  "devDependencies": {
    "@types/bun": "latest"
  }
}

** **

### File: ts/tsconfig.json ** **
{
  "compilerOptions": {
    // Enable latest features
    "lib": ["ESNext", "DOM"],
    "target": "ESNext",
    "module": "ESNext",
    "moduleDetection": "force",
    "jsx": "react-jsx",
    "allowJs": true,

    // Bundler mode
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "noEmit": true,

    // Best practices
    "strict": true,
    "skipLibCheck": true,
    "noFallthroughCasesInSwitch": true,

    // Some stricter flags (disabled by default)
    "noUnusedLocals": false,
    "noUnusedParameters": false,
    "noPropertyAccessFromIndexSignature": false
  }
}

** **

### File: ts/README.md ** **
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

** **

### File: ts/RELEASING.md ** **
# Sida loo Releas-gareeyo MyGit (Distribution Guide)

Hambalyo! Hadda oo MyGit uu dhammaystiran yahay, halkan waa talaabooyinka aad u qaadayso si aad dadka kale ugu gudbiso.

## 1. Dhalinta Hal Fayl (Single Executable)
Halkii aad dadka u diri lahayd code-ka oo dhan, waxaad u diraysaa hal fayl oo ay ordon karaan.

### A. Isticmaalka Bun (Aad u dhakhso badan)
Haddii aad laptop-kaaga ku haysato Bun:
```bash
# Windows
bun build app/main.ts --compile --outfile mygit-windows.exe

# Linux
bun build app/main.ts --compile --outfile mygit-linux

# macOS
bun build app/main.ts --compile --outfile mygit-macos
```

### B. Isticmaalka `pkg` (Habka Node.js)
Haddii aad rabto inaad isticmaasho Node.js standard:
1. Install: `npm install -g pkg`
2. Build: `pkg .`
*Kani wuxuu kuu soo saarayaa 3 fayl oo kala ah Windows, Linux, iyo Mac.*

## 2. Ku Publish-garaynta NPM
Tani waa habka ugu habboon ee dadka developers-ka ah ay ku soo degsan karaan.
1. Gal [npmjs.com](https://www.npmjs.com) oo akoon ka samayso.
2. Terminal-ka ku qor: `npm login`
3. Markaad gasho, qor: `npm publish`
4. Dadka waxay ku soo degsan karaan: `npm install -g my-own-git`

## 3. Sameynta Installer (.msi ama .exe setup)
Haddii aad rabto "Setup Wizard":
*   **Windows**: Isticmaal [Inno Setup](https://jrsoftware.org/isinfo.php). Waa bilaash. Waxaad siinaysaa `mygit-windows.exe`-ga aad dhashay, isna wuxuu kuu samaynayaa `setup.exe`.
*   **Mac**: Isticmaal [Homebrew](https://brew.sh). Waxaad samaynaysaa wax loo yaqaan "Formula" si dadku u dhahaan `brew install mygit`.

## 4. GitHub Releases
Habka ugu caansan:
1. Code-kaaga geli GitHub.
2. Tag qaybta **Releases**.
3. Upload-garee faylasha aad dhashay (`mygit-windows.exe`, `mygit-linux`, iwm).
4. Dadka waxay si toos ah uga soo degsanayaan GitHub.

---
**Waa diyaar sxb! Hadda MyGit diyaar ayuu u yahay inuu caalamka ku faafo.** 🚀

** **

### File: ts/scratch_test_pack.ts ** **
import { createPackfile, parsePackfile } from "./app/packfile.js";
import { resolveHead, readObject } from "./app/git-helpers.js";
import * as fs from "fs";

// This script must be run inside a git repo initialized by mygit
const head = resolveHead();
if (!head) {
  console.log("No HEAD found. Run ./mygit init and commit something first.");
  process.exit(1);
}

console.log("Creating packfile for HEAD:", head);
const pack = createPackfile([head]);
console.log("Packfile created, length:", pack.length);

console.log("Attempting to parse the created packfile...");
const objects = parsePackfile(pack);
console.log("Parsed objects count:", objects.size);
if (objects.has(head)) {
  console.log("SUCCESS: Packfile contains HEAD commit.");
} else {
  console.log("FAILURE: Packfile does not contain HEAD commit.");
}

** **

### File: ts/.gitignore ** **
node_modules/

** **

### File: ts/.gitattributes ** **
* text=auto

** **

