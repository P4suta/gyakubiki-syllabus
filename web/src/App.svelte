<script lang="ts">
import { onDestroy, onMount } from 'svelte'
import CourseModal from './components/CourseModal.svelte'
import Disclaimer from './components/Disclaimer.svelte'
import FilterBar from './components/FilterBar.svelte'
import PlanPanel from './components/PlanPanel.svelte'
import SearchBar from './components/SearchBar.svelte'
import Timetable from './components/Timetable.svelte'
import { type GridKey, type PlanSummaryResult, SyllabusEngine } from './lib/engine'
import { highlights } from './lib/highlight.svelte'
import { initPlanSync, plan } from './lib/plan.svelte'
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
let showPlan = $state(false)
let planSummary = $state<PlanSummaryResult | null>(null)

$effect(() => {
	const value = searchText
	const timer = setTimeout(() => {
		debouncedSearch = value
	}, 180)
	return () => clearTimeout(timer)
})

// Recompute the plan summary (conflicts + credits) whenever the plan or engine
// changes. cd → index → summary, one worker round-trip; a cancel flag drops a
// stale result. `plan.count` is read so this re-runs on add/remove.
$effect(() => {
	const e = engine
	const cds = [...plan.cds]
	if (!e || cds.length === 0) {
		planSummary = null
		return
	}
	let cancelled = false
	e.resolvePlan(cds)
		.then((indices) => e.planSummary(indices))
		.then((summary) => {
			if (!cancelled) planSummary = summary
		})
		.catch(() => {})
	return () => {
		cancelled = true
	}
})

// Conflicting timetable cells, as grid keys, for the highlight overlay.
let conflictKeys = $derived.by(() => {
	const keys = new Set<GridKey>()
	if (!engine || !planSummary) return keys
	for (const c of planSummary.conflicts) {
		const day = engine.days[c.day]
		if (day !== undefined) keys.add(`${day}-${c.period}`)
	}
	return keys
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
				highlights.set(r.highlights)
			}
		})
		.catch(() => {
			// A failed query (e.g. worker hiccup) is non-fatal — keep the last grid.
		})
	return () => {
		cancelled = true
	}
})

let teardownPlanSync: (() => void) | undefined

onMount(async () => {
	teardownPlanSync = initPlanSync() // URL hash ↔ localStorage ↔ plan store
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

onDestroy(() => teardownPlanSync?.())
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
		<Timetable {grid} {conflictKeys} days={engine.days} onselect={(c) => { selectedCourse = c }} />
	</div>

	<!-- Floating「マイ時間割」button: opens the plan panel; badges the count and
	     turns red when the plan has a timetable conflict. -->
	{#if plan.count > 0}
		<button
			class="fixed right-4 bottom-4 safe-bottom z-nav flex items-center gap-2 rounded-full px-4 py-3 shadow-card text-cta font-normal cursor-pointer transition-colors
				{conflictKeys.size > 0 ? 'bg-apple-red text-on-accent' : 'bg-apple-blue text-on-accent'}"
			onclick={() => { showPlan = true }}
		>
			マイ時間割 {plan.count}
		</button>
	{/if}

	{#if showPlan}
		<PlanPanel
			summary={planSummary}
			courses={engine.courses}
			onselect={(c) => { showPlan = false; selectedCourse = c }}
			onclose={() => { showPlan = false }}
		/>
	{/if}

	{#if selectedCourse}
		<CourseModal course={selectedCourse} dicts={engine.dicts} year={engine.year} onclose={() => { selectedCourse = null }} />
	{/if}
{/if}
