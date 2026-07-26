<script lang="ts">
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: Course[]
	onselect: (course: Course) => void
	/** Keep the first interaction light while making every result reachable. */
	initialSize?: number
	pageSize?: number
	className?: string
	label: string
}

let { courses, onselect, initialSize = 24, pageSize = 48, className = '', label }: Props = $props()

let limit = $state(0)

// A filter/query result replaces the source array. Reset pagination so stale
// limits do not turn a narrow result back into a large synchronous render.
$effect(() => {
	const resultCount = courses.length
	limit = Math.min(resultCount, initialSize)
})

const visibleCourses = $derived(courses.slice(0, limit))
const remaining = $derived(Math.max(0, courses.length - visibleCourses.length))

function showMore() {
	limit = Math.min(courses.length, limit + pageSize)
}
</script>

<div class={className}>
	{#each visibleCourses as course (course.cd)}
		<CourseCard {course} onclick={() => onselect(course)} />
	{/each}
</div>
{#if remaining > 0}
	<button
		type="button"
		onclick={showMore}
		class="mx-auto mt-3 block min-h-tap rounded-full border border-overlay-strong bg-surface-primary px-5 py-2 text-caption font-medium text-apple-blue cursor-pointer"
		aria-label="{label}をさらに{Math.min(pageSize, remaining)}件表示（残り{remaining}件）"
	>
		さらに{Math.min(pageSize, remaining)}件表示
		<span class="ml-1 text-apple-text-tertiary">残り{remaining}件</span>
	</button>
{/if}
