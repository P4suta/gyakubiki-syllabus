<script lang="ts">
import { fade, scale } from 'svelte/transition'

let accepted = $state(false)
</script>

{#if !accepted}
<div
	class="fixed inset-0 z-[100] flex items-center justify-center bg-overlay-backdrop backdrop-blur-md p-4"
	transition:fade={{ duration: 200 }}
>
	<div
		class="bg-surface-primary rounded-2xl shadow-modal max-w-md w-full max-h-[90dvh] overflow-y-auto p-5 sm:p-8"
		transition:scale={{ start: 0.95, duration: 300 }}
	>
		<h2 class="text-xl font-bold text-apple-text tracking-tight mb-5">ご利用にあたって</h2>

		<div class="text-body text-apple-text/70 leading-relaxed space-y-3 mb-8 tracking-tight">
			<p>
				本ツールは個人が作成した非公式のものであり、高知大学とは一切の関係がなく、同大学による承認又は推奨を受けたものではありません。
			</p>
			<p>
				表示されるデータは、<a href="https://www.kochi-u.ac.jp/education-support/courses/syllabus/" class="text-apple-blue hover:underline" target="_blank" rel="noopener noreferrer">同大学が一般に公開しているシラバス情報</a>のみに基づいており、非公開情報は一切使用していません。
			</p>
			<p>
				本ツールは現状有姿（AS IS）で提供され、明示又は黙示を問わず、正確性、完全性、最新性、特定目的への適合性その他一切の保証をいたしません。
			</p>
			<p>
				本ツールの利用又は利用不能により生じた直接的又は間接的な損害について、作成者は一切の責任を負いません。
				履修登録その他の判断は、必ず大学公式の情報に基づいて行ってください。
			</p>
		</div>

		<button
			onclick={() => { accepted = true }}
			class="w-full py-3 px-8 bg-apple-blue text-white text-cta font-normal rounded-full hover:bg-apple-blue-hover transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-apple-blue/40 focus:ring-offset-2"
		>
			上記を確認の上、利用する
		</button>
	</div>
</div>
{/if}
