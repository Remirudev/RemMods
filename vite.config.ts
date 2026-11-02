import react from '@vitejs/plugin-react'
import { defineConfig, type Plugin } from 'vite'

// Set by the Tauri CLI while it runs `beforeDevCommand` / `beforeBuildCommand`.
const targetPlatform = process.env.TAURI_ENV_PLATFORM
const debugBuild = process.env.TAURI_ENV_DEBUG === 'true'

// `vite --mode e2e` only: puts the Playwright stand-in backend (e2e/mock) in front of the app.
// Dev server only, so a build never contains it; Vitest runs in mode `test`, where it does nothing.
function e2eMockBackend(): Plugin {
  let active = false
  return {
    name: 'remmods-e2e-mock-backend',
    apply: 'serve',
    // The dependency scan starts from the mock too: a dependency found only later makes Vite
    // reload the page, which would reset the mock state in the middle of a test.
    config(_config, env) {
      return env.mode === 'e2e'
        ? { optimizeDeps: { entries: ['index.html', 'e2e/mock/install.ts'] } }
        : undefined
    },
    configResolved(config) {
      active = config.mode === 'e2e'
    },
    transformIndexHtml() {
      return active
        ? [
            {
              tag: 'script',
              attrs: { type: 'module', src: '/e2e/mock/install.ts' },
              injectTo: 'head-prepend',
            },
          ]
        : []
    },
  }
}

export default defineConfig({
  plugins: [react(), e2eMockBackend()],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  server: {
    port: 1430,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**', '**/target/**'],
    },
  },
  build: {
    // WebView2 on Windows is Chromium; Linux runs WebKitGTK.
    target: targetPlatform === 'windows' ? 'chrome120' : 'safari16',
    minify: !debugBuild,
    sourcemap: debugBuild,
    // Bundles load from disk inside the app, so the web's 500 kB download warning does not apply.
    chunkSizeWarningLimit: 2048,
  },
})
