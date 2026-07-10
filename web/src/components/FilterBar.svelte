<script lang="ts">
import IconClose from '~icons/ic/round-close'
import IconExpandMore from '~icons/ic/round-expand-more'
import IconFilterList from '~icons/ic/round-filter-list'
import IconSearch from '~icons/ic/round-search'
import { haptic } from '../lib/gestures'
import BottomSheet from './BottomSheet.svelte'

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
	generatedAt: string
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
	generatedAt,
}: Props = $props()

let mobileFilterOpen = $state(false)
let hasActiveFilters = $derived(department !== 'all' || campus !== 'all' || searchText !== '')

// Format ISO-8601 as YYYY-MM-DD (local TZ) for the data's last-updated label.
let generatedAtLabel = $derived.by(() => {
	if (!generatedAt) return ''
	const d = new Date(generatedAt)
	if (Number.isNaN(d.getTime())) return ''
	const y = d.getFullYear()
	const m = String(d.getMonth() + 1).padStart(2, '0')
	const dd = String(d.getDate()).padStart(2, '0')
	return `${y}-${m}-${dd}`
})

function resetFilters() {
	semester = 'all'
	department = 'all'
	campus = 'all'
	searchText = ''
}
</script>

<!-- Shared bits: the "全て + each value" option list and the dropdown chevron,
     each rendered by both the mobile sheet and the desktop bar below. -->
{#snippet selectOptions(allLabel: string, items: string[])}
	<option value="all">{allLabel}</option>
	{#each items as item}
		<option value={item}>{item}</option>
	{/each}
{/snippet}

{#snippet chevron(className: string)}
	<IconExpandMore class={className} />
{/snippet}

<!-- Mobile: compact bar -->
<div class="glass-nav sticky top-0 z-nav border-b border-overlay-subtle px-3 py-2 sm:hidden">
	<div class="flex items-center gap-2">
		<h1 class="text-body font-semibold text-apple-text whitespace-nowrap tracking-tight">時間割</h1>
		{#if generatedAtLabel}
			<span class="bg-overlay-subtle text-apple-text-secondary rounded-full px-2 py-0.5 text-micro whitespace-nowrap">
				最終更新: <span class="tabular-nums">{generatedAtLabel}</span>
			</span>
		{/if}

		<button
			class="bg-overlay-muted rounded-full px-3 py-1 text-caption font-medium text-apple-text truncate max-w-36"
			onclick={() => { mobileFilterOpen = true }}
		>
			{semester === 'all' ? '全学期' : semester}
		</button>

		<div class="ml-auto flex items-center gap-2">
			<span class="bg-overlay-subtle text-apple-text-secondary rounded-full px-2 py-0.5 text-micro tabular-nums whitespace-nowrap">{displayCount}件</span>
			<button
				class="relative w-9 h-9 rounded-full bg-overlay-subtle flex items-center justify-center active:bg-overlay-medium"
				onclick={() => { mobileFilterOpen = true }}
				aria-label="フィルターを開く"
			>
				<IconFilterList class="w-4 h-4 text-apple-text-secondary" />
				{#if hasActiveFilters}
					<span class="absolute top-1 right-1 w-2 h-2 rounded-full bg-apple-blue"></span>
				{/if}
			</button>
		</div>
	</div>
</div>

<!-- Mobile: bottom sheet -->
{#if mobileFilterOpen}
	<BottomSheet onclose={() => { mobileFilterOpen = false }} ariaLabel="フィルター">
		{#snippet header()}
			<div class="flex items-center justify-between px-4 pt-1 pb-3 border-b border-overlay-subtle">
				<h2 class="text-body font-semibold text-apple-text">フィルター</h2>
				<button class="text-caption text-apple-blue font-medium" onclick={resetFilters}>リセット</button>
			</div>
		{/snippet}

		<div class="px-4 py-4 space-y-5">
			<div>
				<label class="text-micro font-medium text-apple-text-tertiary mb-1.5 block" for="mobile-search">検索</label>
				<div class="relative">
					<IconSearch class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text-tertiary pointer-events-none" />
					<input
						id="mobile-search"
						type="text"
						bind:value={searchText}
						placeholder="科目名・教員・キーワードで検索"
						class="w-full bg-overlay-subtle rounded-xl pl-10 pr-10 py-2.5 text-body text-apple-text outline-none placeholder:text-apple-text-tertiary focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/40"
					/>
					{#if searchText}
						<button
							class="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 rounded-full bg-overlay-medium flex items-center justify-center hover:bg-overlay-strong transition-colors"
							onclick={() => { searchText = '' }}
							aria-label="検索をクリア"
						>
							<IconClose class="w-3.5 h-3.5 text-apple-text-secondary" />
						</button>
					{/if}
				</div>
			</div>

			<div>
				<span class="text-micro font-medium text-apple-text-tertiary mb-1.5 block">学期</span>
				<div class="flex gap-1.5 overflow-x-auto pb-1 -mx-4 px-4 snap-x hide-scrollbar">
					<button
						class="snap-start shrink-0 px-3 py-2 rounded-full text-caption font-medium min-h-tap transition-colors
							{semester === 'all'
								? 'bg-apple-blue text-on-accent'
								: 'bg-overlay-subtle text-apple-text-secondary active:bg-overlay-medium'}"
						onclick={() => { semester = 'all' }}
					>全て</button>
					{#each semesters as s}
						<button
							class="snap-start shrink-0 px-3 py-2 rounded-full text-caption font-medium min-h-tap transition-colors
								{semester === s
									? 'bg-apple-blue text-on-accent'
									: 'bg-overlay-subtle text-apple-text-secondary active:bg-overlay-medium'}"
							onclick={() => { semester = s }}
						>{s}</button>
					{/each}
				</div>
			</div>

			<div>
				<label class="text-micro font-medium text-apple-text-tertiary mb-1.5 block" for="mobile-campus">キャンパス</label>
				<div class="relative">
					<select
						id="mobile-campus"
						bind:value={campus}
						class="w-full appearance-none bg-overlay-subtle rounded-xl px-3 py-2.5 pr-8 text-body text-apple-text min-h-tap outline-none cursor-pointer focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/40"
					>
						{@render selectOptions('全キャンパス', campuses)}
					</select>
					{@render chevron('absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text-tertiary pointer-events-none')}
				</div>
			</div>

			<div>
				<label class="text-micro font-medium text-apple-text-tertiary mb-1.5 block" for="mobile-dept">開講部署</label>
				<div class="relative">
					<select
						id="mobile-dept"
						bind:value={department}
						class="w-full appearance-none bg-overlay-subtle rounded-xl px-3 py-2.5 pr-8 text-body text-apple-text min-h-tap outline-none cursor-pointer focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/40"
					>
						{@render selectOptions('全部署', departments)}
					</select>
					{@render chevron('absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text-tertiary pointer-events-none')}
				</div>
			</div>
		</div>

		{#snippet footer(close)}
			<div class="px-4 pb-6 pt-2">
				<button
					class="w-full py-3 bg-apple-blue text-on-accent text-body font-medium rounded-full min-h-tap active:bg-apple-blue-hover transition-colors cursor-pointer"
					onclick={() => { haptic('light'); close() }}
				>
					{displayCount}科目を表示
				</button>
			</div>
		{/snippet}
	</BottomSheet>
{/if}

<!-- Desktop: horizontal layout -->
<div class="glass-nav sticky top-0 z-nav border-b border-overlay-subtle px-6 py-3 hidden sm:block">
	<div class="flex items-center gap-4 flex-wrap">
		<h1 class="text-headline font-semibold text-apple-text whitespace-nowrap tracking-tight">時間割</h1>
		{#if generatedAtLabel}
			<span class="bg-overlay-subtle text-apple-text-secondary rounded-full px-2.5 py-0.5 text-caption whitespace-nowrap">
				最終更新: <span class="tabular-nums">{generatedAtLabel}</span>
			</span>
		{/if}

		<div class="flex bg-overlay-muted rounded-full p-0.5">
			<button
				class="px-4 py-1.5 text-caption font-medium rounded-full transition-all duration-200
					{semester === 'all'
						? 'bg-surface-primary text-apple-text font-semibold shadow-sm'
						: 'text-apple-text-tertiary hover:text-apple-text-secondary'}"
				onclick={() => { semester = 'all' }}
			>
				全て
			</button>
			{#each semesters as s}
				<button
					class="px-4 py-1.5 text-caption font-medium rounded-full transition-all duration-200
						{semester === s
							? 'bg-surface-primary text-apple-text font-semibold shadow-sm'
							: 'text-apple-text-tertiary hover:text-apple-text-secondary'}"
					onclick={() => { semester = s }}
				>
					{s}
				</button>
			{/each}
		</div>

		<div class="relative">
			<select
				bind:value={campus}
				aria-label="キャンパスで絞り込み"
				class="appearance-none bg-overlay-subtle hover:bg-overlay-medium rounded-xl px-3 py-1.5 pr-7 text-caption text-apple-text outline-none transition-colors duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/40 focus:shadow-sm max-w-40 cursor-pointer"
			>
				{@render selectOptions('全キャンパス', campuses)}
			</select>
			{@render chevron('absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-apple-text-tertiary pointer-events-none')}
		</div>

		<div class="relative">
			<select
				bind:value={department}
				aria-label="開講部署で絞り込み"
				class="appearance-none bg-overlay-subtle hover:bg-overlay-medium rounded-xl px-3 py-1.5 pr-7 text-caption text-apple-text outline-none transition-colors duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/40 focus:shadow-sm max-w-48 cursor-pointer"
			>
				{@render selectOptions('全部署', departments)}
			</select>
			{@render chevron('absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-apple-text-tertiary pointer-events-none')}
		</div>

		<!-- Count as a chip, matching the「最終更新」badge beside it. The shown figure
		     is the salient number, so it carries the weight; the「/ 全…件」total stays
		     muted. Text kept contiguous for the E2E counter anchor. -->
		<span class="ml-auto inline-block bg-overlay-subtle rounded-full px-2.5 py-0.5 text-caption text-apple-text-secondary tabular-nums tracking-tight whitespace-nowrap"><span class="font-semibold text-apple-text">{displayCount}</span>科目表示中 / 全{totalCount}件</span>
	</div>
</div>
