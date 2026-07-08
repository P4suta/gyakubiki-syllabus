<script lang="ts">
import { onMount } from 'svelte'
import CourseModal from './components/CourseModal.svelte'
import Disclaimer from './components/Disclaimer.svelte'
import FilterBar from './components/FilterBar.svelte'
import SearchBar from './components/SearchBar.svelte'
import Timetable from './components/Timetable.svelte'
import { type GridKey, SyllabusEngine } from './lib/engine'
import { defaultSemester } from './lib/semester'
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
	const timer = setTimeout(() => {
		debouncedSearch = value
	}, 180)
	return () => clearTimeout(timer)
})

// The engine now lives in a worker, so filter+grid is async (one round-trip per
// change). This effect re-runs on any selector/query change; a cancel flag drops
// a stale result if a newer query resolves first. The last good grid stays on
// screen until the next one arrives — no flicker between queries.
let grid = $state<Map<GridKey, Course[]>>(new Map())
let displayCount = $state(0)

$effect(() => {
	const sem = semester
	const dep = department
	const cam = campus
	const q = debouncedSearch
	const e = engine
	if (!e) return
	let cancelled = false
	e.filterAndGrid(sem, dep, cam, q)
		.then((r) => {
			if (!cancelled) {
				grid = r.grid
				displayCount = r.count
			}
		})
		.catch(() => {
			// A failed query (e.g. worker hiccup) is non-fatal — keep the last grid.
		})
	return () => {
		cancelled = true
	}
})

onMount(async () => {
	try {
		engine = await SyllabusEngine.create()
		// Default to the term in session now (falls back to「全て」off-season).
		semester = defaultSemester(engine.dicts.semesters)
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
			<div class="inline-block w-5 h-5 border-2 border-overlay-subtle border-t-apple-blue rounded-full mb-4 animate-spinner"></div>
			<p class="text-body text-apple-text-secondary tracking-tight">データを読み込み中...</p>
		</div>
	</div>
{:else if error}
	<div class="min-h-screen bg-surface-page flex items-center justify-center">
		<div class="bg-surface-primary rounded-xl p-8 max-w-md text-center shadow-card">
			<p class="text-cta text-apple-text font-semibold mb-2 tracking-tight">読み込みエラー</p>
			<p class="text-body text-apple-text-secondary whitespace-pre-line leading-relaxed tracking-tight">{error}</p>
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
		<CourseModal course={selectedCourse} dicts={engine.dicts} year={engine.year} onclose={() => { selectedCourse = null }} />
	{/if}
{/if}
