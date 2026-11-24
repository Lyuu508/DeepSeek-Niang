export const USER_FRAGMENT_SEPARATOR = '\n[连续消息]\n';
const REPLY_SEPARATOR = '\n[气泡]\n';

export function messageParts(role: string, content: string): string[] {
  const parts = content.split(role === 'user' ? USER_FRAGMENT_SEPARATOR : REPLY_SEPARATOR);
  if (parts.length === 1 || parts.length > 3) return [content];
  if (role === 'assistant' && (/(?:^|\n)\s*(?:```|~~~|[|#]|[-*] |\d+\. )/.test(content)
      || parts.some(part => Array.from(part).length > 180))) return [content];
  return parts.filter(part => part.trim()).length ? parts.filter(part => part.trim()) : [content];
}
