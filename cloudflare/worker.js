
import wasmModule from './pkg/libsubconverter_bg.wasm';
import init, { sub_process_wasm, init_settings_wasm } from './pkg/libsubconverter.js';

let initialized = false;

export default {
    async fetch(request, env, ctx) {
        // Handle CORS
        if (request.method === "OPTIONS") {
            return new Response(null, {
                headers: {
                    "Access-Control-Allow-Origin": "*",
                    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
                    "Access-Control-Allow-Headers": "Content-Type",
                },
            });
        }

        try {
            if (!initialized) {
                // Initialize WASM module
                await init(wasmModule);

                // Initialize settings (optional, if needed)
                // await init_settings_wasm("");

                initialized = true;
            }

            // Set global environment for the WASM bindings
            // Note: Cloudflare Workers bindings are typically consistent per-isolate.
            // Assigning to globalThis is safe for these stateless bindings.
            globalThis.SUB_ENV = env;
            globalThis.SUB_KV = env.KV;

            const url = new URL(request.url);

            // Construct query object from URL search params
            const queryObj = {};
            for (const [key, value] of url.searchParams) {
                if (key === 'url') {
                    if (queryObj[key]) {
                        queryObj[key] += '|' + value;
                    } else {
                        queryObj[key] = value;
                    }
                } else {
                    // For other keys, last value wins (or could also handle arrays if needed)
                    queryObj[key] = value;
                }
            }

            // Call subconverter
            const queryJson = JSON.stringify(queryObj);
            const responseJson = await sub_process_wasm(queryJson);

            // Parse response
            const responseData = JSON.parse(responseJson);

            // Construct Headers
            const headers = new Headers();
            if (responseData.headers) {
                for (const [key, value] of Object.entries(responseData.headers)) {
                    headers.set(key, value);
                }
            }
            // Always set CORS
            headers.set("Access-Control-Allow-Origin", "*");

            return new Response(responseData.content, {
                status: responseData.status_code || 200,
                headers: headers
            });

        } catch (e) {
            return new Response(`Error: ${e.message}`, { status: 500 });
        }
    }
};
