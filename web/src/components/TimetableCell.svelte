<script lang="ts">
import type { Course } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: Course[]
	onselect: (course: Course) => void
}

let { courses, onselect }: Props = $props()
</script>

<div class="bg-surface-primary rounded-lg min-h-24 p-1.5">
	{#each courses as course (course.cd)}
		<CourseCard {course} onclick={() => onselect(course)} />
	{/each}
</div>
