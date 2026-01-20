
// Cloudflare Workers implementation of kv_bindings.js
// This file is swapped in during build for Cloudflare target.

// We expect the worker script to set these globals before calling WASM functions.
// globalThis.SUB_ENV - The environment object (bindings)
// globalThis.SUB_KV - The KV Namespace binding

export function getenv(name) {
    if (globalThis.SUB_ENV && globalThis.SUB_ENV[name]) {
        return globalThis.SUB_ENV[name];
    }
    return "";
}

// Dummy function required by some bindings
export function dummy() {
    return "dummy";
}

// --- KV Storage ---

async function getKv() {
    if (!globalThis.SUB_KV) {
        throw new Error("SUB_KV not set in globalThis. Make sure to set it in the worker fetch handler.");
    }
    return globalThis.SUB_KV;
}

export async function kv_get(key) {
    try {
        const kv = await getKv();
        const value = await kv.get(key, { type: 'arrayBuffer' });
        return value ? new Uint8Array(value) : null;
    } catch (error) {
        console.error(`KV get error for ${key}:`, error);
        throw error;
    }
}

export async function kv_get_text(key) {
    try {
        const kv = await getKv();
        const value = await kv.get(key, { type: 'text' });
        return value === null ? undefined : value;
    } catch (error) {
        console.error(`KV get_text error for ${key}:`, error);
        throw error;
    }
}

export async function kv_set(key, value) {
    try {
        const kv = await getKv();
        // value is Uint8Array
        await kv.put(key, value);
    } catch (error) {
        console.error(`KV set error for ${key}:`, error);
        throw error;
    }
}

export async function kv_set_text(key, value) {
    try {
        const kv = await getKv();
        // value is string
        await kv.put(key, value);
    } catch (error) {
        console.error(`KV set_text error for ${key}:`, error);
        throw error;
    }
}

export async function kv_exists(key) {
    try {
        const kv = await getKv();
        // Cloudflare KV doesn't have exists, use list with limit 1
        const list = await kv.list({ prefix: key, limit: 1 });
        // Check if we found the key exactly
        for (const k of list.keys) {
            if (k.name === key) return 1;
        }
        return 0;
    } catch (error) {
        console.error(`KV exists error for ${key}:`, error);
        return 0;
    }
}

export async function kv_list(prefix) {
    try {
        const kv = await getKv();
        const list = await kv.list({ prefix: prefix });
        return list.keys.map(k => k.name);
    } catch (error) {
        console.error(`KV list error for prefix ${prefix}:`, error);
        return [];
    }
}

export async function kv_del(key) {
    try {
        const kv = await getKv();
        await kv.delete(key);
    } catch (error) {
        console.error(`KV del error for ${key}:`, error);
    }
}

// These functions are kept for compatibility but rely on getKv which returns the KV Namespace
// direct usage of 'getKv' export might not be useful in this context as the API differs from Vercel KV adapter
// But we export it just in case.
export { getKv };

export let localStorageMap = new Map(); // Not used but exported for interface compatibility

export async function migrateStorage(oldVersion, newVersion) {
    // No-op for now
}

// --- Fetch & HTTP ---

export async function fetch_url(url) {
    return await fetch(url);
}

export async function wasm_fetch_with_request(url, options) {
    // Cloudflare Workers support global fetch with options
    return await fetch(url, options);
}

export async function response_status(response) {
    return response.status;
}

export async function response_bytes(response) {
    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
}

export async function response_headers(response) {
    const headers = {};
    for (const [key, value] of response.headers.entries()) {
        headers[key] = value;
    }
    return headers;
}

export async function response_text(response) {
    return await response.text();
}
