<script lang="ts">
import type { EvalRow } from '../types/course'
import { evalKind } from '../lib/syllabus-icons'

interface Props {
	rows: EvalRow[]
	note?: string
}

let { rows, note }: Props = $props()

// Segments with resolved weight/percentage/style. When KULAS gives no weights,
// fall back to equal shares so the ratio chart still communicates the mix.
const segments = $derived.by(() => {
	const withWeights = rows.map((r) => ({ ...r, w: r.weight ?? 0 }))
	const sum = withWeights.reduce((a, b) => a + b.w, 0)
	const total = sum > 0 ? sum : withWeights.length
	return withWeights.map((r) => {
		const value = sum > 0 ? r.w : 1
		const pct = Math.round((value / total) * 100)
		return { ...r, pct, hasWeight: sum > 0, style: evalKind(r.type) }
	})
})

const hasWeights = $derived(segments.some((s) => s.hasWeight))

const R = 42
const C = 2 * Math.PI * R
const arcs = $derived.by(() => {
	let offset = 0
	return segments.map((s) => {
		const len = (s.pct / 100) * C
		const arc = { color: s.style.color, dash: `${len} ${C - len}`, offset: -offset }
		offset += len
		return arc
	})
})

const dominant = $derived(
	[...segments].sort((a, b) => b.pct - a.pct)[0] ?? null,
)
</script>

<div class="flex items-center gap-4 sm:gap-5">
	<div class="relative shrink-0 w-24 h-24 sm:w-28 sm:h-28">
		<svg viewBox="0 0 100 100" class="w-full h-full -rotate-90">
			<circle cx="50" cy="50" r={R} fill="none" class="stroke-overlay-light" stroke-width="14" />
			{#each arcs as arc}
				<circle
					cx="50"
					cy="50"
					r={R}
					fill="none"
					stroke={arc.color}
					stroke-width="14"
					stroke-dasharray={arc.dash}
					stroke-dashoffset={arc.offset}
				/>
			{/each}
		</svg>
		{#if dominant}
			<div class="absolute inset-0 flex flex-col items-center justify-center">
				<span class="text-xl leading-none">{dominant.style.emoji}</span>
				{#if hasWeights}
					<span class="text-caption font-semibold text-apple-text mt-0.5">{dominant.pct}%</span>
				{/if}
			</div>
		{/if}
	</div>

	<div class="min-w-0 flex-1 space-y-1.5">
		{#each segments as s}
			<div class="flex items-center gap-2 text-caption">
				<span class="w-2 h-2 rounded-full shrink-0" style="background: {s.style.color};"></span>
				<span class="text-apple-text/90 truncate">{s.item || s.style.label}</span>
				{#if s.hasWeight}
					<span class="ml-auto shrink-0 text-apple-text/60 tabular-nums font-medium">{s.pct}%</span>
				{/if}
			</div>
		{/each}
	</div>
</div>

{#if note}
	<!-- Grading caveats (e.g.「小テストは毎回実施」) are decision-critical — give them
	     the accent colour and a tinted callout instead of a faint gray line. -->
	<p class="mt-3 flex gap-1.5 rounded-lg bg-apple-blue/10 px-3 py-2 text-micro text-apple-blue leading-relaxed whitespace-pre-line">
		<svg class="w-3.5 h-3.5 shrink-0 mt-px" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m0 3.75h.008M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
		</svg>
		<span>{note}</span>
	</p>
{/if}
