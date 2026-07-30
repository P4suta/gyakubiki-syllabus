import { readdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const workflowsDir = fileURLToPath(new URL('../../.github/workflows/', import.meta.url))
const read = (name: string) =>
	readFileSync(new URL(`../../.github/workflows/${name}`, import.meta.url), 'utf8')
const requireText = (source: string, text: string, label: string) => {
	if (!source.includes(text)) throw new Error(`${label} is missing: ${text}`)
}

const ci = read('ci.yml')
for (const text of [
	'always() &&',
	"needs.required.result == 'success'",
	"needs.production-artifact.result == 'success'",
])
	requireText(ci, text, 'deploy guard')

const syllabus = read('fetch-syllabus.yml')
const details = read('fetch-details.yml')
const release = read('release-please.yml')
requireText(syllabus, 'vars.DATA_AUTOMATION_ENABLED', 'syllabus automation switch')
requireText(details, 'vars.DATA_AUTOMATION_ENABLED', 'details automation switch')
requireText(release, 'vars.RELEASE_AUTOMATION_ENABLED', 'release automation switch')
for (const step of ['steps.app-token.outcome', 'steps.checkout.outcome', 'steps.build.outcome']) {
	requireText(details, step, 'detail post-processing guard')
}
requireText(read('production-smoke.yml'), 'PRODUCTION_URL', 'production smoke URL')

const pinPattern = /uses:\s+[^@\s]+@[a-f0-9]{40}\s+#\s+v\S+/u
for (const name of readdirSync(workflowsDir).filter((entry) => /\.ya?ml$/u.test(entry))) {
	const source = read(name)
	for (const [index, line] of source.split('\n').entries()) {
		if (!/\buses:/u.test(line)) continue
		if (!pinPattern.test(line))
			throw new Error(`${name}:${index + 1} action is not SHA-pinned with a version comment`)
	}
}
for (const obsolete of [
	'jdx/mise-action@',
	'actions/upload-artifact@ea165f8d',
	'actions/download-artifact@d3f86a10',
	'github/codeql-action/init@3b0bd1d1',
]) {
	for (const name of readdirSync(workflowsDir).filter((entry) => /\.ya?ml$/u.test(entry))) {
		if (read(name).includes(obsolete))
			throw new Error(`${name} still uses Node 20 action ${obsolete}`)
	}
}
console.log('workflow policy: deploy, automation, smoke, and action pins verified')
