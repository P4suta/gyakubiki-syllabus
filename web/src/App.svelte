<script lang="ts">
import { onMount } from 'svelte'
import CourseModal from './components/CourseModal.svelte'
import Disclaimer from './components/Disclaimer.svelte'
import FilterBar from './components/FilterBar.svelte'
import Timetable from './components/Timetable.svelte'
import { CourseIndex } from './lib/course-index'
import { buildGrid, countUnique } from './lib/grid'
import { loadData } from './lib/load-data'
import type { Course, ProcessedData } from './types/course'

let loading = $state(true)
let error = $state<string | null>(null)
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

onMount(async () => {
	try {
		const processed = await loadData()
		data = processed
		if (processed.semesters.length > 0) {
			semester = processed.semesters[0]
		}
	} catch (e) {
		error = e instanceof Error ? e.message : 'データの読み込みに失敗しました'
	} finally {
		loading = false
	}
})
</script>

{#if loading}
	<div class="min-h-screen bg-gray-50 flex items-center justify-center">
		<div class="text-center">
			<div class="inline-block h-8 w-8 animate-spin rounded-full border-4 border-blue-600 border-r-transparent mb-4"></div>
			<p class="text-sm text-gray-500">データを読み込み中...</p>
		</div>
	</div>
{:else if error}
	<div class="min-h-screen bg-gray-50 flex items-center justify-center">
		<div class="bg-white border border-red-200 rounded-lg p-6 max-w-md text-center">
			<p class="text-red-600 font-medium mb-2">読み込みエラー</p>
			<p class="text-sm text-gray-600 whitespace-pre-line">{error}</p>
		</div>
	</div>
{:else if data}
	<Disclaimer />
	<div class="min-h-screen bg-gray-50 font-sans flex flex-col">
		<FilterBar
			semesters={data.semesters}
			departments={data.departments}
			bind:semester
			bind:department
			bind:searchText
			{displayCount}
			totalCount={data.courses.length}
		/>
		<Timetable {grid} onselect={(c) => { selectedCourse = c }} />
	</div>
	{#if selectedCourse}
		<CourseModal course={selectedCourse} onclose={() => { selectedCourse = null }} />
	{/if}
{/if}
