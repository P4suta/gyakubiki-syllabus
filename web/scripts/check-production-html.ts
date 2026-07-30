import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'

const html = readFileSync(new URL('../dist/index.html', import.meta.url), 'utf8')
const meta = html.match(/<meta http-equiv="Content-Security-Policy" content="([^"]+)">/u)
if (!meta?.[1]) throw new Error('Production index.html has no CSP meta element')
const policy = meta[1]
for (const directive of [
	"default-src 'self'",
	"object-src 'none'",
	"script-src 'self' 'wasm-unsafe-eval'",
	"connect-src 'self'",
	"worker-src 'self'",
]) {
	if (!policy.includes(directive)) throw new Error(`CSP directive is missing: ${directive}`)
}
if (/script-src[^;]*(?:https:|http:|\*|'unsafe-eval')/u.test(policy)) {
	throw new Error('CSP permits a non-self script source or unrestricted eval')
}
for (const match of html.matchAll(/<script\b(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/giu)) {
	const hash = `'sha256-${createHash('sha256')
		.update(match[1] ?? '')
		.digest('base64')}'`
	if (!policy.includes(hash)) throw new Error(`CSP is missing inline script hash ${hash}`)
}
console.log(`production CSP: ${policy}`)
