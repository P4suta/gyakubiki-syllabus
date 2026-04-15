<script lang="ts">
import type { GridKey } from '../lib/grid'
import { DAYS, PERIODS } from '../lib/grid'
import type { CourseV2 } from '../types/course'
import CourseCard from './CourseCard.svelte'
import TimetableCell from './TimetableCell.svelte'

interface Props {
	grid: Map<GridKey, CourseV2[]>
	onselect: (course: CourseV2) => void
}

let { grid, onselect }: Props = $props()

// Mobile: single-day view state
let activeDay = $state(0)

// Swipe navigation
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
		if (dx < 0 && activeDay < DAYS.length - 1) activeDay++
		if (dx > 0 && activeDay > 0) activeDay--
	}
}

// Desktop grid
const gridCols = `56px repeat(${DAYS.length}, 1fr)`
const minWidth = `${DAYS.length * 120 + 56}px`
</script>

<!-- ==================== Mobile: Single-day view ==================== -->
<div
	class="flex flex-col flex-1 overflow-hidden sm:hidden"
	ontouchstart={handleTouchStart}
	ontouchend={handleTouchEnd}
	role="tabpanel"
	tabindex="-1"
>
	<!-- Day tab strip -->
	<div class="flex bg-surface-page border-b border-overlay-subtle shrink-0">
		{#each DAYS as day, i}
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

	<!-- Periods list for active day -->
	<div class="flex-1 overflow-y-auto p-2 space-y-1.5 bg-surface-page">
		{#each PERIODS as period}
			{@const courses = grid.get(`${DAYS[activeDay]}-${period}`) ?? []}
			<div class="bg-surface-primary rounded-xl p-3">
				<div class="text-micro font-medium text-apple-text/40 mb-1.5">{period}限</div>
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

<!-- ==================== Desktop: Full grid ==================== -->
<div class="overflow-auto flex-1 bg-surface-page hidden sm:block">
	<div class="grid gap-[2px] bg-surface-page" style="grid-template-columns: {gridCols}; min-width: {minWidth};">
		<!-- Corner cell -->
		<div class="sticky top-0 left-0 z-30 bg-surface-page"></div>

		<!-- Day headers -->
		{#each DAYS as day}
			<div
				class="sticky top-0 z-20 text-center py-3 font-semibold text-caption text-apple-text/60 tracking-tight bg-surface-page/80 backdrop-blur-sm"
			>
				{day}
			</div>
		{/each}

		<!-- Grid rows -->
		{#each PERIODS as period}
			<!-- Period label -->
			<div class="sticky left-0 z-10 bg-surface-page">
				<div class="sticky top-10 px-1 py-2 text-center font-medium text-caption text-apple-text/40">
					{period}限
				</div>
			</div>
			{#each DAYS as day}
				<TimetableCell
					courses={grid.get(`${day}-${period}`) ?? []}
					{onselect}
				/>
			{/each}
		{/each}
	</div>
</div>
