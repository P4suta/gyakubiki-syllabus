<script lang="ts">
import IconPushPin from '~icons/ic/round-push-pin'
import { getColor } from '../lib/colors'
import { highlights, segment } from '../lib/highlight.svelte'
import { plan } from '../lib/plan.svelte'
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

// Search-match runs in the course name (the only highlighted field). Plain text
// when the active query doesn't hit this course.
const nameSegs = $derived(segment(course.nm, highlights.get(course.cd)))

// Registered in the user's plan → a small corner pin, in the tile's own accent.
const registered = $derived(plan.has(course.cd))

// Dominant assessment axis: label, share of the whole grade, and its palette
// colour (the same hue as the modal's donut, so the card's bar ties to it).
// Summed by *type* so a「期末試験50 + 中間試験50」course reads as 試験100%, not a
// single 50% — the card must not hide that it's exam-driven.
const topEval = $derived.by(() => {
	if (!course.ev?.length) return null
	const sum = course.ev.reduce((acc, e) => acc + (Number(e.split(':')[1]) || 0), 0)
	const byType = new Map<string, number>()
	for (const e of course.ev) {
		const [type, w] = e.split(':')
		byType.set(type, (byType.get(type) ?? 0) + (Number(w) || 0))
	}
	const top = [...byType.entries()].sort((a, b) => b[1] - a[1])[0]
	if (!top) return null
	const style = evalKind(top[0])
	return {
		label: style.label,
		pct: sum > 0 ? Math.round((top[1] / sum) * 100) : null,
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
	class="relative w-full text-left rounded-lg p-3 sm:p-1.5 mb-1 sm:mb-0.5 cursor-pointer transition-transform active:brightness-95 sm:hover:scale-[1.02] sm:hover:shadow-md border-l-3 min-h-tap sm:min-h-0"
	style="background: {color.bg}; border-left-color: {color.border};"
	{onclick}
>
	{#if registered}
		<!-- Registered marker: a pin in the tile's accent (inline colour). -->
		<IconPushPin class="absolute top-1 right-1 w-3 h-3" style="color: {color.accentText};" aria-label="登録済み" />
	{/if}
	<div class="font-semibold text-caption sm:text-micro leading-snug line-clamp-2" style="color: {color.text};">
		<!-- Match runs get a soft wash of the tile's own accent hue (inline style —
		     dynamic palette colour), so the highlight belongs to the macaron tile
		     rather than the browser's default yellow. -->
		{#each nameSegs as seg}{#if seg.mark}<mark style="background: color-mix(in oklab, {color.accentText} 26%, transparent); color: {color.text};">{seg.text}</mark>{:else}{seg.text}{/if}{/each}
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
				<!-- The chip sits on bg-overlay-medium (tile bg + slate), where mutedText
				     drops below AA; use the tile's max-contrast ink instead. -->
				<span class="inline-flex items-center shrink-0 rounded-full bg-overlay-medium px-1.5 py-0.5 leading-none" style="color: {color.text};">{mode.label}</span>
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
					<span class="flex items-center gap-0.5" aria-hidden="true">
						{#each creditBlocks as _}
							<span class="w-2 h-2" style="background: {color.border};"></span>
						{/each}
						{#if creditHalf}
							<span class="w-1 h-2" style="background: {color.border};"></span>
						{/if}
					</span>
					<span class="text-fine tabular-nums">{creditsN}単位</span>
				</span>
			{/if}
		</div>
	{/if}
</button>
