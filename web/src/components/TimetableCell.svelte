<script lang="ts">
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: Course[]
	onselect: (course: Course) => void
}

let { courses, onselect }: Props = $props()

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
	class="bg-surface-primary rounded-lg min-h-24 p-1.5 content-auto"
>
	{#if visible}
		{#each courses as course (course.cd)}
			<CourseCard {course} onclick={() => onselect(course)} />
		{/each}
	{/if}
</div>
