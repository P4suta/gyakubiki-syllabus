<script lang="ts">
import type { GridKey } from '../lib/grid'
import { DAYS, PERIODS } from '../lib/grid'
import type { Course } from '../types/course'
import TimetableCell from './TimetableCell.svelte'

interface Props {
	grid: Map<GridKey, Course[]>
	onselect: (course: Course) => void
}

let { grid, onselect }: Props = $props()
</script>

<div class="p-3 overflow-x-auto">
	<div class="grid grid-cols-[48px_repeat(6,1fr)] gap-0.5 min-w-[700px]">
		<!-- Header row -->
		<div></div>
		{#each DAYS as day}
			<div
				class="text-center py-2 font-bold text-sm text-gray-700 rounded-t-lg
					{day === '土' ? 'bg-amber-100' : 'bg-white'}"
			>
				{day}
			</div>
		{/each}

		<!-- Grid rows -->
		{#each PERIODS as period}
			<div class="flex items-start justify-center pt-3 font-semibold text-xs text-gray-500">
				{period}限
			</div>
			{#each DAYS as day}
				<TimetableCell
					courses={grid.get(`${day}-${period}`) ?? []}
					isSaturday={day === '土'}
					{onselect}
				/>
			{/each}
		{/each}
	</div>
</div>
