import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { readdirSync } from 'node:fs'

const __dirname = dirname(fileURLToPath(import.meta.url))

const inputs = Object.fromEntries(
    readdirSync(resolve(__dirname, 'src/entrypoints'))
        .filter(file => file.endsWith('.ts'))
        .map(file => [file.replace('.ts', ''), resolve(__dirname, 'src/entrypoints', file)])
)

// https://vite.dev/config/
export default defineConfig({
    plugins: [svelte()],
    build: {
        rollupOptions: {
            treeshake: true,
            input: inputs,
            output: {
                entryFileNames: "[name].js",
                chunkFileNames: "[name].js",
                manualChunks: (id) => {
                    // Force every import inlined
                    return undefined;
                },

                // IIFE is not supported using multiple entry points, mimimi
                // let's make it happen regardless...

                // TODO: does this break sourcemaps?
                // Aw whatever
                banner: "(function() {",
                footer: "})();",
            },
        }
    }
})
