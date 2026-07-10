<script lang="ts">
import IconInfo from '~icons/ic/round-info'
import { evalArcs, evalSegments, sumByType } from '../lib/eval-chart'
import { evalKind } from '../lib/syllabus-icons'
import { useTheme } from '../lib/theme.svelte'
import type { EvalRow } from '../types/course'

interface Props {
	rows: EvalRow[]
	note?: string
}

let { rows, note }: Props = $props()

const theme = useTheme()

// Per-item shares with icon/colour. Colour is keyed by `type`, so same-type
// items share a hue and read as one band.
const segments = $derived(
	evalSegments(rows).map((s) => {
		const style = evalKind(s.type)
		return { ...s, style, color: theme.isDark ? style.color.dark : style.color.light }
	}),
)

const hasWeights = $derived(segments.some((s) => s.hasWeight))

// Aggregate by type so the "main" is the category (レポート60 > 出席40), and use
// that order to keep same-type rows contiguous — the report band reads as a block.
const typeRank = $derived(new Map(sumByType(segments).map((t, i) => [t.type, i])))
const ordered = $derived(
	[...segments].sort(
		(a, b) => (typeRank.get(a.type) ?? 0) - (typeRank.get(b.type) ?? 0) || b.pct - a.pct,
	),
)

// Cumulative offset per row → each row's bar segment sits at its place along the
// 0–100% track, so the stacked bar and the legend are one object.
const laid = $derived.by(() => {
	let acc = 0
	return ordered.map((s) => {
		const offset = acc
		acc += s.pct
		return { ...s, offset }
	})
})

// The dominant *type* (icon + summed %) sits in the donut hole.
const dominant = $derived.by(() => {
	const top = sumByType(segments)[0]
	if (!top) return null
	return { pct: top.pct, style: evalKind(top.type) }
})

const R = 42
const arcs = $derived(
	evalArcs(
		laid.map((s) => s.pct),
		R,
	).map((arc, i) => ({ ...arc, color: laid[i].color })),
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
			{@const Icon = dominant.style.icon}
			<div class="absolute inset-0 flex flex-col items-center justify-center">
				<Icon class="w-6 h-6" style="color: {theme.isDark ? dominant.style.color.dark : dominant.style.color.light};" />
				{#if hasWeights}
					<span class="text-caption font-semibold text-apple-text mt-0.5 tabular-nums">{dominant.pct}%</span>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Legend and bar are one object: each row shows its label + its slice sitting
	     at its cumulative offset, so the segments chain across rows to 100%. -->
	<ul class="min-w-0 flex-1 space-y-2">
		{#each laid as s}
			<li>
				<div class="flex items-center gap-2 text-caption">
					<span class="w-2 h-2 rounded-full shrink-0" style="background: {s.color};"></span>
					<span class="text-apple-text truncate">{s.item || s.style.label}</span>
					{#if s.hasWeight && s.pct > 0}
						<span class="ml-auto shrink-0 text-apple-text-secondary tabular-nums font-medium">{s.pct}%</span>
					{/if}
				</div>
				<div class="mt-1 h-1.5 w-full rounded-full bg-overlay-light overflow-hidden">
					<span class="block h-full rounded-full" style="margin-left: {s.offset}%; width: {s.pct}%; background: {s.color};"></span>
				</div>
			</li>
		{/each}
	</ul>
</div>

{#if note}
	<!-- Grading caveats (e.g.「小テストは毎回実施」) are decision-critical — give them
	     the accent colour and a tinted callout instead of a faint gray line. -->
	<p class="mt-3 flex gap-1.5 rounded-lg bg-apple-blue/10 px-3 py-2 text-micro text-apple-blue leading-relaxed whitespace-pre-line">
		<IconInfo class="w-3.5 h-3.5 shrink-0 mt-px" />
		<span>{note}</span>
	</p>
{/if}
