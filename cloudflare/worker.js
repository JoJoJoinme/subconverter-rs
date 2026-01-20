
import wasmModule from './pkg/libsubconverter_bg.wasm';
import init, { sub_process_wasm, init_settings_wasm, init_panic_hook } from './pkg/libsubconverter.js';

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

                // Initialize panic hook for better error messages
                try {
                    init_panic_hook();
                } catch (e) {
                    console.log("Failed to init panic hook (maybe already set):", e);
                }

                initialized = true;
            }

            // Set global environment for the WASM bindings
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
                    queryObj[key] = value;
                }
            }

            // Call subconverter
            const queryJson = JSON.stringify(queryObj);

            let responseJson;
            try {
                responseJson = await sub_process_wasm(queryJson);
            } catch (e) {
                 // WASM errors might be objects or strings
                 throw new Error(`WASM Error: ${typeof e === 'string' ? e : JSON.stringify(e)}`);
            }

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
            const msg = e instanceof Error ? e.message : (typeof e === 'string' ? e : JSON.stringify(e));
            return new Response(`Error: ${msg}\nStack: ${e.stack || 'none'}`, { status: 500 });
        }
    }
};
