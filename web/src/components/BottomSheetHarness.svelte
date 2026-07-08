<script lang="ts">
// Test-only wrapper: BottomSheet requires a `children` snippet, which is
// awkward to pass from a `.svelte.test.ts`. This supplies a trivial body so the
// spec can focus on the close-routing state machine.
import BottomSheet from './BottomSheet.svelte'

let { onclose, ariaLabel = 'テストシート' }: { onclose: () => void; ariaLabel?: string } = $props()
</script>

<BottomSheet {onclose} {ariaLabel}>
	<p>本文</p>
</BottomSheet>
