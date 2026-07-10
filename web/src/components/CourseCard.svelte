<script lang="ts">
import { getColor } from '../lib/colors'
import { deliveryMode, evalKind } from '../lib/syllabus-icons'
import { useTheme } from '../lib/theme.svelte'
import type { Course } from '../types/course'

interface Props {
	course: Course
	onclick: () => void
}

let { course, onclick }: Props = $props()
const theme = useTheme()
let color = $derived.by(() => {
	const c = getColor(course.cd)
	return theme.isDark ? c.dark : c.light
})

// Representative instructor; "… ほか" once there is more than one.
const prof = $derived.by(() => {
	const names = course.prof
		.split(',')
		.map((s) => s.trim())
		.filter(Boolean)
	if (names.length === 0) return ''
	return names.length > 1 ? `${names[0]} ほか` : names[0]
})

const mode = $derived(deliveryMode(course.dm))

// Dominant assessment axis: label, share of the whole grade, and its palette
// colour (the same hue as the modal's donut, so the card's bar ties to it).
const topEval = $derived.by(() => {
	if (!course.ev?.length) return null
	const parsed = course.ev.map((e) => {
		const [type, w] = e.split(':')
		return { type, w: Number(w) || 0 }
	})
	const sum = parsed.reduce((acc, r) => acc + r.w, 0)
	const top = [...parsed].sort((a, b) => b.w - a.w)[0]
	if (!top) return null
	const style = evalKind(top.type)
	return {
		label: style.label,
		pct: sum > 0 ? Math.round((top.w / sum) * 100) : null,
		color: theme.isDark ? style.color.dark : style.color.light,
	}
})

// Credits: blocks are the hero (one per credit, a half block for .5 — the count
// reads at a glance), with a small「X単位」caption for the exact figure + label.
const creditsN = $derived(Number(course.unit) || 0)
const creditBlocks = $derived(Array.from({ length: Math.min(Math.floor(creditsN), 8) }))
const creditHalf = $derived(creditsN - Math.floor(creditsN) >= 0.5)
</script>

<button
	class="w-full text-left rounded-lg p-3 sm:p-1.5 mb-1 sm:mb-0.5 cursor-pointer transition-transform active:brightness-95 sm:hover:scale-[1.02] sm:hover:shadow-md border-l-3 min-h-tap sm:min-h-0"
	style="background: {color.bg}; border-left-color: {color.border};"
	{onclick}
>
	<div class="font-semibold text-caption sm:text-micro leading-snug line-clamp-2" style="color: {color.text};">
		{course.nm}
	</div>
	{#if prof}
		<div class="text-micro sm:text-fine truncate" style="color: {color.mutedText};">{prof}</div>
	{/if}
	{#if mode || topEval || creditsN > 0}
		<div class="flex items-center gap-1.5 text-micro sm:text-fine mt-1" style="color: {color.mutedText};">
			<!-- Delivery as its own filled chip so it reads as a separate tag, not part
			     of the assessment beside it. `inline-flex items-center` + padding keeps
			     the (CJK) label optically centred, where a bare bordered span doesn't. -->
			{#if mode}
				<span class="inline-flex items-center shrink-0 rounded-full bg-overlay-medium px-1.5 py-0.5 leading-none" style="color: {color.mutedText}; text-box: trim-both cap alphabetic;">{mode.label}</span>
			{/if}
			{#if topEval}
				<!-- Assessment group: label then donut, so the ring clearly belongs to
				     試験 (not to 対面 on its left). % printed teeny in the hole; only the
				     arc is rotated so the text stays upright; round caps keep it smooth. -->
				<span class="shrink-0 font-semibold">{topEval.label}</span>
				{#if topEval.pct != null}
					<svg class="w-4 h-4 shrink-0" viewBox="0 0 36 36" role="img" aria-label="{topEval.label} {topEval.pct}%">
						<title>{topEval.label} {topEval.pct}%</title>
						<circle cx="18" cy="18" r="15" fill="none" class="stroke-overlay-medium" stroke-width="5" />
						<circle cx="18" cy="18" r="15" fill="none" stroke={topEval.color} stroke-width="5" stroke-linecap="round" pathLength="100" stroke-dasharray="{topEval.pct} 100" transform="rotate(-90 18 18)" />
						<text x="18" y="18" text-anchor="middle" dominant-baseline="central" font-size="11" font-weight="700" fill="currentColor">{topEval.pct}%</text>
					</svg>
				{/if}
			{/if}
			{#if creditsN > 0}
				<span class="ml-auto flex items-center gap-1 shrink-0" title="{course.unit}単位" aria-label="{course.unit}単位">
					<span class="flex items-center gap-0.5">
						{#each creditBlocks as _}
							<span class="w-2 h-2" style="background: {color.border};"></span>
						{/each}
						{#if creditHalf}
							<span class="w-1 h-2" style="background: {color.border};"></span>
						{/if}
					</span>
					<span class="text-fine tabular-nums opacity-80">{creditsN}単位</span>
				</span>
			{/if}
		</div>
	{/if}
</button>
