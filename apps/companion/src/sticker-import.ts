import { readChatImage, type ChatImage } from './ChatAttachment';

const moodTags: Readonly<Record<string,string>> = {
  happy: '开心', angry: '生气', sad: '失落', shy: '害羞', confused: '好奇', daily: '平静',
};

export type ImportedSticker = { image: ChatImage; tag: string };

/** Match the upstream remote-pack manifest with files the user selects locally. */
export async function readDshPack(files: File[], capacity: number): Promise<ImportedSticker[]> {
  const manifests = files.filter(file => file.name.toLowerCase().endsWith('.json'));
  const pictures = files.filter(file => !manifests.includes(file));
  if (manifests.length !== 1 || manifests[0].size > 2_000_000 ||
      !pictures.length || pictures.length > Math.min(capacity, 24)) throw new Error('DSH_PACK_FILES');
  const manifest: unknown = JSON.parse(await manifests[0].text());
  if (!manifest || typeof manifest !== 'object' || !('memes' in manifest) ||
      !Array.isArray(manifest.memes) || manifest.memes.length > 500) throw new Error('DSH_PACK_MANIFEST');
  const matches = new Map<string,{ caption: string; tag: string }>();
  const selected = new Set(pictures.map(file => file.name));
  if (selected.size !== pictures.length) throw new Error('DSH_PACK_DUPLICATE');
  for (const value of manifest.memes) {
    if (!value || typeof value !== 'object' || typeof value.tag !== 'string' ||
        !/^[a-z0-9_-]{1,20}$/.test(value.tag) || typeof value.url !== 'string') continue;
    let filename: string;
    try {
      const url = new URL(value.url);
      if (!['http:', 'https:'].includes(url.protocol)) continue;
      filename = typeof value.file === 'string' ? value.file : decodeURIComponent(url.pathname.split('/').at(-1) ?? '');
    } catch { continue; }
    if (!selected.has(filename)) continue;
    if (matches.has(filename)) throw new Error('DSH_PACK_DUPLICATE');
    const caption = typeof value.caption === 'string'
      ? value.caption.replace(/[\u0000-\u001f\u007f]/g, ' ').trim().slice(0, 80)
      : '';
    matches.set(filename, { caption, tag: Object.hasOwn(moodTags,value.tag)?moodTags[value.tag]:value.tag });
  }
  if (matches.size !== selected.size) throw new Error('DSH_PACK_MISSING');
  const entries: ImportedSticker[] = [];
  for (const file of pictures) {
    const metadata = matches.get(file.name)!;
    const image = await readChatImage(file);
    entries.push({ image: {...image, name: metadata.caption || image.name}, tag: metadata.tag });
  }
  return entries;
}
