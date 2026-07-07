<script lang="ts">
interface Props {
	searchText: string
}

let { searchText = $bindable() }: Props = $props()
</script>

<!-- Desktop full-width search bar; mobile shares this state via the filter sheet. -->
<div class="hidden sm:block border-b border-overlay-light bg-surface-primary/40 backdrop-blur-md px-6 py-3">
	<div class="relative">
		<svg
			class="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-apple-text/40 pointer-events-none"
			fill="none"
			viewBox="0 0 24 24"
			stroke="currentColor"
			stroke-width="2"
		>
			<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
		</svg>
		<input
			type="text"
			bind:value={searchText}
			placeholder="科目名・教員・内容で検索"
			class="w-full bg-overlay-subtle rounded-xl pl-12 pr-12 py-3 text-body text-apple-text outline-none placeholder:text-apple-text/40 transition-all duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/30 focus:shadow-sm"
		/>
		{#if searchText}
			<button
				class="absolute right-3 top-1/2 -translate-y-1/2 w-7 h-7 rounded-full bg-apple-text/20 flex items-center justify-center hover:bg-apple-text/30 transition-colors cursor-pointer"
				onclick={() => { searchText = '' }}
				aria-label="検索をクリア"
			>
				<svg
					class="w-3 h-3 text-surface-primary"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="3"
				>
					<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</button>
		{/if}
	</div>
</div>
