# Subconverter for Cloudflare Workers

This directory contains the configuration and build scripts for deploying subconverter-rs to Cloudflare Workers.

## Prerequisites

- [Rust & Cargo](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) (for Wrangler)
- [Wrangler](https://developers.cloudflare.com/workers/wrangler/install-and-update/) (`npm install -g wrangler`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

## Directory Structure

- `worker.js`: The main entry point for the Cloudflare Worker. It initializes the WASM module and handles requests.
- `wrangler.toml`: Configuration file for Cloudflare Workers.
- `pkg/`: Generated WASM and JS files (after build).

## Build and Deploy

### Automatic Deploy (GitHub Actions)

This repository includes `.github/workflows/cloudflare-deploy.yml`.
It deploys to Cloudflare Workers automatically on push to `main`.

Required repository secrets:

- `CLOUDFLARE_API_TOKEN`
- `CLOUDFLARE_ACCOUNT_ID`

1.  **Build the WASM module:**

    From the root of the repository, run:
    ```bash
    ./scripts/build-cloudflare.sh
    ```
    This script compiles the Rust code into WebAssembly with the `cloudflare` feature enabled.

2.  **Configure Wrangler:**

    Edit `cloudflare/wrangler.toml` to set your KV namespace ID.
    ```toml
    [[kv_namespaces]]
    binding = "KV"
    id = "YOUR_KV_NAMESPACE_ID"
    ```
    You can create a KV namespace with:
    ```bash
    wrangler kv:namespace create SUB_KV
    ```

3.  **Deploy to Cloudflare:**

    Navigate to the `cloudflare` directory and run:
    ```bash
    cd cloudflare
    wrangler deploy
    ```

## Usage

Once deployed, you can access the subconverter via your Worker's URL:

```
https://your-worker.your-subdomain.workers.dev/sub?target=clash&url=...
```

The parameters are compatible with the standard subconverter API.
