<script lang="ts">
import IconWarning from '~icons/ic/round-warning'
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: Course[]
	onselect: (course: Course) => void
	/** Slot is filled by a registered course — show it confirmed (blue ring). */
	locked?: boolean
	/** Ring this cell red when two registered courses collide here. */
	conflict?: boolean
}

let { courses, onselect, locked = false, conflict = false }: Props = $props()

// The desktop grid is 30+ cells with dozens of cards each; mounting every
// CourseCard up front is the bulk of the initial main-thread work. Defer a
// cell's cards until it nears the viewport, so first paint only builds what's
// on screen — the desktop analogue of the mobile single-day view. `rootMargin`
// pre-mounts a screenful ahead so scrolling reveals cards with no pop-in, and
// once shown a cell stays mounted (no churn as it scrolls back out).
let el = $state<HTMLDivElement>()
let visible = $state(false)

$effect(() => {
	if (!el || visible) return
	const io = new IntersectionObserver(
		(entries) => {
			if (entries.some((e) => e.isIntersecting)) {
				visible = true
				io.disconnect()
			}
		},
		{ rootMargin: '200px' },
	)
	io.observe(el)
	return () => io.disconnect()
})
</script>

<!-- content-visibility defers layout/paint of off-screen cells; the intrinsic
     size reserves height so both the skeleton and scrolling stay stable. -->
<div
	bind:this={el}
	class="relative bg-surface-primary rounded-lg min-h-24 p-1.5 content-auto {conflict ? 'ring-2 ring-apple-red' : locked ? 'ring-1 ring-apple-blue' : ''}"
>
	{#if conflict}
		<!-- Two registered courses collide here — a badge, not just a ring, so the
		     clash is unmissable. -->
		<span class="absolute -top-1.5 -right-1.5 z-sticky inline-flex items-center gap-0.5 rounded-full bg-apple-red text-on-accent px-1.5 py-0.5 text-fine font-medium shadow-card">
			<IconWarning class="w-2.5 h-2.5" aria-hidden="true" />重複
		</span>
	{/if}
	{#if courses.length === 0}
		<!-- Free slot: a faint centred marker, quieter than any tile, so filled
		     cells read first and an empty one is unmistakably empty (not unloaded). -->
		<span class="absolute inset-0 flex items-center justify-center text-fine text-apple-text-tertiary pointer-events-none">空き</span>
	{:else if visible}
		{#each courses as course (course.cd)}
			<CourseCard {course} onclick={() => onselect(course)} />
		{/each}
	{/if}
</div>
