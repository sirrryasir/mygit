import * as fs from "fs";
import * as zlib from "zlib";

export function cmdCatFile(sha: string) {
  const dir = sha.slice(0, 2);
  const file = sha.slice(2);
  const filePath = `.git/objects/${dir}/${file}`;
  
  const compressed = fs.readFileSync(filePath);
  const decompressed = zlib.unzipSync(compressed);
  
  const nullByteIndex = decompressed.indexOf(0);
  const content = decompressed.subarray(nullByteIndex + 1);
  
  process.stdout.write(content);
}
