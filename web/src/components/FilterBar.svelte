<script lang="ts">
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
</script>

<div class="glass-nav sticky top-0 z-50 border-b border-overlay-light px-6 py-3">
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
				placeholder="科目名・教員名・コード..."
				class="bg-overlay-subtle rounded-lg pl-8 pr-3 py-1.5 text-caption text-apple-text outline-none w-52 placeholder:text-apple-text/30 transition-all duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30 focus:shadow-sm"
			/>
		</div>

		<span class="text-caption text-apple-text/40 ml-auto tabular-nums tracking-tight">
			{displayCount}科目表示中 / 全{totalCount}件
		</span>
	</div>
</div>
