<script lang="ts">
import { onDestroy, onMount } from 'svelte'
import IconCheck from '~icons/ic/round-check'
import IconEventNote from '~icons/ic/round-event-note'
import IconSearchOff from '~icons/ic/round-search-off'
import CourseModal from './components/CourseModal.svelte'
import Disclaimer from './components/Disclaimer.svelte'
import FilterBar from './components/FilterBar.svelte'
import SearchBar from './components/SearchBar.svelte'
import Timetable from './components/Timetable.svelte'
import { type GridKey, type PlanSummaryResult, SyllabusEngine } from './lib/engine'
import { highlights } from './lib/highlight.svelte'
import { initPlanSync, plan, shareUrl } from './lib/plan.svelte'
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
let planSummary = $state<PlanSummaryResult | null>(null)

// cd → Course, so a related-course code in the modal can open that card.
const courseByCd = $derived(new Map((engine?.courses ?? []).map((c) => [c.cd, c])))
const knownCds = $derived(new Set(courseByCd.keys()))
function openByCd(cd: string) {
	const c = courseByCd.get(cd)
	if (c) selectedCourse = c
}

// Clear the search + facet filters (keeps the semester). Drives the empty state.
function resetFilters() {
	searchText = ''
	department = 'all'
	campus = 'all'
}

// Floating plan control: the pill toggles a small action menu (share / clear).
// There is no separate「マイ時間割」screen — the grid itself is the plan.
let planMenuOpen = $state(false)
let copied = $state(false)
let copyTimer: ReturnType<typeof setTimeout> | undefined
const totalCredits = $derived(planSummary?.credits.totalCredits ?? 0)

async function sharePlan() {
	try {
		await navigator.clipboard.writeText(shareUrl())
		copied = true
		clearTimeout(copyTimer)
		copyTimer = setTimeout(() => {
			copied = false
		}, 1800)
	} catch {
		copied = false
	}
}

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

// The registered courses laid onto the current semester's grid — a locked-in
// slot shows this in place of the search candidates (the grid IS the plan).
let planGrid = $state<Map<GridKey, Course[]>>(new Map())
$effect(() => {
	const e = engine
	const sem = semester
	const cds = [...plan.cds]
	if (!e || cds.length === 0) {
		planGrid = new Map()
		return
	}
	let cancelled = false
	e.planGrid(cds, sem)
		.then((g) => {
			if (!cancelled) planGrid = g
		})
		.catch(() => {})
	return () => {
		cancelled = true
	}
})

