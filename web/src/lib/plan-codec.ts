// Encode/decode a plan (a list of course codes) as a compact, shareable token.
//
// Format: `<version>.<cd-cd-cd>`. Each cd is percent-encoded (and its `-`
// escaped) so the `-` separator is unambiguous even for an opaque code. Decoding
// is deliberately tolerant — a shared link may outlive a schema bump or a data
// refresh — so junk, an unknown version, or a code no longer in the dataset just
// drops out rather than throwing. Future fields append after a `;`, which a v1
// decoder ignores (additive forward-compat).

export const PLAN_SCHEMA_VERSION = 1

/** Encode course codes to a token. Empty list → empty string (no plan). */
export function encodePlan(cds: readonly string[]): string {
	if (cds.length === 0) return ''
	const payload = cds.map((cd) => encodeURIComponent(cd).replace(/-/g, '%2D')).join('-')
	return `${PLAN_SCHEMA_VERSION}.${payload}`
}

/** Decode a token to course codes: tolerant, de-duplicated, order-preserving. */
export function decodePlan(token: string): string[] {
	if (!token) return []
	const dot = token.indexOf('.')
	if (dot < 0) return []
	const version = Number(token.slice(0, dot))
	if (!Number.isFinite(version) || version < 1) return []

	// Read the code list; anything after an optional `;` is a future extension.
	const payload = token.slice(dot + 1).split(';')[0]
	if (!payload) return []

	const seen = new Set<string>()
	const out: string[] = []
	for (const part of payload.split('-')) {
		if (!part) continue
		let cd: string
		try {
			cd = decodeURIComponent(part)
		} catch {
			continue // malformed escape — skip this entry, keep the rest
		}
		if (cd && !seen.has(cd)) {
			seen.add(cd)
			out.push(cd)
		}
	}
	return out
}
