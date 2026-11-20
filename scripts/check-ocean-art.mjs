import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const relative = 'apps/companion/public/branding/ocean';
const { assets } = JSON.parse(fs.readFileSync(path.join(root, 'assets/branding/ocean-art.json')));
const expected = [
  'avatar-standard','avatar-chibi','happy','shy','sad','angry','thinking','sleepy',
  'sleeping','peace','wave','resting','peeking','waiting','portrait-full','portrait-half',
  'scene-chat','scene-settings','scene-plugins','scene-video','scene-night',
];

function metadata(contents) {
  assert.equal(contents.toString('ascii',0,4), 'RIFF');
  assert.equal(contents.toString('ascii',8,12), 'WEBP');
  const kind = contents.toString('ascii',12,16);
  if (kind === 'VP8X') {
    return { width: contents.readUIntLE(24,3)+1, height: contents.readUIntLE(27,3)+1, alpha: !!(contents[20] & 16) };
  }
  if (kind === 'VP8L') {
    assert.equal(contents[20],47);
    const packed = contents.readUInt32LE(21);
    return { width: (packed & 16383)+1, height: ((packed>>>14)&16383)+1, alpha: !!(packed & (1<<28)) };
  }
  assert.equal(kind,'VP8 ');
  assert.equal(contents.readUIntLE(23,3),0x2a019d);
  return { width: contents.readUInt16LE(26)&16383, height: contents.readUInt16LE(28)&16383, alpha:false };
}

assert.deepEqual(Object.keys(assets).sort(), expected.sort());
assert.deepEqual(fs.readdirSync(path.join(root,relative)).sort(), expected.map(name=>name+'.webp').sort());
let bytes = 0;
for (const [name, asset] of Object.entries(assets)) {
  assert.equal(asset.path, relative+'/'+name+'.webp');
  const contents = fs.readFileSync(path.join(root, asset.path));
  assert.equal(asset.format, 'webp');
  assert(asset.role.length > 4);
  assert.equal(contents.length, asset.bytes);
  assert.equal(createHash('sha256').update(contents).digest('hex'), asset.sha256);
  assert.deepEqual(metadata(contents), {width:asset.width,height:asset.height,alpha:asset.transparent});
  bytes += contents.length;
}
assert(bytes < 1_500_000);
console.log(JSON.stringify({ verified: Object.keys(assets).length, bytes, transparent: Object.values(assets).filter(asset=>asset.transparent).length }));
