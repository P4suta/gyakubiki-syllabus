<script lang="ts">
let accepted = $state(false)
</script>

{#if !accepted}
<div class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
	<div class="bg-white rounded-lg shadow-xl max-w-lg w-full max-h-[90vh] overflow-y-auto p-6">
		<h2 class="text-base font-bold text-gray-900 mb-4">ご利用にあたって</h2>

		<div class="text-sm text-gray-700 leading-relaxed space-y-3 mb-6">
			<p>
				本ツールは個人が作成した非公式のものであり、高知大学とは一切の関係がなく、同大学による承認又は推奨を受けたものではありません。
			</p>
			<p>
				表示されるデータは、<a href="https://www.kochi-u.ac.jp/education-support/courses/syllabus/" class="text-blue-600 underline hover:text-blue-800" target="_blank" rel="noopener noreferrer">同大学が一般に公開しているシラバス情報</a>のみに基づいており、非公開情報は一切使用していません。
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
			class="w-full py-2.5 px-4 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors cursor-pointer"
		>
			上記を確認の上、利用する
		</button>
	</div>
</div>
{/if}
