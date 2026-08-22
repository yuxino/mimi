import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const buildPublicBaseUrl = process.env.BUILD_PUBLIC_BASE_URL?.trim()
const base = buildPublicBaseUrl
  ? `${buildPublicBaseUrl.replace(/\/$/, '')}/`
  : './'

// https://vite.dev/config/
export default defineConfig({
  base,
  plugins: [
    react(),
  ],
})
