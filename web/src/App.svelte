<script lang="ts">
import { onMount } from 'svelte'
import CourseModal from './components/CourseModal.svelte'
import Disclaimer from './components/Disclaimer.svelte'
import FilterBar from './components/FilterBar.svelte'
import Timetable from './components/Timetable.svelte'
import { initDeptColors } from './lib/colors'
import { CourseIndex } from './lib/course-index'
import { buildGrid, countUnique } from './lib/grid'
import { loadData } from './lib/load-data'
import type { CourseV2, ProcessedDataV2 } from './types/course'

let loading = $state(true)
let error = $state<string | null>(null)
let data = $state<ProcessedDataV2 | null>(null)
let semester = $state('all')
let department = $state('all')
let campus = $state('all')
let searchText = $state('')
let debouncedSearch = $state('')
let selectedCourse: CourseV2 | null = $state(null)

$effect(() => {
	const value = searchText
	const timer = setTimeout(() => { debouncedSearch = value }, 180)
	return () => clearTimeout(timer)
})

let index = $derived(data ? new CourseIndex(data.courses, data.dicts, data.indices) : null)
let filteredCourses = $derived(
	index ? index.filter(semester, department, campus, debouncedSearch) : [],
)
let grid = $derived(data ? buildGrid(filteredCourses, semester, data.dicts) : new Map())
let displayCount = $derived(countUnique(grid))

onMount(async () => {
	try {
		const processed = await loadData()
		initDeptColors(processed.dicts.departments)
		data = processed
		if (processed.dicts.semesters.length > 0) {
			semester = processed.dicts.semesters[0]
		}
	} catch (e) {
		error = e instanceof Error ? e.message : 'データの読み込みに失敗しました'
	} finally {
		loading = false
	}
})
</script>

{#if loading}
	<div class="min-h-screen bg-surface-page flex items-center justify-center">
		<div class="text-center">
			<div class="inline-block w-5 h-5 border-2 border-apple-text/20 border-t-apple-text rounded-full mb-4 animate-spinner"></div>
			<p class="text-body text-apple-text/60 tracking-tight">データを読み込み中...</p>
		</div>
	</div>
{:else if error}
	<div class="min-h-screen bg-surface-page flex items-center justify-center">
		<div class="bg-surface-primary rounded-xl p-8 max-w-md text-center shadow-card">
			<p class="text-cta text-apple-text font-semibold mb-2 tracking-tight">読み込みエラー</p>
			<p class="text-body text-apple-text/60 whitespace-pre-line leading-relaxed tracking-tight">{error}</p>
		</div>
	</div>
{:else if data}
	<Disclaimer />
	<div class="h-screen bg-surface-page font-sans flex flex-col overflow-hidden animate-fade-in">
		<FilterBar
			semesters={data.dicts.semesters}
			departments={data.dicts.departments}
			campuses={data.dicts.campuses}
			bind:semester
			bind:department
			bind:campus
			bind:searchText
			{displayCount}
			totalCount={data.courses.length}
		/>
		<Timetable {grid} onselect={(c) => { selectedCourse = c }} />
	</div>
	{#if selectedCourse}
		<CourseModal course={selectedCourse} dicts={data.dicts} onclose={() => { selectedCourse = null }} />
	{/if}
{/if}
