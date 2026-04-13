<script lang="ts">
interface Props {
	semesters: string[]
	departments: string[]
	semester: string
	department: string
	searchText: string
	displayCount: number
	totalCount: number
}

let {
	semesters,
	departments,
	semester = $bindable(),
	department = $bindable(),
	searchText = $bindable(),
	displayCount,
	totalCount,
}: Props = $props()
</script>

<div class="bg-white border-b border-gray-200 px-5 py-3 sticky top-0 z-50">
	<div class="flex items-center gap-4 flex-wrap">
		<h1 class="text-base font-bold text-gray-900 whitespace-nowrap">時間割</h1>

		<!-- Semester tabs -->
		<div class="flex gap-1 bg-gray-100 rounded-lg p-0.5">
			<button
				class="px-3.5 py-1 text-xs font-medium rounded-md transition-all
					{semester === 'all' ? 'bg-blue-600 text-white' : 'text-gray-500 hover:text-gray-700'}"
				onclick={() => { semester = 'all' }}
			>
				全て
			</button>
			{#each semesters as s}
				<button
					class="px-3.5 py-1 text-xs font-medium rounded-md transition-all
						{semester === s ? 'bg-blue-600 text-white' : 'text-gray-500 hover:text-gray-700'}"
					onclick={() => { semester = s }}
				>
					{s}
				</button>
			{/each}
		</div>

		<!-- Department filter -->
		<select
			bind:value={department}
			class="px-2.5 py-1 text-xs border border-gray-300 rounded-md bg-white text-gray-700 outline-none max-w-48"
		>
			<option value="all">全部署</option>
			{#each departments as d}
				<option value={d}>{d}</option>
			{/each}
		</select>

		<!-- Search -->
		<input
			type="text"
			bind:value={searchText}
			placeholder="科目名・教員名・コード..."
			class="px-3 py-1 text-xs border border-gray-300 rounded-md outline-none w-44 text-gray-700 placeholder:text-gray-400 focus:border-blue-400"
		/>

		<span class="text-xs text-gray-400 ml-auto">
			{displayCount}科目表示中 / 全{totalCount}件
		</span>
	</div>
</div>
