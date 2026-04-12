<script lang="ts">
import CourseModal from './components/CourseModal.svelte'
import FileLoader from './components/FileLoader.svelte'
import FilterBar from './components/FilterBar.svelte'
import Timetable from './components/Timetable.svelte'
import { filterCourses } from './lib/filters'
import { buildGrid, countUnique } from './lib/grid'
import type { Course, ProcessedData } from './types/course'

let data: ProcessedData | null = $state(null)
let semester = $state('all')
let department = $state('all')
let searchText = $state('')
let selectedCourse: Course | null = $state(null)

let filteredCourses = $derived(
	data ? filterCourses(data.courses, semester, department, searchText) : [],
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
