import * as fs from "fs";
import * as zlib from "zlib";

export function cmdLsTree(sha: string, nameOnly: boolean) {
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
