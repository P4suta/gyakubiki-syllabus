import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  base: process.env.GITHUB_PAGES === 'true'
    ? '/gyakubiki-syllabus/'
    : '/',
  plugins: [svelte(), tailwindcss()],
})
