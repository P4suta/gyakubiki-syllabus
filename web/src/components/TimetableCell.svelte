<script lang="ts">
import type { CourseV2 } from '../types/course'
import CourseCard from './CourseCard.svelte'

interface Props {
	courses: CourseV2[]
	onselect: (course: CourseV2) => void
}

let { courses, onselect }: Props = $props()
</script>

<div class="bg-white rounded-lg min-h-24 p-1.5">
	{#each courses as course (course.cd)}
		<CourseCard {course} onclick={() => onselect(course)} />
	{/each}
</div>
