<script lang="ts">
import IconClose from '~icons/ic/round-close'
import IconSearch from '~icons/ic/round-search'

interface Props {
	searchText: string
}

let { searchText = $bindable() }: Props = $props()
</script>

<!-- Desktop full-width search bar; mobile shares this state via the filter sheet. -->
<div class="hidden sm:block border-b border-overlay-subtle bg-surface-primary/40 backdrop-blur-md px-6 py-3">
	<div class="relative">
		<IconSearch class="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-apple-text-tertiary pointer-events-none" />
		<input
			type="text"
			bind:value={searchText}
			aria-label="科目名・教員・キーワードで検索"
			placeholder="科目名・教員・キーワードで検索"
			class="w-full bg-overlay-subtle rounded-xl pl-12 pr-12 py-3 text-body text-apple-text outline-none placeholder:text-apple-text-tertiary transition-all duration-200 focus:bg-surface-primary focus:ring-2 focus:ring-apple-blue/40 focus:shadow-sm"
		/>
		{#if searchText}
			<button
				class="absolute right-3 top-1/2 -translate-y-1/2 w-7 h-7 rounded-full bg-overlay-medium flex items-center justify-center hover:bg-overlay-strong transition-colors cursor-pointer"
				onclick={() => { searchText = '' }}
				aria-label="検索をクリア"
			>
				<IconClose class="w-3.5 h-3.5 text-apple-text-secondary" />
			</button>
		{/if}
	</div>
</div>