// A registered cell holding two or more courses is a real clash (you can't
// attend both) — mark it. Derived from what's actually shown this semester.
let conflictKeys = $derived.by(() => {
	const keys = new Set<GridKey>()
	for (const [key, courses] of planGrid) {
		if (courses.length > 1) keys.add(key)
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

// Settled copy of the count for the screen-reader live region, so typing a
// query doesn't announce every intermediate result.
let announcedCount = $state(0)
$effect(() => {
	const value = displayCount
	const timer = setTimeout(() => {
		announcedCount = value
	}, 500)
	return () => clearTimeout(timer)
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
	<!-- Skeleton shaped like the app shell (faux filter bar + timetable grid), so
	     the first paint reads as the real screen rather than a bare spinner. -->
	<div class="h-dvh bg-surface-page flex flex-col overflow-hidden animate-fade-in">
		<div class="glass-nav border-b border-overlay-subtle px-4 py-3 sm:px-6 flex items-center gap-3">
			<div class="h-5 w-16 rounded-lg bg-overlay-light animate-pulse"></div>
			<div class="h-7 w-44 rounded-full bg-overlay-light animate-pulse ml-auto"></div>
		</div>
		<div class="grow overflow-hidden p-2 sm:p-3">
			<div class="grid grid-cols-5 gap-1.5 sm:gap-2">
				{#each Array.from({ length: 35 }) as _, i}
					<div class="h-16 sm:h-20 rounded-lg bg-overlay-light animate-pulse" style="animation-delay: {(i % 5) * 70}ms"></div>
				{/each}
			</div>
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
	<!-- data-*-count: invisible counter anchor for the E2E suite (helpers.counts). -->
	<div
		class="h-dvh bg-surface-page font-sans flex flex-col overflow-hidden animate-fade-in"
		data-shown-count={displayCount}
		data-total-count={engine.courses.length}
	>
		<header>
			<FilterBar
				semesters={engine.dicts.semesters}
				departments={engine.dicts.departments}
				campuses={engine.dicts.campuses}
				bind:semester
				bind:department
				bind:campus
				bind:searchText
				{displayCount}
				generatedAt={engine.generatedAt}
			/>
			<SearchBar bind:searchText />
		</header>
		<!-- The visible count chip is gone; announce filter results to AT instead. -->
		<p class="sr-only" role="status">{announcedCount}件の科目を表示中</p>
		<!-- The landmark carries the flex chain so children keep their layout. -->
		<main class="flex flex-col flex-1 overflow-hidden">
			{#if displayCount === 0 && plan.count === 0}
				<!-- Empty state: nothing matches and no plan to fall back on. -->
				<div class="grow flex items-center justify-center p-6 animate-fade-in">
					<div class="text-center max-w-xs">
						<IconSearchOff class="w-12 h-12 mx-auto text-apple-text-tertiary mb-3" />
						<p class="text-cta text-apple-text font-semibold mb-1 tracking-tight">該当する科目がありません</p>
						<p class="text-caption text-apple-text-secondary mb-4 tracking-tight leading-relaxed">検索語や絞り込みを見直してみてください。</p>
						<button
							onclick={resetFilters}
							class="rounded-full bg-apple-blue text-on-accent px-4 py-2 text-cta font-normal hover:bg-apple-blue-hover transition-colors cursor-pointer"
						>
							条件をリセット
						</button>
					</div>
				</div>
			{:else}
				<Timetable {grid} {planGrid} {conflictKeys} days={engine.days} onselect={(c) => { selectedCourse = c }} />
			{/if}
		</main>
	</div>

	<!-- Floating plan control: a compact pill showing the total credits (red on a
	     timetable conflict) that toggles a small action menu — share / clear.
	     There is one plan, not many; the grid itself is it. -->
	{#if plan.count > 0}
		<div class="fixed right-4 bottom-4 safe-bottom z-nav flex flex-col items-end gap-2">
			{#if planMenuOpen}
				<!-- Click-away catcher: a fixed full-screen layer under the menu/pill
				     (both made `relative` so DOM order paints them above it). -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="fixed inset-0" onclick={() => { planMenuOpen = false }}></div>
				<div class="relative flex flex-col gap-0.5 rounded-2xl bg-surface-primary p-1.5 shadow-modal animate-dialog-in origin-bottom-right" role="menu">
					<button
						class="flex items-center gap-1.5 text-left rounded-xl px-4 py-2.5 text-cta text-apple-text active:bg-overlay-light sm:hover:bg-overlay-light transition-colors cursor-pointer"
						onclick={sharePlan}
						role="menuitem"
					>
						{#if copied}<IconCheck class="w-4 h-4 shrink-0" aria-hidden="true" />コピーしました{:else}共有リンクをコピー{/if}
					</button>
					<button
						class="text-left rounded-xl px-4 py-2.5 text-cta text-apple-red active:bg-overlay-light sm:hover:bg-overlay-light transition-colors cursor-pointer"
						onclick={() => { plan.clear(); planMenuOpen = false }}
						role="menuitem"
					>
						全消去
					</button>
				</div>
			{/if}
			<button
				class="relative flex items-center gap-1.5 rounded-full px-3.5 py-2 shadow-card text-cta font-normal cursor-pointer transition duration-200 ease-spring active:scale-95
					{conflictKeys.size > 0 ? 'bg-apple-red text-on-accent' : 'bg-apple-blue text-on-accent'}"
				onclick={() => { planMenuOpen = !planMenuOpen }}
				aria-haspopup="menu"
				aria-expanded={planMenuOpen}
				aria-label="履修プラン {totalCredits > 0 ? `合計${totalCredits}単位` : `${plan.count}科目`}"
			>
				<IconEventNote class="w-4 h-4 shrink-0" aria-hidden="true" />
				<span class="tabular-nums">{totalCredits > 0 ? `${totalCredits}単位` : `${plan.count}科目`}</span>
			</button>
		</div>
	{/if}

	{#if selectedCourse}
		<CourseModal
			course={selectedCourse}
			dicts={engine.dicts}
			year={engine.year}
			{knownCds}
			onclose={() => { selectedCourse = null }}
			onsearch={(q) => { searchText = q; selectedCourse = null }}
			onopencourse={openByCd}
		/>
	{/if}
{/if}
