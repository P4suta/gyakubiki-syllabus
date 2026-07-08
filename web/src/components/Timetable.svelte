<script lang="ts">
import { type GridKey, PERIODS } from '../lib/engine'
import { haptic, type SwipeDir, swipeNavigate } from '../lib/gestures'
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

// Finger-follow day pager (mobile): `dragX` translates the current day with the
// finger; on commit the day slides out and the neighbour slides in (single day
// rendered — the swap happens while the incoming day is parked off-screen).
let dayEl = $state<HTMLElement>()
let dragX = $state(0)
let sliding = $state(false)

const canPrev = () => activeDay > 0
const canNext = () => activeDay < days.length - 1

function onDrag(dx: number) {
	dragX = dx
}

function commit(dir: 1 | -1) {
	const w = dayEl?.clientWidth || window.innerWidth
	haptic('select')
	sliding = true
	dragX = -dir * w // slide the current day out
	window.setTimeout(() => {
		activeDay += dir
		sliding = false
		dragX = dir * w // park the new day just off the incoming edge (no transition)
		requestAnimationFrame(() =>
			requestAnimationFrame(() => {
				sliding = true
				dragX = 0 // slide it into place
				window.setTimeout(() => {
					sliding = false
				}, 240)
			}),
		)
	}, 200)
}

function onSettle(dir: SwipeDir) {
	if (dir === 0) {
		sliding = true
		dragX = 0
		window.setTimeout(() => {
			sliding = false
		}, 240)
		return
	}
	commit(dir)
}

// Day count is data-driven, so the grid tracks `days`.
const gridCols = $derived(`64px repeat(${days.length}, 1fr)`)
const minWidth = $derived(`${days.length * 120 + 64}px`)

// Only the active layout is mounted — the desktop grid alone is 30 cells ×
// dozens of cards, and mounting both (hiding one with CSS) doubled the initial
// component work for nothing. `sm` = 640px, matching the Tailwind breakpoints
// below. Seeded synchronously so the correct view renders on the first frame.
const DESKTOP = '(min-width: 640px)'
let isDesktop = $state(typeof window !== 'undefined' && window.matchMedia(DESKTOP).matches)
$effect(() => {
	const mq = window.matchMedia(DESKTOP)
	const sync = () => {
		isDesktop = mq.matches
	}
	sync()
	mq.addEventListener('change', sync)
	return () => mq.removeEventListener('change', sync)
})
</script>

{#if !isDesktop}

<!-- Mobile: single-day view -->
<div
	class="flex flex-col flex-1 overflow-hidden sm:hidden"
	role="tabpanel"
	tabindex="-1"
>
	<div class="flex bg-surface-page border-b border-overlay-subtle shrink-0">
		{#each days as day, i}
			<button
				class="flex-1 py-2.5 text-center text-caption font-semibold min-h-[44px] transition-colors
					{activeDay === i
						? 'text-apple-blue border-b-2 border-apple-blue'
						: 'text-apple-text-tertiary active:text-apple-text-secondary'}"
				onclick={() => { activeDay = i }}
			>
				{day}
			</button>
		{/each}
	</div>

	<div
		bind:this={dayEl}
		class="flex-1 overflow-y-auto p-2 space-y-1.5 bg-surface-page touch-pan-y"
		style="translate: {dragX}px 0; transition: {sliding ? 'translate 0.22s var(--ease-spring)' : 'none'};"
		use:swipeNavigate={{ onDrag, onSettle, canPrev, canNext }}
	>
		{#each PERIODS as period}
			{@const courses = grid.get(`${days[activeDay]}-${period}`) ?? []}
			<div class="bg-surface-primary rounded-xl p-3">
				<div class="flex items-baseline gap-2 mb-1.5">
					<span class="text-micro font-medium text-apple-text-tertiary">{period}限</span>
					{#if PERIOD_TIMES[period]}
						<span class="text-fine text-apple-text-tertiary tabular-nums">
							{PERIOD_TIMES[period].start}–{PERIOD_TIMES[period].end}{#if PERIOD_TIMES[period].note} ・{PERIOD_TIMES[period].note}{/if}
						</span>
					{/if}
				</div>
				{#if courses.length === 0}
					<div class="text-micro text-apple-text-tertiary py-2">空きコマ</div>
				{:else}
					{#each courses as course (course.cd)}
						<CourseCard {course} onclick={() => onselect(course)} />
					{/each}
				{/if}
			</div>
		{/each}
	</div>
</div>
{/if}

<!-- Desktop: full grid -->
{#if isDesktop}
<div class="overflow-auto flex-1 bg-surface-page hidden sm:block">
	<div class="grid gap-[2px] bg-surface-page" style="grid-template-columns: {gridCols}; min-width: {minWidth};">
		<div class="sticky top-0 left-0 z-30 bg-surface-page"></div>

		{#each days as day}
			<div
				class="sticky top-0 z-20 text-center py-3 font-semibold text-caption text-apple-text-secondary tracking-tight bg-surface-page/80 backdrop-blur-sm"
			>
				{day}
			</div>
		{/each}

		{#each PERIODS as period}
			<div class="sticky left-0 z-10 bg-surface-page flex flex-col items-center px-1 py-2">
				<!-- start ── N限 ── end: times pinned to the row edges and joined by a
				     line, with the period label centered in the day-header style. -->
				{#if PERIOD_TIMES[period]}
					<div class="text-fine text-apple-text-tertiary tabular-nums leading-tight">{PERIOD_TIMES[period].start}</div>
					<div class="w-px flex-1 bg-overlay-strong my-1"></div>
				{/if}
				<div class="font-semibold text-caption text-apple-text-secondary tracking-tight">{period}限</div>
				{#if PERIOD_TIMES[period]?.note}
					<div class="text-fine text-apple-text-tertiary leading-tight">{PERIOD_TIMES[period].note}</div>
				{/if}
				{#if PERIOD_TIMES[period]}
					<div class="w-px flex-1 bg-overlay-strong my-1"></div>
					<div class="text-fine text-apple-text-tertiary tabular-nums leading-tight">{PERIOD_TIMES[period].end}</div>
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
{/if}
