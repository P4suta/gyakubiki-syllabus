<script lang="ts">
import CourseModal from './components/CourseModal.svelte'
import FileLoader from './components/FileLoader.svelte'
import FilterBar from './components/FilterBar.svelte'
import Timetable from './components/Timetable.svelte'
import { CourseIndex } from './lib/course-index'
import { buildGrid, countUnique } from './lib/grid'
import type { Course, ProcessedData } from './types/course'

let data = $state<ProcessedData | null>(null)
let semester = $state('all')
let department = $state('all')
let searchText = $state('')
let debouncedSearch = $state('')
let selectedCourse: Course | null = $state(null)

$effect(() => {
	const value = searchText
	const timer = setTimeout(() => { debouncedSearch = value }, 180)
	return () => clearTimeout(timer)
})

let index = $derived(data ? new CourseIndex(data.courses) : null)
let filteredCourses = $derived(
	index ? index.filter(semester, department, debouncedSearch) : [],
)
let grid = $derived(buildGrid(filteredCourses, semester))
let displayCount = $derived(countUnique(grid))

function handleDataLoaded(processed: ProcessedData) {
	console.log('[App] handleDataLoaded called, courses:', processed.courses.length)
	data = processed
	console.log('[App] data set, semesters:', processed.semesters)
	if (processed.semesters.length > 0) {
		semester = processed.semesters[0]
		console.log('[App] semester set to:', semester)
	}
}
</script>

{#if !data}
	<FileLoader onload={handleDataLoaded} />
{:else}
	<div class="min-h-screen bg-gray-50 font-sans">
		<FilterBar
			semesters={data.semesters}
			departments={data.departments}
			bind:semester
			bind:department
			bind:searchText
			{displayCount}
			totalCount={data.courses.length}
			onChangeData={() => { data = null }}
		/>
		<Timetable {grid} onselect={(c) => { selectedCourse = c }} />
	</div>
	{#if selectedCourse}
		<CourseModal course={selectedCourse} onclose={() => { selectedCourse = null }} />
	{/if}
{/if}
