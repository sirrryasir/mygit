import * as fs from "fs";
import * as crypto from "crypto";
import * as zlib from "zlib";

export function cmdHashObject(file: string, write: boolean) {
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
