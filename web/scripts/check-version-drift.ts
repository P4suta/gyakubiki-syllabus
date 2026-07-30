import { readFileSync } from 'node:fs'

const cargoManifest = readFileSync(new URL('../../Cargo.toml', import.meta.url), 'utf8')
const packageManifest = JSON.parse(
	readFileSync(new URL('../package.json', import.meta.url), 'utf8'),
) as { version?: unknown }
const cargoVersion = cargoManifest.match(
	/\[workspace\.package\][\s\S]*?\bversion\s*=\s*"([^"]+)"/u,
)?.[1]
const webVersion = packageManifest.version

if (!cargoVersion) throw new Error('Cargo workspace version was not found')
if (typeof webVersion !== 'string') throw new Error('web/package.json version is missing')
if (cargoVersion !== webVersion) {
	throw new Error(`Version drift: Cargo.toml=${cargoVersion}, web/package.json=${webVersion}`)
}
console.log(`version mirror: ${webVersion}`)
