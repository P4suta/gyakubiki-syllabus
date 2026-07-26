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
		class="fixed inset-x-3 top-3 z-nav mx-auto max-w-2xl rounded-2xl bg-surface-primary p-4 shadow-modal"
		aria-labelledby="unofficial-notice-title"
	>
		<h2 id="unofficial-notice-title" class="text-cta font-semibold text-apple-text">
			非公式のシラバス検索ツールです
		</h2>
		<p class="mt-1 text-caption leading-relaxed text-apple-text-secondary">
			高知大学による承認・推奨を受けたものではありません。履修登録などの判断は、必ず
			<a
				href="https://www.kochi-u.ac.jp/education-support/courses/syllabus/"
				class="text-apple-blue underline"
				target="_blank"
				rel="noopener noreferrer"
			>大学公式シラバス</a>
			で確認してください。
		</p>
		<button
			onclick={dismiss}
			class="mt-3 rounded-full bg-apple-blue px-4 py-2 text-caption text-on-accent cursor-pointer"
		>
			確認しました
		</button>
	</aside>
{/if}
