<script lang="ts">
import { onDestroy, onMount } from 'svelte'
import IconCheck from '~icons/ic/round-check'
import IconEventNote from '~icons/ic/round-event-note'
import IconSearchOff from '~icons/ic/round-search-off'
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
type CourseModalComponent = typeof import('./components/CourseModal.svelte')['default']
let courseModalComponent = $state<CourseModalComponent | null>(null)
let courseModalError = $state<string | null>(null)
let courseModalLoad: Promise<void> | null = null
let planSummary = $state<PlanSummaryResult | null>(null)
let unscheduled = $state<Course[]>([])
let queryError = $state<string | null>(null)
let queryPending = $state(false)
let planError = $state<string | null>(null)
let planNotice = $state<string | null>(null)
let queryEpoch = $state(0)
let offline = $state(false)
let offlineReady = $state(false)
let updateAvailable = $state(false)
let pwaError = $state<string | null>(null)
let updateServiceWorker: ((reloadPage?: boolean) => Promise<void>) | undefined
let teardownNetwork: (() => void) | undefined

async function loadCourseModal() {
	if (courseModalComponent) return
	courseModalError = null
	courseModalLoad ??= import('./components/CourseModal.svelte')
		.then((module) => {
			courseModalComponent = module.default
		})
		.finally(() => {
			courseModalLoad = null
		})
	try {
		await courseModalLoad
	} catch (reason) {
		courseModalError = reason instanceof Error ? reason.message : '詳細画面を読み込めませんでした'
	}
}

$effect(() => {
	if (selectedCourse && !courseModalComponent && !courseModalError) void loadCourseModal()
})

$effect(() => {
	const notice = plan.notice
	if (notice) {
		planNotice = notice
		plan.clearNotice()
	}
})

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
let planButton = $state<HTMLButtonElement>()
let planMenu = $state<HTMLDivElement>()
let aboutOpen = $state(false)
let aboutButton = $state<HTMLButtonElement>()
let aboutDialog = $state<HTMLDialogElement>()
let copied = $state(false)
let copyTimer: ReturnType<typeof setTimeout> | undefined
const focusFrames = new Set<number>()
const totalCredits = $derived(planSummary?.credits.totalCredits ?? 0)

function focusNextFrame(callback: () => void) {
	const frame = requestAnimationFrame(() => {
		focusFrames.delete(frame)
		callback()
	})
	focusFrames.add(frame)
}

function closePlanMenu() {
	planMenuOpen = false
	focusNextFrame(() => planButton?.focus())
}

function closeAbout() {
	aboutOpen = false
	focusNextFrame(() => aboutButton?.focus())
}

function onGlobalKeydown(event: KeyboardEvent) {
	if (event.key === 'Escape') {
		if (aboutOpen) {
			event.preventDefault()
			closeAbout()
			return
		}
		if (!planMenuOpen) return
		event.preventDefault()
		closePlanMenu()
	}
}

$effect(() => {
	const menu = planMenu
	if (!planMenuOpen || !menu) return
	const frame = requestAnimationFrame(() =>
		menu.querySelector<HTMLButtonElement>('button')?.focus(),
	)
	return () => cancelAnimationFrame(frame)
})

$effect(() => {
	const dialog = aboutDialog
	if (!dialog) return
	if (aboutOpen && !dialog.open) dialog.showModal()
	if (!aboutOpen && dialog.open) dialog.close()
})

async function sharePlan() {
	const url = shareUrl()
	try {
		await navigator.clipboard.writeText(url)
		copied = true
		clearTimeout(copyTimer)
		copyTimer = setTimeout(() => {
			copied = false
		}, 1800)
	} catch {
		copied = false
		if (navigator.share) {
			try {
				await navigator.share({ title: '逆引きシラバス 履修プラン', url })
				return
			} catch {
				// User cancellation falls through to a selectable prompt.
			}
		}
		window.prompt('共有URLをコピーしてください', url)
	}
}

$effect(() => {
	const value = searchText
	const timer = setTimeout(() => {
		debouncedSearch = value
	}, 180)
	return () => clearTimeout(timer)
})

