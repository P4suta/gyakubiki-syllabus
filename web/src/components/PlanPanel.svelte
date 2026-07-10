<script lang="ts">
import type { PlanSummaryResult } from '../lib/engine'
import { plan, shareUrl } from '../lib/plan.svelte'
import type { Course } from '../types/course'
import BottomSheet from './BottomSheet.svelte'

interface Props {
	summary: PlanSummaryResult | null
	courses: readonly Course[]
	onclose: () => void
}

let { summary, courses, onclose }: Props = $props()

const byCd = $derived(new Map(courses.map((c) => [c.cd, c])))
const registered = $derived(plan.cds.map((cd) => byCd.get(cd)).filter((c): c is Course => !!c))

const totalCredits = $derived(summary?.credits.totalCredits ?? 0)
const conflictCount = $derived(summary?.conflicts.length ?? 0)

// Course names colliding in each conflict cell (indices → names).
const conflictNames = $derived(
	(summary?.conflicts ?? []).map((c) => ({
		...c,
		names: c.courses.map((i) => courses[i]?.nm ?? '?'),
	})),
)

let copied = $state(false)
async function share() {
	try {
		await navigator.clipboard.writeText(shareUrl())
		copied = true
		setTimeout(() => {
			copied = false
		}, 1800)
	} catch {
		copied = false
	}
}
</script>

<BottomSheet {onclose} ariaLabel="マイ時間割">
	{#snippet header()}
		<div class="flex items-baseline gap-2">
			<h2 class="text-headline font-semibold text-apple-text tracking-tight">マイ時間割</h2>
			<span class="text-caption text-apple-text-secondary tabular-nums">
				{registered.length}科目 · {totalCredits}単位
			</span>
		</div>
	{/snippet}

	{#if registered.length === 0}
		<p class="text-body text-apple-text-secondary text-center py-8 tracking-tight">
			科目の詳細から「登録」すると、ここに集まります。
		</p>
	{:else}
		{#if conflictCount > 0}
			<div class="mb-3 rounded-lg bg-overlay-medium p-3">
				<p class="text-caption font-semibold text-apple-text mb-1">
					⚠ 時間割の重複が{conflictCount}件あります
				</p>
				<ul class="text-micro text-apple-text-secondary space-y-0.5">
					{#each conflictNames as c}
						<li>{c.names.join(' ・ ')}</li>
					{/each}
				</ul>
			</div>
		{/if}

		<!-- The grid itself is the course list now — manage a slot by tapping its
		     card. Here we keep just the at-a-glance summary. -->
		<p class="text-micro text-apple-text-tertiary mb-4 tracking-tight">
			各コマのカードをタップすると登録・解除できます。
		</p>

		<!-- 必修/選択 breakdown -->
		{#if summary && summary.credits.byNen.length > 0}
			<div class="flex flex-wrap gap-1.5 mb-4">
				{#each summary.credits.byNen as tally}
					<span class="rounded-full bg-overlay-light px-2.5 py-1 text-micro text-apple-text-secondary">
						{tally.key} {tally.credits}単位
					</span>
				{/each}
			</div>
		{/if}
	{/if}

	{#snippet footer()}
		<div class="flex gap-2">
			<button
				class="flex-1 py-2.5 rounded-full bg-apple-blue text-on-accent text-cta font-normal hover:bg-apple-blue-hover transition-colors cursor-pointer"
				onclick={share}
			>
				{copied ? 'コピーしました' : '共有リンクをコピー'}
			</button>
			{#if registered.length > 0}
				<button
					class="shrink-0 py-2.5 px-4 rounded-full bg-overlay-medium text-apple-text text-cta font-normal hover:bg-overlay-strong transition-colors cursor-pointer"
					onclick={() => plan.clear()}
				>
					全消去
				</button>
			{/if}
		</div>
	{/snippet}
</BottomSheet>
