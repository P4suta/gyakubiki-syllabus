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

const gridCols = `56px repeat(${DAYS.length}, 1fr)`
const minWidth = `${DAYS.length * 120 + 56}px`
</script>

<div class="overflow-auto flex-1 bg-surface-page">
	<div class="grid gap-[2px] bg-surface-page" style="grid-template-columns: {gridCols}; min-width: {minWidth};">
		<!-- Corner cell -->
		<div class="sticky top-0 left-0 z-30 bg-surface-page"></div>

		<!-- Day headers -->
		{#each DAYS as day}
			<div
				class="sticky top-0 z-20 text-center py-3 font-semibold text-caption text-apple-text/60 tracking-tight bg-surface-page/80 backdrop-blur-sm"
			>
				{day}
			</div>
		{/each}

		<!-- Grid rows -->
		{#each PERIODS as period}
			<!-- Period label -->
			<div class="sticky left-0 z-10 bg-surface-page">
				<div class="sticky top-10 px-1 py-2 text-center font-medium text-caption text-apple-text/40">
					{period}限
				</div>
			</div>
			{#each DAYS as day}
				<TimetableCell
					courses={grid.get(`${day}-${period}`) ?? []}
					{onselect}
				/>
			{/each}
		{/each}
	</div>
</div>
