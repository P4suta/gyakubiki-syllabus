<script lang="ts">
import { fade, fly } from 'svelte/transition'

interface Props {
	semesters: string[]
	departments: string[]
	campuses: string[]
	semester: string
	department: string
	campus: string
	searchText: string
	displayCount: number
	totalCount: number
}

let {
	semesters,
	departments,
	campuses,
	semester = $bindable(),
	department = $bindable(),
	campus = $bindable(),
	searchText = $bindable(),
	displayCount,
	totalCount,
}: Props = $props()

let mobileFilterOpen = $state(false)
let hasActiveFilters = $derived(department !== 'all' || campus !== 'all' || searchText !== '')

function resetFilters() {
	semester = 'all'
	department = 'all'
	campus = 'all'
	searchText = ''
}
</script>

<!-- ==================== Mobile: Compact bar ==================== -->
<div class="glass-nav sticky top-0 z-50 border-b border-overlay-light px-3 py-2 sm:hidden">
	<div class="flex items-center gap-2">
		<h1 class="text-body font-semibold text-apple-text whitespace-nowrap tracking-tight">時間割</h1>

		<button
			class="bg-overlay-muted rounded-full px-3 py-1 text-caption font-medium text-apple-text truncate max-w-[140px]"
			onclick={() => { mobileFilterOpen = true }}
		>
			{semester === 'all' ? '全学期' : semester}
		</button>

		<div class="ml-auto flex items-center gap-2">
			<span class="text-micro text-apple-text/40 tabular-nums">{displayCount}件</span>
			<button
				class="relative w-9 h-9 rounded-full bg-overlay-subtle flex items-center justify-center active:bg-overlay-medium"
				onclick={() => { mobileFilterOpen = true }}
				aria-label="フィルターを開く"
			>
				<svg class="w-4 h-4 text-apple-text/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
				</svg>
				{#if hasActiveFilters}
					<span class="absolute top-1 right-1 w-2 h-2 rounded-full bg-apple-blue"></span>
				{/if}
			</button>
		</div>
	</div>
</div>

<!-- ==================== Mobile: Bottom sheet ==================== -->
{#if mobileFilterOpen}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[60] bg-overlay-backdrop sm:hidden"
		onclick={() => { mobileFilterOpen = false }}
		onkeydown={() => {}}
		transition:fade={{ duration: 150 }}
	></div>

	<div
		class="fixed inset-x-0 bottom-0 z-[61] sm:hidden bg-surface-primary rounded-t-2xl shadow-modal max-h-[75dvh] overflow-y-auto safe-bottom"
		transition:fly={{ y: 300, duration: 250 }}
	>
		<!-- Drag handle -->
		<div class="flex justify-center pt-2 pb-1">
			<div class="w-9 h-1 rounded-full bg-overlay-strong"></div>
		</div>

		<!-- Sheet header -->
		<div class="flex items-center justify-between px-4 pb-3 border-b border-overlay-subtle">
			<h2 class="text-body font-semibold text-apple-text">フィルター</h2>
			<button class="text-caption text-apple-blue font-medium" onclick={resetFilters}>リセット</button>
		</div>

		<div class="px-4 py-4 space-y-5">
			<!-- Search -->
			<div>
				<label class="text-micro font-medium text-apple-text/50 mb-1.5 block" for="mobile-search">検索</label>
				<div class="relative">
					<svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text/30 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
					</svg>
					<input
						id="mobile-search"
						type="text"
						bind:value={searchText}
						placeholder="科目名・教員名で検索"
						class="w-full bg-overlay-subtle rounded-xl pl-10 pr-10 py-2.5 text-body text-apple-text outline-none placeholder:text-apple-text/30 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30"
					/>
					{#if searchText}
						<button
							class="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 rounded-full bg-apple-text/20 flex items-center justify-center"
							onclick={() => { searchText = '' }}
							aria-label="検索をクリア"
						>
							<svg class="w-3 h-3 text-surface-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
								<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
							</svg>
						</button>
					{/if}
				</div>
			</div>

			<!-- Semester: horizontal scroll -->
			<div>
				<span class="text-micro font-medium text-apple-text/50 mb-1.5 block">学期</span>
				<div class="flex gap-1.5 overflow-x-auto pb-1 -mx-4 px-4 snap-x hide-scrollbar">
					<button
						class="snap-start shrink-0 px-3 py-2 rounded-xl text-caption font-medium min-h-[44px] transition-colors
							{semester === 'all'
								? 'bg-apple-blue text-white'
								: 'bg-overlay-subtle text-apple-text/60 active:bg-overlay-medium'}"
						onclick={() => { semester = 'all' }}
					>全て</button>
					{#each semesters as s}
						<button
							class="snap-start shrink-0 px-3 py-2 rounded-xl text-caption font-medium min-h-[44px] transition-colors
								{semester === s
									? 'bg-apple-blue text-white'
									: 'bg-overlay-subtle text-apple-text/60 active:bg-overlay-medium'}"
							onclick={() => { semester = s }}
						>{s}</button>
					{/each}
				</div>
			</div>

			<!-- Campus -->
			<div>
				<label class="text-micro font-medium text-apple-text/50 mb-1.5 block" for="mobile-campus">キャンパス</label>
				<div class="relative">
					<select
						id="mobile-campus"
						bind:value={campus}
						class="w-full appearance-none bg-overlay-subtle rounded-xl px-3 py-2.5 pr-8 text-body text-apple-text min-h-[44px] outline-none cursor-pointer focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30"
					>
						<option value="all">全キャンパス</option>
						{#each campuses as c}
							<option value={c}>{c}</option>
						{/each}
					</select>
					<svg class="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text/40 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
					</svg>
				</div>
			</div>

			<!-- Department -->
			<div>
				<label class="text-micro font-medium text-apple-text/50 mb-1.5 block" for="mobile-dept">開講部署</label>
				<div class="relative">
					<select
						id="mobile-dept"
						bind:value={department}
						class="w-full appearance-none bg-overlay-subtle rounded-xl px-3 py-2.5 pr-8 text-body text-apple-text min-h-[44px] outline-none cursor-pointer focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30"
					>
						<option value="all">全部署</option>
						{#each departments as d}
							<option value={d}>{d}</option>
						{/each}
					</select>
					<svg class="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text/40 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
					</svg>
				</div>
			</div>
		</div>

		<!-- Apply button -->
		<div class="px-4 pb-6 pt-2">
			<button
				class="w-full py-3 bg-apple-blue text-white text-body font-medium rounded-full min-h-[44px] active:bg-apple-blue-hover transition-colors cursor-pointer"
				onclick={() => { mobileFilterOpen = false }}
			>
				{displayCount}科目を表示
			</button>
		</div>
	</div>
{/if}

<!-- ==================== Desktop: Horizontal layout ==================== -->
<div class="glass-nav sticky top-0 z-50 border-b border-overlay-light px-6 py-3 hidden sm:block">
	<div class="flex items-center gap-4 flex-wrap">
		<h1 class="text-lg font-semibold text-apple-text whitespace-nowrap tracking-tight">時間割</h1>

		<!-- Semester segmented control -->
		<div class="flex bg-overlay-muted rounded-full p-0.5">
			<button
				class="px-4 py-1.5 text-caption font-medium rounded-full transition-all duration-200
					{semester === 'all'
						? 'bg-surface-primary text-apple-text font-semibold shadow-sm'
						: 'text-apple-text/50 hover:text-apple-text/70'}"
				onclick={() => { semester = 'all' }}
			>
				全て
			</button>
			{#each semesters as s}
				<button
					class="px-4 py-1.5 text-caption font-medium rounded-full transition-all duration-200
						{semester === s
							? 'bg-surface-primary text-apple-text font-semibold shadow-sm'
							: 'text-apple-text/50 hover:text-apple-text/70'}"
					onclick={() => { semester = s }}
				>
					{s}
				</button>
			{/each}
		</div>

		<!-- Campus filter -->
		<div class="relative">
			<select
				bind:value={campus}
				class="appearance-none bg-overlay-subtle hover:bg-overlay-medium rounded-lg px-3 py-1.5 pr-7 text-caption text-apple-text outline-none transition-colors duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30 focus:shadow-sm max-w-40 cursor-pointer"
			>
				<option value="all">全キャンパス</option>
				{#each campuses as c}
					<option value={c}>{c}</option>
				{/each}
			</select>
			<svg class="absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-apple-text/40 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
			</svg>
		</div>

		<!-- Department filter -->
		<div class="relative">
			<select
				bind:value={department}
				class="appearance-none bg-overlay-subtle hover:bg-overlay-medium rounded-lg px-3 py-1.5 pr-7 text-caption text-apple-text outline-none transition-colors duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30 focus:shadow-sm max-w-48 cursor-pointer"
			>
				<option value="all">全部署</option>
				{#each departments as d}
					<option value={d}>{d}</option>
				{/each}
			</select>
			<svg class="absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-apple-text/40 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
			</svg>
		</div>

		<!-- Search -->
		<div class="relative">
			<svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-apple-text/30 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
			</svg>
			<input
				type="text"
				bind:value={searchText}
				placeholder="科目名・教員名で検索"
				class="bg-overlay-subtle rounded-lg pl-8 pr-8 py-1.5 text-caption text-apple-text outline-none w-64 placeholder:text-apple-text/30 transition-all duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30 focus:shadow-sm"
			/>
			{#if searchText}
				<button
					class="absolute right-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full bg-apple-text/20 flex items-center justify-center hover:bg-apple-text/30 transition-colors cursor-pointer"
					onclick={() => { searchText = '' }}
					aria-label="検索をクリア"
				>
					<svg class="w-2.5 h-2.5 text-surface-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
						<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			{/if}
		</div>

		<span class="text-caption text-apple-text/40 ml-auto tabular-nums tracking-tight">
			{displayCount}科目表示中 / 全{totalCount}件
		</span>
	</div>
</div>
