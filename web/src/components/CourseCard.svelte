<script lang="ts">
import { getColor } from '../lib/colors'
import type { CourseV2 } from '../types/course'

interface Props {
	course: CourseV2
	onclick: () => void
}

let { course, onclick }: Props = $props()
let color = $derived(getColor(course.dept, course.cd))
</script>

<button
	class="w-full text-left rounded-lg p-2 mb-1 cursor-pointer transition-all duration-300 ease-spring hover:-translate-y-px hover:shadow-card-hover active:scale-[0.97] active:shadow-none"
	style="background: {color.bg};"
	{onclick}
>
	<div class="h-0.5 rounded-full mb-1.5 -mx-0.5" style="background: {color.border};"></div>
	<div class="font-semibold text-caption leading-tight tracking-tight mb-0.5 line-clamp-2 text-apple-text">
		{course.nm}
	</div>
	<div class="text-micro text-apple-text/50 tracking-tight">
		{course.prof?.split(',')[0]?.trim() ?? ''}
	</div>
</button>
