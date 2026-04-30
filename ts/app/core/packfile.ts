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
