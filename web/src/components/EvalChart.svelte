<script lang="ts">
import IconInfo from '~icons/ic/round-info'
import { evalArcs, evalSegments } from '../lib/eval-chart'
import { evalKind } from '../lib/syllabus-icons'
import { useTheme } from '../lib/theme.svelte'
import type { EvalRow } from '../types/course'

interface Props {
	rows: EvalRow[]
	note?: string
}

let { rows, note }: Props = $props()

const theme = useTheme()

// Percentages (equal-split when weightless) plus each row's icon/colour style,
// ordered largest share first — so the donut starts the biggest slice at 12
// o'clock (svg is -rotate-90) sweeping clockwise, and the bar/legend match.
const segments = $derived(
	evalSegments(rows)
		.map((s) => {
			const style = evalKind(s.type)
			return { ...s, style, color: theme.isDark ? style.color.dark : style.color.light }
		})
		.sort((a, b) => b.pct - a.pct),
)

const hasWeights = $derived(segments.some((s) => s.hasWeight))

const R = 42
const arcs = $derived(
	evalArcs(
		segments.map((s) => s.pct),
		R,
	).map((arc, i) => ({ ...arc, color: segments[i].color })),
)

const dominant = $derived(segments[0] ?? null)
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
				<Icon class="w-6 h-6" style="color: {dominant.color};" />
				{#if hasWeights}
					<span class="text-caption font-semibold text-apple-text mt-0.5 tabular-nums">{dominant.pct}%</span>
				{/if}
			</div>
		{/if}
	</div>

	<div class="min-w-0 flex-1">
		<!-- A single cumulative bar: each segment's width is its share, laid end to
		     end — the "横に連続した" reading the vertical list couldn't give. -->
		<div class="flex h-2.5 w-full overflow-hidden rounded-full bg-overlay-light" aria-hidden="true">
			{#each segments as s}
				{#if s.pct > 0}
					<span class="h-full" style="width: {s.pct}%; background: {s.color};"></span>
				{/if}
			{/each}
		</div>
		<ul class="mt-2.5 space-y-1.5">
			{#each segments as s}
				<li class="flex items-center gap-2 text-caption">
					<span class="w-2 h-2 rounded-full shrink-0" style="background: {s.color};"></span>
					<span class="text-apple-text truncate">{s.item || s.style.label}</span>
					<!-- A 0% here means the source listed this component without a weight
					     while others had one — assessed but its share is unstated, so no
					     misleading %. -->
					{#if s.hasWeight && s.pct > 0}
						<span class="ml-auto shrink-0 text-apple-text-secondary tabular-nums font-medium">{s.pct}%</span>
					{/if}
				</li>
			{/each}
		</ul>
	</div>
</div>

{#if note}
	<!-- Grading caveats (e.g.「小テストは毎回実施」) are decision-critical — give them
	     the accent colour and a tinted callout instead of a faint gray line. -->
	<p class="mt-3 flex gap-1.5 rounded-lg bg-apple-blue/10 px-3 py-2 text-micro text-apple-blue leading-relaxed whitespace-pre-line">
		<IconInfo class="w-3.5 h-3.5 shrink-0 mt-px" />
		<span>{note}</span>
	</p>
{/if}
