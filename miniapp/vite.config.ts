import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ command }) => ({
  plugins: [react()],
  base: command === 'serve' ? '/' : '/static/miniapp/',
  build: {
    outDir: '../static/miniapp',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/api': 'http://localhost:8889',
      '/write': 'http://localhost:8889',
      '/static': 'http://localhost:8889',
    },
  },
}))
