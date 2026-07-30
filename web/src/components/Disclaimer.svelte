<script lang="ts">
import { onMount } from 'svelte'

const STORAGE_KEY = 'unofficialNoticeSeenV1'
let visible = $state(false)

onMount(() => {
	try {
		visible = localStorage.getItem(STORAGE_KEY) !== '1'
	} catch {
		visible = true
	}
})

function dismiss() {
	visible = false
	try {
		localStorage.setItem(STORAGE_KEY, '1')
	} catch {
		// Storage is optional; the notice still dismisses for this page view.
	}
}
</script>

{#if visible}
	<aside
		data-unofficial-banner
		class="mx-3 mt-2 rounded-xl border border-apple-blue/20 bg-apple-blue/5 px-4 py-3 sm:mx-6"
		aria-labelledby="unofficial-notice-title"
	>
		<div class="flex items-start gap-3">
			<div class="min-w-0 grow">
				<h2 id="unofficial-notice-title" class="text-caption font-semibold text-apple-text">
					非公式のシラバス検索ツールです
				</h2>
				<p class="mt-0.5 text-micro leading-relaxed text-apple-text-secondary sm:text-caption">
					高知大学による承認・推奨を受けたものではありません。履修判断は必ず
					<a
						href="https://www.kochi-u.ac.jp/education-support/courses/syllabus/"
						class="font-medium text-apple-text underline"
						target="_blank"
						rel="noopener noreferrer"
					>大学公式シラバス</a>
					で確認してください。
				</p>
			</div>
			<button
				onclick={dismiss}
				class="shrink-0 rounded-full bg-overlay-subtle px-3 py-1.5 text-micro text-apple-text cursor-pointer"
			>
				確認しました
			</button>
		</div>
	</aside>
{/if}
