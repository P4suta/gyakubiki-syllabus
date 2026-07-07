<script lang="ts">
import { type GridKey, PERIODS } from '../lib/engine'
import { PERIOD_TIMES } from '../lib/schedule'
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'
import TimetableCell from './TimetableCell.svelte'

interface Props {
	grid: Map<GridKey, Course[]>
	days: readonly string[]
	onselect: (course: Course) => void
}

let { grid, days, onselect }: Props = $props()

let activeDay = $state(0)

let touchStartX = 0
let touchStartY = 0

function handleTouchStart(e: TouchEvent) {
	touchStartX = e.touches[0].clientX
	touchStartY = e.touches[0].clientY
}

function handleTouchEnd(e: TouchEvent) {
	const dx = e.changedTouches[0].clientX - touchStartX
	const dy = e.changedTouches[0].clientY - touchStartY
	if (Math.abs(dx) > 50 && Math.abs(dx) > Math.abs(dy) * 1.5) {
		if (dx < 0 && activeDay < days.length - 1) activeDay++
		if (dx > 0 && activeDay > 0) activeDay--
	}
}

// Day count is data-driven, so the grid tracks `days`.
const gridCols = $derived(`64px repeat(${days.length}, 1fr)`)
const minWidth = $derived(`${days.length * 120 + 64}px`)
</script>

<!-- Mobile: single-day view -->
<div
	class="flex flex-col flex-1 overflow-hidden sm:hidden"
	ontouchstart={handleTouchStart}
	ontouchend={handleTouchEnd}
	role="tabpanel"
	tabindex="-1"
>
	<div class="flex bg-surface-page border-b border-overlay-subtle shrink-0">
		{#each days as day, i}
			<button
				class="flex-1 py-2.5 text-center text-caption font-semibold min-h-[44px] transition-colors
					{activeDay === i
						? 'text-apple-blue border-b-2 border-apple-blue'
						: 'text-apple-text/40 active:text-apple-text/60'}"
				onclick={() => { activeDay = i }}
			>
				{day}
			</button>
		{/each}
	</div>

	<div class="flex-1 overflow-y-auto p-2 space-y-1.5 bg-surface-page">
		{#each PERIODS as period}
			{@const courses = grid.get(`${days[activeDay]}-${period}`) ?? []}
			<div class="bg-surface-primary rounded-xl p-3">
				<div class="flex items-baseline gap-2 mb-1.5">
					<span class="text-micro font-medium text-apple-text/40">{period}限</span>
					{#if PERIOD_TIMES[period]}
						<span class="text-fine text-apple-text/30 tabular-nums">
							{PERIOD_TIMES[period].start}–{PERIOD_TIMES[period].end}{#if PERIOD_TIMES[period].note} ・{PERIOD_TIMES[period].note}{/if}
						</span>
					{/if}
				</div>
				{#if courses.length === 0}
					<div class="text-micro text-apple-text/20 py-2">空きコマ</div>
				{:else}
					{#each courses as course (course.cd)}
						<CourseCard {course} onclick={() => onselect(course)} />
					{/each}
				{/if}
			</div>
		{/each}
	</div>
</div>

<!-- Desktop: full grid -->
<div class="overflow-auto flex-1 bg-surface-page hidden sm:block">
	<div class="grid gap-[2px] bg-surface-page" style="grid-template-columns: {gridCols}; min-width: {minWidth};">
		<div class="sticky top-0 left-0 z-30 bg-surface-page"></div>

		{#each days as day}
			<div
				class="sticky top-0 z-20 text-center py-3 font-semibold text-caption text-apple-text/60 tracking-tight bg-surface-page/80 backdrop-blur-sm"
			>
				{day}
			</div>
		{/each}

		{#each PERIODS as period}
			<div class="sticky left-0 z-10 bg-surface-page flex flex-col items-center px-1 py-2">
				<!-- start ── N限 ── end: times pinned to the row edges and joined by a
				     line, with the period label centered in the day-header style. -->
				{#if PERIOD_TIMES[period]}
					<div class="text-fine text-apple-text/30 tabular-nums leading-tight">{PERIOD_TIMES[period].start}</div>
					<div class="w-px flex-1 bg-overlay-strong my-1"></div>
				{/if}
				<div class="font-semibold text-caption text-apple-text/60 tracking-tight">{period}限</div>
				{#if PERIOD_TIMES[period]?.note}
					<div class="text-fine text-apple-text/30 leading-tight">{PERIOD_TIMES[period].note}</div>
				{/if}
				{#if PERIOD_TIMES[period]}
					<div class="w-px flex-1 bg-overlay-strong my-1"></div>
					<div class="text-fine text-apple-text/30 tabular-nums leading-tight">{PERIOD_TIMES[period].end}</div>
				{/if}
			</div>
			{#each days as day}
				<TimetableCell
					courses={grid.get(`${day}-${period}`) ?? []}
					{onselect}
				/>
			{/each}
		{/each}
	</div>
</div>
