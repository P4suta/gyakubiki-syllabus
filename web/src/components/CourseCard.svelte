<script lang="ts">
import { getColor } from '../lib/colors'
import type { Course } from '../types/course'

interface Props {
	course: Course
	onclick: () => void
}

let { course, onclick }: Props = $props()
let color = $derived(getColor(course.kogiCd))
</script>

<button
	class="w-full text-left rounded p-1.5 mb-0.5 cursor-pointer transition-transform hover:scale-[1.02] hover:shadow-md border-l-3"
	style="background: {color.bg}; border-left-color: {color.border};"
	{onclick}
>
	<div
		class="font-semibold text-[11px] leading-snug mb-0.5 line-clamp-2"
		style="color: {color.text};"
	>
		{course.kogiNm}
	</div>
	<div class="text-gray-500 text-[10px]">
		{course.tantoKyoin?.split(',')[0]?.trim() ?? ''}
	</div>
</button>