// Resolve codes, remove retired entries, lay out scheduled courses, retain
// intensive/TBA courses, and compute conflicts/credits in one worker round-trip.
let planGrid = $state<Map<GridKey, Course[]>>(new Map())
let planUnscheduled = $state<Course[]>([])
$effect(() => {
	const e = engine
	const cds = [...plan.cds]
	const sem = semester
	if (!e || cds.length === 0) {
		planSummary = null
		planGrid = new Map()
		planUnscheduled = []
		return
	}
	let cancelled = false
	e.plan(cds, sem)
		.then((result) => {
			if (cancelled) return
			if (result.unknownCodes.length > 0) {
				plan.hydrate(result.validCodes)
				planNotice = `${result.unknownCodes.length}件の廃止・不明科目を履修プランから除外しました`
			}
			planSummary = { conflicts: result.conflicts, credits: result.credits }
			planGrid = result.grid
			planUnscheduled = result.unscheduled
			planError = null
		})
		.catch((reason) => {
			if (!cancelled)
				planError = reason instanceof Error ? reason.message : '履修プランを計算できません'
		})
	return () => {
		cancelled = true
	}
})

let visibleUnscheduled = $derived.by(() => {
	const values = new Map<string, Course>()
	for (const course of [...planUnscheduled, ...unscheduled]) values.set(course.cd, course)
	return [...values.values()]
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
	const requestEpoch = queryEpoch
	const e = engine
	if (!e) return
	let cancelled = false
	queryPending = true
	e.filterAndGrid(sem, dep, cam, q)
		.then((r) => {
			if (!cancelled && requestEpoch === queryEpoch) {
				grid = r.grid
				unscheduled = r.unscheduled
				displayCount = r.count
				highlights.set(r.highlights)
				highlights.setFields(r.matchFields)
				queryError = null
				queryPending = false
			}
		})
		.catch((reason) => {
			if (cancelled || (reason instanceof DOMException && reason.name === 'AbortError')) return
			queryError = reason instanceof Error ? reason.message : '検索に失敗しました'
			queryPending = false
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

async function loadEngine() {
	loading = true
	error = null
	engine?.dispose()
	try {
		engine = await SyllabusEngine.create()
		if (import.meta.env.DEV && import.meta.env.VITE_E2E === 'true') {
			window.__GYAKUBIKI_E2E__ = {
				crashNextWorkerRequest: () => engine?.debugCrashNextRequest(),
				stallNextWorkerRequest: () => engine?.debugStallNextRequest(),
			}
		}
		semester = defaultSemester(engine.dicts.semesters)
	} catch (reason) {
		error = reason instanceof Error ? reason.message : 'データの読み込みに失敗しました'
	} finally {
		loading = false
	}
}

async function retrySearch() {
	queryError = null
	try {
		await engine?.retrySearchIndex()
		queryEpoch += 1
	} catch (reason) {
		queryError = reason instanceof Error ? reason.message : '全文検索を再開できません'
	}
}

async function registerPwa() {
	if (!('serviceWorker' in navigator)) return
	pwaError = null
	try {
		const { registerSW } = await import('virtual:pwa-register')
		updateServiceWorker = registerSW({
			immediate: true,
			onNeedRefresh() {
				updateAvailable = true
			},
			onOfflineReady() {
				offlineReady = true
			},
			onRegisterError(reason) {
				pwaError = reason instanceof Error ? reason.message : 'オフライン機能を開始できません'
			},
		})
	} catch (reason) {
		pwaError = reason instanceof Error ? reason.message : 'オフライン機能を開始できません'
	}
}

async function applyUpdate() {
	try {
		if (!updateServiceWorker) throw new Error('更新機能を再登録してください')
		await updateServiceWorker(true)
	} catch (reason) {
		pwaError = reason instanceof Error ? reason.message : 'アプリを更新できません'
	}
}

onMount(async () => {
	teardownPlanSync = initPlanSync() // URL hash ↔ localStorage ↔ plan store
	// Keep the modal out of the main JavaScript chunk, but warm its async chunk
	// while the worker initializes so the first course click is immediate even
	// on browsers whose development module compilation is comparatively slow.
	void loadCourseModal()
	offline = navigator.onLine === false
	const onOnline = () => {
		offline = false
	}
	const onOffline = () => {
		offline = true
	}
	window.addEventListener('online', onOnline)
	window.addEventListener('offline', onOffline)
	teardownNetwork = () => {
		window.removeEventListener('online', onOnline)
		window.removeEventListener('offline', onOffline)
	}
	void registerPwa()
	await loadEngine()
})

onDestroy(() => {
	teardownPlanSync?.()
	teardownNetwork?.()
	engine?.dispose()
	delete window.__GYAKUBIKI_E2E__
	clearTimeout(copyTimer)
	for (const frame of focusFrames) cancelAnimationFrame(frame)
	focusFrames.clear()
})
</script>

<svelte:window onkeydown={onGlobalKeydown} />

<!-- Mounted over the loading skeleton, not the filled grid: showModal() forces
     a synchronous layout of whatever is already in the DOM, and opening it in
     the same flush that renders ~5k grid elements doubled that work (the boot
     forced-reflow Lighthouse flagged). Over the skeleton it costs ~nothing, and
     the disclaimer is readable while data loads. -->
{#if !error}
	<Disclaimer />
{/if}

{#if loading}
	<!-- Skeleton shaped like the app shell (faux filter bar + timetable grid), so
	     the first paint reads as the real screen rather than a bare spinner. -->
	<main id="main" tabindex="-1" class="h-dvh bg-surface-page flex flex-col overflow-hidden animate-fade-in outline-none">
		<h1 class="sr-only">時間割を読み込み中</h1>
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
	</main>
{:else if error}
	<div class="min-h-screen bg-surface-page flex items-center justify-center">
		<div class="bg-surface-primary rounded-xl p-8 max-w-md text-center shadow-card">
			<p class="text-cta text-apple-text font-semibold mb-2 tracking-tight">読み込みエラー</p>
			<p class="text-body text-apple-text-secondary whitespace-pre-line leading-relaxed tracking-tight">{error}</p>
			<button
				onclick={loadEngine}
				class="mt-5 rounded-full bg-apple-blue text-on-accent px-4 py-2 text-cta cursor-pointer"
			>
				再試行
			</button>
		</div>
	</div>
{:else if engine}
	<!-- data-*-count: invisible counter anchor for the E2E suite (helpers.counts). -->
	<div
		class="h-dvh bg-surface-page font-sans flex flex-col overflow-hidden animate-fade-in"
		data-shown-count={displayCount}
		data-total-count={engine.courses.length}
	>
		<header>
			<a
				href="#main"
				class="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-nav focus:rounded-full focus:bg-apple-blue focus:text-on-accent focus:px-4 focus:py-2 focus:text-caption focus:font-medium"
			>本文へスキップ</a>
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
			<div class="flex items-center justify-between gap-3 px-4 py-1 text-fine text-apple-text-tertiary bg-surface-page border-b border-overlay-subtle">
				<span>非公式ツール — 履修判断は必ず大学公式情報で確認してください</span>
				<button
					bind:this={aboutButton}
					onclick={() => { aboutOpen = true }}
					class="shrink-0 underline underline-offset-2 cursor-pointer"
					aria-haspopup="dialog"
				>
					Data {engine.generatedAt.slice(0, 10)} · {engine.datasetId.slice(0, 8)}
				</button>
			</div>
			{#if offline}
				<div role="status" class="px-4 py-2 text-caption bg-overlay-subtle text-apple-text">
					オフラインです。保存済みデータを表示しています。未取得の詳細や更新は利用できません。
				</div>
			{/if}
			{#if updateAvailable}
				<div role="alert" class="px-4 py-2 flex items-center gap-3 bg-apple-blue/10 text-caption text-apple-text">
					<span class="grow">新しいアプリversionを利用できます。</span>
					<button onclick={applyUpdate} class="rounded-full bg-apple-blue px-3 py-1.5 text-on-accent cursor-pointer">更新</button>
				</div>
			{/if}
			{#if pwaError}
				<div role="alert" class="px-4 py-2 flex items-center gap-3 bg-apple-red/10 text-caption text-apple-red">
					<span class="grow">オフライン・更新機能: {pwaError}</span>
					<button onclick={registerPwa} class="rounded-full bg-apple-blue px-3 py-1.5 text-on-accent cursor-pointer">再試行</button>
				</div>
			{:else if offlineReady}
				<div role="status" class="px-4 py-1.5 flex items-center gap-3 bg-apple-blue/10 text-fine text-apple-text">
					<span class="grow">オフライン用のアプリを準備しました。</span>
					<button onclick={() => { offlineReady = false }} class="underline cursor-pointer">閉じる</button>
				</div>
			{/if}
		</header>
		<!-- The landmark carries the flex chain so children keep their layout. -->
		<main id="main" tabindex="-1" class="flex flex-col flex-1 overflow-hidden outline-none">
			<!-- The visible count chip is gone; announce filter results to AT instead. -->
			<p class="sr-only" role="status">{announcedCount}件の科目を表示中</p>
			{#if queryError}
				<div role="alert" class="mx-3 mt-2 rounded-xl bg-surface-primary px-4 py-3 shadow-card flex items-center gap-3">
					<p class="text-caption text-apple-red grow">{queryError}</p>
					<button onclick={retrySearch} class="rounded-full bg-apple-blue text-on-accent px-3 py-1.5 text-caption cursor-pointer">再試行</button>
				</div>
			{:else if debouncedSearch && queryPending}
				<p role="status" class="mx-3 mt-2 text-caption text-apple-text-secondary">
					全文検索を準備・実行しています…
				</p>
			{/if}
			{#if planError || planNotice}
				<p role="status" class="mx-3 mt-2 text-caption {planError ? 'text-apple-red' : 'text-apple-text-secondary'}">
					{planError ?? planNotice}
				</p>
			{/if}
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
				<Timetable
					{grid}
					{planGrid}
					{conflictKeys}
					unscheduled={visibleUnscheduled}
					days={engine.days}
					periods={engine.periods}
					onselect={(c) => { selectedCourse = c }}
				/>
			{/if}
		</main>
	</div>

	<!-- Floating plan control: a compact pill showing the total credits (red on a
	     timetable conflict) that toggles a small action popover — share / clear.
	     There is one plan, not many; the grid itself is it. -->
	{#if plan.count > 0}
		<aside
			aria-label="履修プラン"
			class="fixed right-4 bottom-4 safe-bottom z-nav flex flex-col items-end gap-2"
		>
			{#if planMenuOpen}
				<button class="fixed inset-0 cursor-default" aria-label="履修プラン操作を閉じる" onclick={closePlanMenu}></button>
				<div
					id="plan-actions"
					bind:this={planMenu}
					class="relative flex flex-col gap-0.5 rounded-2xl bg-surface-primary p-1.5 shadow-modal animate-dialog-in origin-bottom-right"
				>
					<button
						class="flex items-center gap-1.5 text-left rounded-xl px-4 py-2.5 text-cta text-apple-text active:bg-overlay-light sm:hover:bg-overlay-light transition-colors cursor-pointer"
						onclick={sharePlan}
					>
						{#if copied}<IconCheck class="w-4 h-4 shrink-0" aria-hidden="true" />コピーしました{:else}共有リンクをコピー{/if}
					</button>
					<button
						class="text-left rounded-xl px-4 py-2.5 text-cta text-apple-red active:bg-overlay-light sm:hover:bg-overlay-light transition-colors cursor-pointer"
						onclick={() => { plan.clear(); planMenuOpen = false }}
					>
						全消去
					</button>
				</div>
			{/if}
			<button
				bind:this={planButton}
				class="relative flex items-center gap-1.5 rounded-full px-3.5 py-2 shadow-card text-cta font-normal cursor-pointer transition duration-200 ease-spring active:scale-95
					{conflictKeys.size > 0 ? 'bg-apple-red text-on-accent' : 'bg-apple-blue text-on-accent'}"
				onclick={() => { planMenuOpen = !planMenuOpen }}
				aria-controls="plan-actions"
				aria-expanded={planMenuOpen}
				aria-label="履修プラン {totalCredits > 0 ? `合計${totalCredits}単位` : `${plan.count}科目`}"
			>
				<IconEventNote class="w-4 h-4 shrink-0" aria-hidden="true" />
				<span class="tabular-nums">{totalCredits > 0 ? `${totalCredits}単位` : `${plan.count}科目`}</span>
			</button>
		</aside>
	{/if}

	{#if selectedCourse && courseModalComponent}
		{@const Modal = courseModalComponent}
		<Modal
			course={selectedCourse}
			dicts={engine.dicts}
			year={engine.year}
			{knownCds}
			onclose={() => { selectedCourse = null }}
			onsearch={(q) => { searchText = q; selectedCourse = null }}
			onopencourse={openByCd}
		/>
	{:else if selectedCourse}
		<div class="fixed inset-0 z-nav grid place-items-center bg-overlay-backdrop p-6">
			<div class="rounded-xl bg-surface-primary p-5 shadow-modal" role={courseModalError ? 'alert' : 'status'}>
				<p class="text-body text-apple-text">
					{courseModalError ? `詳細画面: ${courseModalError}` : '詳細画面を読み込んでいます…'}
				</p>
				<div class="mt-3 flex justify-end gap-2">
					{#if courseModalError}
						<button
							class="rounded-full bg-apple-blue px-3 py-1.5 text-caption text-on-accent cursor-pointer"
							onclick={loadCourseModal}
						>
							再試行
						</button>
					{/if}
					<button
						class="rounded-full px-3 py-1.5 text-caption text-apple-text cursor-pointer"
						onclick={() => { selectedCourse = null }}
					>
						閉じる
					</button>
				</div>
			</div>
		</div>
	{/if}

	<dialog
		bind:this={aboutDialog}
		class="overlay"
		aria-labelledby="about-title"
		oncancel={(event) => {
			event.preventDefault()
			closeAbout()
		}}
	>
		<button
			class="fixed inset-0 bg-overlay-backdrop cursor-default"
			aria-label="Aboutとデータ状態を閉じる"
			onclick={closeAbout}
		></button>
		<section
			class="fixed inset-x-4 top-1/2 -translate-y-1/2 mx-auto max-w-lg rounded-2xl bg-surface-primary p-5 shadow-modal"
		>
			<div class="flex items-start justify-between gap-4">
				<div>
					<h2 id="about-title" class="text-title font-semibold text-apple-text">About / Data Status</h2>
					<p class="mt-1 text-caption text-apple-text-secondary">非公式の高知大学シラバス検索ツールです。</p>
				</div>
				<button
					onclick={closeAbout}
					class="rounded-full bg-overlay-light px-3 py-1.5 text-caption text-apple-text cursor-pointer"
				>閉じる</button>
			</div>
			<dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-caption">
				<dt class="text-apple-text-tertiary">App</dt>
				<dd class="text-apple-text">v{__APP_VERSION__} · {__APP_COMMIT__.slice(0, 12)}</dd>
				<dt class="text-apple-text-tertiary">Dataset</dt>
				<dd class="text-apple-text break-all">{engine.datasetId}</dd>
				<dt class="text-apple-text-tertiary">Source commit</dt>
				<dd class="text-apple-text break-all">{engine.manifest.sourceCommit}</dd>
				<dt class="text-apple-text-tertiary">Generated</dt>
				<dd class="text-apple-text">{engine.generatedAt}</dd>
				<dt class="text-apple-text-tertiary">Courses</dt>
				<dd class="text-apple-text tabular-nums">
					{engine.manifest.counts.courses}（時刻指定 {engine.manifest.counts.scheduledCourses} / 集中・未定 {engine.manifest.counts.unscheduledCourses}）
				</dd>
				<dt class="text-apple-text-tertiary">Details</dt>
				<dd class="text-apple-text tabular-nums">
					{engine.manifest.counts.details} / {engine.manifest.counts.courses}
					({(engine.manifest.counts.detailCoverage * 100).toFixed(1)}%)
				</dd>
			</dl>
			<nav aria-label="関連リンク" class="mt-5 flex flex-wrap gap-x-5 gap-y-2 text-caption">
				<a class="text-apple-blue underline" href="https://www.kochi-u.ac.jp/education-support/courses/syllabus/" target="_blank" rel="noopener noreferrer">大学公式シラバス</a>
				<a class="text-apple-blue underline" href="https://github.com/P4suta/gyakubiki-syllabus" target="_blank" rel="noopener noreferrer">GitHub source</a>
				<a class="text-apple-blue underline" href="https://github.com/P4suta/gyakubiki-syllabus/blob/main/LICENSE" target="_blank" rel="noopener noreferrer">AGPL-3.0 license</a>
			</nav>
		</section>
	</dialog>
{/if}
