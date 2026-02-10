import { LRUCache } from './components/LRU';

export const avatarCache = new LRUCache<string, string>(100, (_key: string, value: string) => {
    URL.revokeObjectURL(value);
});

export const attachedCache = new LRUCache<string, string>(200, (_key: string, value: string) => {
    URL.revokeObjectURL(value);
});

export const emojiCache = new LRUCache<string, string>(20, (_key: string, value: string) => {
    URL.revokeObjectURL(value);
});
