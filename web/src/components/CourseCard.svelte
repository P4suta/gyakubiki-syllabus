<script lang="ts">
import { getColor } from '../lib/colors'
import type { CourseV2 } from '../types/course'

interface Props {
	course: CourseV2
	onclick: () => void
}

let { course, onclick }: Props = $props()
let color = $derived(getColor(course.cd))
</script>

<button
	class="w-full text-left rounded-lg p-3 sm:p-1.5 mb-1 sm:mb-0.5 cursor-pointer transition-transform active:bg-black/5 sm:hover:scale-[1.02] sm:hover:shadow-md border-l-3 min-h-[44px] sm:min-h-0"
	style="background: {color.bg}; border-left-color: {color.border};"
	{onclick}
>
	<div class="font-semibold text-caption sm:text-micro leading-snug mb-0.5 line-clamp-2 text-apple-text">
		{course.nm}
	</div>
	<div class="text-micro sm:text-fine text-apple-text/50">
		{course.prof?.split(',')[0]?.trim() ?? ''}
	</div>
</button>
