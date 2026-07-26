import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

const output = path.resolve(process.argv[2] ?? 'reports/web.cdx.json')
const modulesRoot = path.resolve('node_modules')
const allowedLicenses = new Set([
	'0BSD',
	'(MIT OR CC0-1.0)',
	'Apache-2.0',
	'BSD-2-Clause',
	'BSD-3-Clause',
	'BlueOak-1.0.0',
	'CC-BY-4.0',
	'CC0-1.0',
	'ISC',
	'MIT',
	'MIT OR Apache-2.0',
	'MIT-0',
	'MPL-2.0',
])
// package.json omits the field, but the installed package includes the full
// three-clause BSD text in node_modules/parse-cache-control/LICENSE.
const reviewedMissingLicenses = new Map([['parse-cache-control@1.0.1', 'BSD-3-Clause']])

interface PackageManifest {
	name?: unknown
	version?: unknown
	license?: unknown
}

interface Component {
	type: 'library'
	'bom-ref': string
	name: string
	version: string
	purl: string
	licenses: { expression: string }[]
	hashes: { alg: 'SHA-256'; content: string }[]
}

function licenseExpression(manifest: PackageManifest, key: string): string {
	const declared = typeof manifest.license === 'string' ? manifest.license.trim() : ''
	const expression = declared || reviewedMissingLicenses.get(key)
	if (!expression) throw new Error(`${key} has no reviewed license declaration`)
	if (!allowedLicenses.has(expression)) {
		throw new Error(`${key} uses disallowed/unreviewed license ${JSON.stringify(expression)}`)
	}
	return expression
}

function packageHash(manifestPath: string): string {
	return createHash('sha256').update(fs.readFileSync(manifestPath)).digest('hex')
}

function npmPurl(name: string, version: string): string {
	const [scope, packageName] = name.startsWith('@')
		? name.slice(1).split('/', 2)
		: [undefined, name]
	const path =
		scope && packageName
			? `%40${encodeURIComponent(scope)}/${encodeURIComponent(packageName)}`
			: encodeURIComponent(name)
	return `pkg:npm/${path}@${encodeURIComponent(version)}`
}

function scan(root: string): Component[] {
	const components = new Map<string, Component>()
	const visited = new Set<string>()

	function visit(directory: string) {
		let real: string
		try {
			real = fs.realpathSync(directory)
		} catch {
			return
		}
		if (visited.has(real)) return
		visited.add(real)

		const entries = fs
			.readdirSync(directory, { withFileTypes: true })
			.filter((entry) => entry.isDirectory() || entry.isSymbolicLink())
			.sort((left, right) => left.name.localeCompare(right.name))
		for (const entry of entries) {
			if (entry.name === '.bin') continue
			const child = path.join(directory, entry.name)
			const manifestPath = path.join(child, 'package.json')
			if (fs.existsSync(manifestPath)) {
				const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as PackageManifest
				if (typeof manifest.name === 'string' && typeof manifest.version === 'string') {
					const key = `${manifest.name}@${manifest.version}`
					const purl = npmPurl(manifest.name, manifest.version)
					components.set(key, {
						type: 'library',
						'bom-ref': purl,
						name: manifest.name,
						version: manifest.version,
						purl,
						licenses: [{ expression: licenseExpression(manifest, key) }],
						hashes: [{ alg: 'SHA-256', content: packageHash(manifestPath) }],
					})
				}
			}
			visit(child)
		}
	}

	visit(root)
	return [...components.values()].sort((left, right) =>
		`${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`),
	)
}

if (!fs.existsSync(modulesRoot)) {
	throw new Error('node_modules is missing; run bun install --frozen-lockfile first')
}
const rootManifest = JSON.parse(fs.readFileSync('package.json', 'utf8')) as {
	version: string
}
const components = scan(modulesRoot)
if (components.length === 0) throw new Error('Web SBOM unexpectedly has no components')

const bom = {
	bomFormat: 'CycloneDX',
	specVersion: '1.6',
	version: 1,
	metadata: {
		component: {
			type: 'application',
			'bom-ref': `pkg:github/P4suta/gyakubiki-syllabus@${rootManifest.version}`,
			name: 'gyakubiki-syllabus-web',
			version: rootManifest.version,
			licenses: [{ expression: 'AGPL-3.0-or-later' }],
		},
	},
	components,
}

fs.mkdirSync(path.dirname(output), { recursive: true })
fs.writeFileSync(output, `${JSON.stringify(bom, null, 2)}\n`)
console.log(
	JSON.stringify({
		output: path.relative(process.cwd(), output),
		components: components.length,
		licensePolicy: 'pass',
	}),
)
