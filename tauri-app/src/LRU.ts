// --- LRU Cache for images ---
export class LRUCache<K, V> {
    private maxSize: number;
    private cache: Map<K, V>;
    private onEvict: ((key: K, value: V) => void) | null;

    constructor(maxSize: number, onEvict: ((key: K, value: V) => void) | null = null) {
        this.maxSize = maxSize;
        this.cache = new Map<K, V>();
        this.onEvict = onEvict;
    }

    get(key: K): V | null {
        if (!this.cache.has(key)) {
            return null;
        }
        const value = this.cache.get(key)!;
        this.cache.delete(key);
        this.cache.set(key, value); // move to end
        return value;
    }

    set(key: K, value: V): void {
        if (this.cache.has(key)) {
            this.cache.delete(key);
        } else if (this.cache.size >= this.maxSize) {
            const oldestKey = this.cache.keys().next().value!;
            const evictedValue = this.cache.get(oldestKey);
            this.cache.delete(oldestKey);
            if (this.onEvict && evictedValue) {
                this.onEvict(oldestKey, evictedValue);
            }
        }
        this.cache.set(key, value);
    }
}