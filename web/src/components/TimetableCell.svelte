<script lang="ts">
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: Course[]
	isSaturday: boolean
	onselect: (course: Course) => void
}

let { courses, isSaturday, onselect }: Props = $props()
</script>

<div
	class="min-h-20 p-1 rounded border border-gray-100 {isSaturday ? 'bg-amber-50' : 'bg-white'}"
>
	{#each courses as course (course.kogiCd)}
		<CourseCard {course} onclick={() => onselect(course)} />
	{/each}
</div>
