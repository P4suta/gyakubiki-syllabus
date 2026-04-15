import { readFileSync } from 'node:fs'
import tailwindcss from '@tailwindcss/vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

interface DataJson {
	courses: { slots: { d: number }[] }[]
}

function readBuildFlags() {
	try {
		const raw = readFileSync('./public/data.json', 'utf-8')
		const data: DataJson = JSON.parse(raw)
		const hasSaturday = data.courses.some((c) =>
			c.slots.some((s) => s.d === 5),
		)
		return { __HAS_SATURDAY__: hasSaturday }
	} catch {
		// data.json not yet generated (e.g. fresh clone before CLI run)
		return { __HAS_SATURDAY__: false }
	}
}

const flags = readBuildFlags()

export default defineConfig({
	base: process.env.GITHUB_PAGES === 'true' ? '/gyakubiki-syllabus/' : '/',
	plugins: [svelte(), tailwindcss()],
	define: {
		__HAS_SATURDAY__: JSON.stringify(flags.__HAS_SATURDAY__),
	},
})
