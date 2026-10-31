import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// Set by the Tauri CLI while it runs `beforeDevCommand` / `beforeBuildCommand`.
const targetPlatform = process.env.TAURI_ENV_PLATFORM
const debugBuild = process.env.TAURI_ENV_DEBUG === 'true'

export default defineConfig({
  plugins: [react()],
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
