import { fileURLToPath, URL } from 'node:url'
import { resolve
 } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

import vueDevTools from 'vite-plugin-vue-devtools'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vueDevTools(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
build: {
    rollupOptions: {
      output: {
        assetFileNames: "aoer-wled-doppler.[ext]",
        entryFileNames: 'app-[name].js'
      },
    },
  },
})
