<script lang="ts">
import type { Course } from '../types/course'

interface Props {
	course: Course
	onclose: () => void
}

let { course, onclose }: Props = $props()

const fields: [string, string | undefined | null][] = $derived([
	['授業コード', course.kogiCd],
	['時間割', course.jikanwariRaw],
	['担当教員', course.tantoKyoin],
	['開講時期', course.kogiKaikojikiNm],
	['講義区分', course.kogiKubunNm],
	['校地', course.kochiNm],
	['開講責任部署', course.sekininBushoNm],
	['学則科目', course.gakusokuKamokuNm],
	['対象学科/年次', course.taishoGakka],
	['必須/選択', course.taishoNenji],
	['科目分類', course.kamokuBunrui],
	['科目分野', course.kamokuBunya],
])

function handleKeydown(e: KeyboardEvent) {
	if (e.key === 'Escape') onclose()
}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 bg-black/40 flex items-center justify-center z-[200] p-5"
	onclick={onclose}
	onkeydown={(e) => { if (e.key === 'Escape') onclose() }}
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="bg-white rounded-2xl p-7 max-w-lg w-full max-h-[80vh] overflow-auto shadow-2xl"
		onclick={(e) => e.stopPropagation()}
		onkeydown={() => {}}
	>
		<div class="flex justify-between items-start mb-4">
			<div>
				<h2 class="text-lg font-bold text-gray-900 leading-snug">
					{course.kogiNm}
				</h2>
				{#if course.fukudai}
					<p class="text-sm text-gray-500 mt-1">{course.fukudai}</p>
				{/if}
			</div>
			<button
				class="text-gray-400 hover:text-gray-600 text-xl px-1 bg-transparent border-none cursor-pointer"
				onclick={onclose}
			>
				&times;
			</button>
		</div>

		{#each fields as [label, value]}
			{#if value}
				<div class="flex py-2 border-b border-gray-100 gap-3">
					<span class="text-xs text-gray-400 min-w-28 shrink-0">{label}</span>
					<span class="text-sm text-gray-700 leading-relaxed">{value}</span>
				</div>
			{/if}
		{/each}
	</div>
</div>
