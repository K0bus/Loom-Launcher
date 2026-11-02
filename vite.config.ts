import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/igdb-api': {
        target: 'https://api.igdb.com/v4',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/igdb-api/, '')
      },
      '/twitch-oauth': {
        target: 'https://id.twitch.tv/oauth2',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/twitch-oauth/, '')
      }
    }
  },
  clearScreen: false
})
