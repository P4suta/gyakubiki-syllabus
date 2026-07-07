<script lang="ts">
import { getColor } from '../lib/colors'
import { deliveryMode, evalKind } from '../lib/syllabus-icons'
import type { Course } from '../types/course'

interface Props {
	course: Course
	onclick: () => void
}

let { course, onclick }: Props = $props()
let color = $derived(getColor(course.cd))

const mode = $derived(deliveryMode(course.dm))

// The course's dominant assessment axis with its share of the whole grade
// (e.g. 試験 60%). Percentage is the weight relative to the total, matching the
// modal's chart; omitted when KULAS gives no weights.
const topEval = $derived.by(() => {
	if (!course.ev?.length) return null
	const parsed = course.ev.map((e) => {
		const [type, w] = e.split(':')
		return { type, w: Number(w) || 0 }
	})
	const sum = parsed.reduce((acc, r) => acc + r.w, 0)
	const top = [...parsed].sort((a, b) => b.w - a.w)[0]
	if (!top) return null
	const pct = sum > 0 ? Math.round((top.w / sum) * 100) : null
	return `${evalKind(top.type).label}${pct != null ? ` ${pct}%` : ''}`
})

// Compact "delivery · assessment%" line; omits whichever part is missing.
const meta = $derived([mode?.label, topEval].filter(Boolean).join(' · '))
</script>

<button
	class="w-full text-left rounded-lg p-3 sm:p-1.5 mb-1 sm:mb-0.5 cursor-pointer transition-transform active:brightness-95 sm:hover:scale-[1.02] sm:hover:shadow-md border-l-3 min-h-[44px] sm:min-h-0"
	style="background: {color.bg}; border-left-color: {color.border};"
	{onclick}
>
	<div class="font-semibold text-caption sm:text-micro leading-snug line-clamp-2 text-apple-text">
		{course.nm}
	</div>
	{#if meta || course.unit}
		<div class="flex items-center gap-1 text-micro sm:text-fine text-apple-text/50 mt-0.5">
			<span class="truncate">{meta}</span>
			{#if course.unit}<span class="ml-auto shrink-0 tabular-nums">{course.unit}</span>{/if}
		</div>
	{/if}
</button>
