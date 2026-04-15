<script lang="ts">
import type { GridKey } from '../lib/grid'
import { DAYS, PERIODS } from '../lib/grid'
import type { CourseV2 } from '../types/course'
import TimetableCell from './TimetableCell.svelte'

interface Props {
	grid: Map<GridKey, CourseV2[]>
	onselect: (course: CourseV2) => void
}

let { grid, onselect }: Props = $props()
</script>

<div class="overflow-auto flex-1 bg-gray-50">
	<div class="grid grid-cols-[48px_repeat(6,1fr)] min-w-[700px]">
		<!-- Corner cell: sticks both directions, highest z -->
		<div class="sticky top-0 left-0 z-30 bg-gray-50 border-b border-r border-gray-200"></div>

		<!-- Day headers: stick to top -->
		{#each DAYS as day}
			<div
				class="sticky top-0 z-20 text-center py-2 font-bold text-sm text-gray-700 border-b border-gray-200
					{day === '土' ? 'bg-amber-100' : 'bg-white'}"
			>
				{day}
			</div>
		{/each}

		<!-- Grid rows -->
		{#each PERIODS as period}
			<!-- Period label: sticks to left, text sticks to top within cell -->
			<div class="sticky left-0 z-10 bg-gray-50 border-r border-gray-200">
				<div class="sticky top-8 px-1 py-2 text-center font-semibold text-xs text-gray-500">
					{period}限
				</div>
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
