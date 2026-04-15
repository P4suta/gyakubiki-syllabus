<script lang="ts">
import { fade, fly } from 'svelte/transition'
import type { CourseV2, Dictionaries } from '../types/course'

interface Props {
	course: CourseV2
	dicts: Dictionaries
	onclose: () => void
}

let { course, dicts, onclose }: Props = $props()

const fields: [string, string | undefined | null][] = $derived([
	['授業コード', course.cd],
	['時間割', course.raw],
	['担当教員', course.prof],
	['開講時期', dicts.kaikojiki[course.ki]],
	['講義区分', dicts.kubun[course.kbn]],
	['校地', dicts.campuses[course.campus]],
	['開講責任部署', dicts.departments[course.dept]],
	['学則科目', course.gaku ?? course.nm],
	['対象学科/年次', course.gakka],
	['必須/選択', course.nen],
	['科目分類', course.bunrui],
	['科目分野', course.bunya],
])

function handleKeydown(e: KeyboardEvent) {
	if (e.key === 'Escape') onclose()
}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 bg-overlay-backdrop backdrop-blur-[6px] flex items-end sm:items-center justify-center sm:p-5 z-[200]"
	onclick={onclose}
	onkeydown={(e) => { if (e.key === 'Escape') onclose() }}
	transition:fade={{ duration: 200 }}
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="bg-surface-primary w-full sm:max-w-lg rounded-t-2xl sm:rounded-2xl max-h-[90dvh] sm:max-h-[80vh] overflow-hidden shadow-modal"
		onclick={(e) => e.stopPropagation()}
		onkeydown={() => {}}
		transition:fly={{ y: 20, duration: 300, opacity: 0 }}
	>
		<!-- Drag handle (mobile) -->
		<div class="flex justify-center pt-2 pb-0 sm:hidden">
			<div class="w-9 h-1 rounded-full bg-overlay-strong"></div>
		</div>

		<!-- Header -->
		<div class="px-4 pt-4 pb-3 sm:px-7 sm:pt-7 sm:pb-4">
			<div class="flex justify-between items-start gap-3">
				<div class="min-w-0">
					<h2 class="text-xl font-bold text-apple-text leading-snug tracking-tight">
						{course.nm}
					</h2>
					{#if course.sub}
						<p class="text-sub text-apple-text/50 mt-1 tracking-tight">{course.sub}</p>
					{/if}
				</div>
				<button
					class="shrink-0 w-10 h-10 sm:w-8 sm:h-8 rounded-full bg-overlay-light flex items-center justify-center active:bg-overlay-strong sm:hover:bg-overlay-strong transition-colors duration-200 cursor-pointer"
					onclick={onclose}
					aria-label="閉じる"
				>
					<svg class="w-3.5 h-3.5 text-apple-text/60" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>
		</div>

		<!-- Details -->
		<div class="px-4 pb-6 sm:px-7 sm:pb-7 overflow-auto max-h-[calc(90dvh-120px)] sm:max-h-[calc(80vh-120px)]">
			{#each fields as [label, value]}
				{#if value}
					<div class="py-3 border-b border-overlay-subtle last:border-0">
						<span class="text-micro sm:text-caption text-apple-text/40 block sm:inline sm:min-w-28 sm:shrink-0 mb-0.5 sm:mb-0">{label}</span>
						<span class="text-body text-apple-text leading-relaxed tracking-tight block sm:inline">{value}</span>
					</div>
				{/if}
			{/each}
		</div>
	</div>
</div>
