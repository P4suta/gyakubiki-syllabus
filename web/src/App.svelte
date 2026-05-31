<script lang="ts">
import { onMount } from 'svelte'
import CourseModal from './components/CourseModal.svelte'
import Disclaimer from './components/Disclaimer.svelte'
import FilterBar from './components/FilterBar.svelte'
import SearchBar from './components/SearchBar.svelte'
import Timetable from './components/Timetable.svelte'
import { SyllabusEngine } from './lib/engine'
import type { Course } from './types/course'

let loading = $state(true)
let error = $state<string | null>(null)
let engine = $state<SyllabusEngine | null>(null)
let semester = $state('all')
let department = $state('all')
let campus = $state('all')
let searchText = $state('')
let debouncedSearch = $state('')
let selectedCourse: Course | null = $state(null)

$effect(() => {
	const value = searchText
	const timer = setTimeout(() => { debouncedSearch = value }, 180)
	return () => clearTimeout(timer)
})

// filter → course indices; grid → resolved cells + distinct count. The flat
// filtered list never materializes — it only ever fed the grid.
let filtered = $derived(
	engine ? engine.filter(semester, department, campus, debouncedSearch) : new Uint32Array(),
)
let layout = $derived(engine ? engine.grid(filtered, semester) : null)
let grid = $derived(layout ? layout.grid : new Map())
let displayCount = $derived(layout ? layout.count : 0)

onMount(async () => {
	try {
		engine = await SyllabusEngine.create()
		if (engine.dicts.semesters.length > 0) {
			semester = engine.dicts.semesters[0]
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
{:else if engine}
	<Disclaimer />
	<div class="h-dvh bg-surface-page font-sans flex flex-col overflow-hidden animate-fade-in">
		<FilterBar
			semesters={engine.dicts.semesters}
			departments={engine.dicts.departments}
			campuses={engine.dicts.campuses}
			bind:semester
			bind:department
			bind:campus
			bind:searchText
			{displayCount}
			totalCount={engine.courses.length}
			generatedAt={engine.generatedAt}
		/>
		<SearchBar bind:searchText />
		<Timetable {grid} days={engine.days} onselect={(c) => { selectedCourse = c }} />
	</div>
	{#if selectedCourse}
		<CourseModal course={selectedCourse} dicts={engine.dicts} onclose={() => { selectedCourse = null }} />
	{/if}
{/if}
