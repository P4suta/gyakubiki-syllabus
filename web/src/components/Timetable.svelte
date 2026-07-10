<script lang="ts">
import IconWarning from '~icons/ic/round-warning'
import { useDesktop } from '../lib/breakpoint.svelte'
import { type GridKey, PERIODS } from '../lib/engine'
import { haptic, type SwipeDir, swipeNavigate } from '../lib/gestures'
import { PERIOD_TIMES } from '../lib/schedule'
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'
import TimetableCell from './TimetableCell.svelte'

interface Props {
	grid: Map<GridKey, Course[]>
	/** Registered courses per cell. A cell here is "locked" — it shows these in
	 *  place of the search candidates (the grid doubles as the plan). */
	planGrid?: Map<GridKey, Course[]>
	days: readonly string[]
	onselect: (course: Course) => void
	/** Grid keys whose locked cell holds two+ registered courses (a clash). */
	conflictKeys?: Set<GridKey>
}

let { grid, planGrid, days, onselect, conflictKeys }: Props = $props()

/** What a cell shows: the registered course(s) when the slot is locked, else the
 *  search candidates. `locked` drives the "confirmed" styling. */
function cell(key: GridKey): { courses: Course[]; locked: boolean } {
	const registered = planGrid?.get(key) ?? []
	if (registered.length > 0) return { courses: registered, locked: true }
	return { courses: grid.get(key) ?? [], locked: false }
}

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
const desktop = useDesktop()
const isDesktop = $derived(desktop.current)

// The「N限」badge stays readable while scrolling a tall row via native
// `position: sticky` (top: header height, bottom: 6px). Compositor-driven, so it
// tracks the scroll with zero jank; it rests at the row centre for short cells,
// glides within the band for tall ones, and — being confined to its own cell —
// never pulls an off-screen period into the rail. `headerH` feeds the top inset.
let headerH = $state(0)
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
				class="flex-1 py-2.5 text-center text-caption font-semibold min-h-tap transition-colors
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
			{@const key = `${days[activeDay]}-${period}` as GridKey}
			{@const { courses, locked } = cell(key)}
			<div class="bg-surface-primary rounded-xl p-3 {conflictKeys?.has(key) ? 'ring-2 ring-apple-red' : locked ? 'ring-1 ring-apple-blue' : ''}">
				<div class="sticky top-0 z-sticky -mx-3 -mt-3 px-3 pt-3 pb-1.5 mb-1.5 flex items-baseline gap-2 bg-surface-primary rounded-t-xl">
					<span class="text-micro font-medium text-apple-text-tertiary">{period}限</span>
					{#if PERIOD_TIMES[period]}
						<span class="text-fine text-apple-text-tertiary tabular-nums">
							{PERIOD_TIMES[period].start}–{PERIOD_TIMES[period].end}{#if PERIOD_TIMES[period].note} ・{PERIOD_TIMES[period].note}{/if}
						</span>
					{/if}
					{#if conflictKeys?.has(key)}
						<span class="ml-auto self-center inline-flex items-center gap-0.5 rounded-full bg-apple-red text-on-accent px-1.5 py-0.5 text-fine font-medium">
							<IconWarning class="w-2.5 h-2.5" aria-hidden="true" />重複
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
	<div class="grid gap-0.5 bg-surface-page" style="grid-template-columns: {gridCols}; min-width: {minWidth};">
		<div bind:clientHeight={headerH} class="sticky top-0 left-0 z-sticky-corner bg-surface-page"></div>

		{#each days as day}
			<div
				class="sticky top-0 z-sticky-head text-center py-3 font-semibold text-caption text-apple-text-secondary tracking-tight bg-surface-page/80 backdrop-blur-sm"
			>
				{day}
			</div>
		{/each}

		{#each PERIODS as period}
			<div data-period-label class="sticky left-0 z-sticky bg-surface-page relative flex flex-col items-center px-1 py-2">
				<!-- Times pinned to the row edges (they scroll away); the「N限」badge
				     rides a continuous rail and is JS-clamped to stay on screen. -->
				{#if PERIOD_TIMES[period]}
					<div class="text-fine text-apple-text-tertiary tabular-nums leading-tight">{PERIOD_TIMES[period].start}</div>
				{/if}
				<div class="relative flex-1 w-full my-1 flex items-center justify-center">
					{#if PERIOD_TIMES[period]}
						<div class="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px bg-overlay-strong"></div>
					{/if}
					<div data-period-badge class="sticky bg-surface-page px-0.5 text-center" style="top: {headerH}px; bottom: 6px;">
						<div class="font-semibold text-caption text-apple-text-secondary tracking-tight leading-tight">{period}限</div>
						{#if PERIOD_TIMES[period]?.note}
							<div class="text-fine text-apple-text-tertiary leading-tight">{PERIOD_TIMES[period].note}</div>
						{/if}
					</div>
				</div>
				{#if PERIOD_TIMES[period]}
					<div class="text-fine text-apple-text-tertiary tabular-nums leading-tight">{PERIOD_TIMES[period].end}</div>
				{/if}
			</div>
			{#each days as day}
				{@const key = `${day}-${period}` as GridKey}
				{@const c = cell(key)}
				<TimetableCell
					courses={c.courses}
					locked={c.locked}
					conflict={conflictKeys?.has(key) ?? false}
					{onselect}
				/>
			{/each}
		{/each}
	</div>
</div>
{/if}
