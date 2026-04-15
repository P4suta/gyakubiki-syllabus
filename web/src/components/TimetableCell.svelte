<script lang="ts">
import type { CourseV2 } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: CourseV2[]
	isSaturday: boolean
	onselect: (course: CourseV2) => void
}

let { courses, isSaturday, onselect }: Props = $props()
</script>

<div
	class="min-h-20 p-1 rounded border border-gray-100 {isSaturday ? 'bg-amber-50' : 'bg-white'}"
>
	{#each courses as course (course.cd)}
		<CourseCard {course} onclick={() => onselect(course)} />
	{/each}
</div>
