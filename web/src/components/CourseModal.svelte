<script lang="ts">
import { loadDetail } from '../lib/details'
import { FIELD_SPEC } from '../lib/syllabus-fields.generated'
import { deliveryMode } from '../lib/syllabus-icons'
import type { Course, CourseDetail, Dictionaries } from '../types/course'
import BottomSheet from './BottomSheet.svelte'
import EvalChart from './EvalChart.svelte'

interface Props {
	course: Course
	dicts: Dictionaries
	year: string
	onclose: () => void
}

let { course, dicts, year, onclose }: Props = $props()

// Deep link to KULAS's official「シラバス参照」page (plain GET, no token).
const SANSHO_BASE =
	'https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/simple/1900/3000280/wsl/SyllabusSansho'
const officialUrl = $derived(
	`${SANSHO_BASE}?kogiCd=${encodeURIComponent(course.cd)}&kaikoNendo=${encodeURIComponent(year)}&syllabusKomokuPatternId=${encodeURIComponent(course.pat ?? '4')}`,
)

// Base grid fields (always available, no fetch) — shown as one more accordion.
const baseFields: [string, string | undefined | null][] = $derived([
	['授業コード', course.cd],
	['時間割', course.raw],
	['担当教員', course.prof],
	['開講責任部署', dicts.departments[course.dept]],
	['学則科目', course.gaku ?? course.nm],
	['対象学科/年次', course.gakka],
	['必須/選択', course.nen],
	['科目分類', course.bunrui],
	['科目分野', course.bunya],
])

// Lazily loaded rich syllabus detail.
let detail = $state<CourseDetail | null>(null)
let loading = $state(true)

$effect(() => {
	const cd = course.cd
	loading = true
	detail = null
	loadDetail(cd).then((d) => {
		// Guard against a race if the user opened another course meanwhile.
		if (cd === course.cd) {
			detail = d
			loading = false
		}
	})
})

const delivery = $derived(detail?.delivery ? deliveryMode(detail.delivery.mode) : null)

// Spec-driven sections, bundled into groups. Order/group come from FIELD_SPEC;
// `meta`/`delivery-badge` render as header chips, not rows. Hero fields (group
// `''` — 成績評価/概要) open at the top with no subheading; the rest sit under a
// subheading (授業内容 / その他), collapsed, all sharing one disclosure affordance.
type Section = { key: string; label: string; group: string; render: string; value: unknown }

// 科目情報 (base fields) always exist; treat them as one more row in the last
// FIELD_SPEC group so the grouping needs no separate definition here.
const OTHER_GROUP = FIELD_SPEC[FIELD_SPEC.length - 1].group

const allSections = $derived.by<Section[]>(() => {
	const rows: Section[] = []
	if (detail) {
		const d = detail as unknown as Record<string, unknown>
		for (const f of FIELD_SPEC) {
			if (f.render === 'meta' || f.render === 'delivery-badge') continue
			const value = d[f.key]
			if (hasValue(value)) {
				rows.push({ key: f.key, label: f.label, group: f.group, render: f.render, value })
			}
		}
	}
	rows.push({
		key: '__base__',
		label: '科目情報',
		group: OTHER_GROUP,
		render: 'base',
		value: baseFields,
	})
	return rows
})

const heroSections = $derived(allSections.filter((s) => s.group === ''))
// Distinct non-hero groups, in FIELD_SPEC order (授業内容 → その他).
const groupOrder = $derived([
	...new Set(allSections.filter((s) => s.group !== '').map((s) => s.group)),
])
const sectionsInGroup = (group: string) => allSections.filter((s) => s.group === group)

function hasValue(v: unknown): boolean {
	if (v == null) return false
	if (Array.isArray(v)) return v.length > 0
	if (typeof v === 'string') return v.trim().length > 0
	if (typeof v === 'object') return Object.keys(v as object).length > 0
	return true
}
</script>

<BottomSheet {onclose} ariaLabel={course.nm}>
	{#snippet header(close)}
		<div class="px-4 pt-2 pb-3 sm:px-7 sm:pt-6 sm:pb-3">
			<div class="flex justify-between items-start gap-3">
				<div class="min-w-0">
					<h2 class="text-xl font-bold text-apple-text leading-snug tracking-tight">
						{course.nm}
					</h2>
					{#if course.sub}
						<p class="text-sub text-apple-text-tertiary mt-1 tracking-tight">{course.sub}</p>
					{/if}
					<div class="flex flex-wrap gap-1.5 mt-2.5">
						{#if delivery}
							<span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-overlay-light text-micro text-apple-text/80">
								{delivery.emoji} {delivery.label}
							</span>
						{/if}
						{#if detail?.unit}
							<span class="px-2 py-0.5 rounded-full bg-overlay-light text-micro text-apple-text/80">{detail.unit}単位</span>
						{/if}
						<span class="px-2 py-0.5 rounded-full bg-overlay-light text-micro text-apple-text/80">{dicts.kubun[course.kbn]}</span>
						<span class="px-2 py-0.5 rounded-full bg-overlay-light text-micro text-apple-text/80">{dicts.kaikojiki[course.ki]}</span>
						<span class="px-2 py-0.5 rounded-full bg-overlay-light text-micro text-apple-text/80">{dicts.campuses[course.campus]}</span>
					</div>
				</div>
				<button
					class="shrink-0 w-10 h-10 sm:w-8 sm:h-8 rounded-full bg-overlay-light flex items-center justify-center active:bg-overlay-strong sm:hover:bg-overlay-strong transition-colors duration-200 cursor-pointer"
					onclick={close}
					aria-label="閉じる"
				>
					<svg class="w-3.5 h-3.5 text-apple-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>
		</div>
	{/snippet}

	<div class="px-4 pb-6 sm:px-7 sm:pb-7">
		{#if loading}
			<div class="space-y-3 py-4 animate-pulse">
				<div class="h-24 rounded-xl bg-overlay-light"></div>
				<div class="h-4 rounded bg-overlay-light w-3/4"></div>
				<div class="h-4 rounded bg-overlay-light w-1/2"></div>
			</div>
		{:else}
			<!-- Hero: 成績評価 + 概要, open by default. -->
			{#each heroSections as s (s.key)}
				<details class="group border-b border-overlay-subtle" open>
					{@render disclosure(s.label)}
					<div class="pb-3.5">{@render sectionBody(s)}</div>
				</details>
			{/each}

			<!-- Each category is one collapsible group (collapsed by default); its
			     fields sit inside as labeled blocks, so the default view is short. -->
			{#each groupOrder as g (g)}
				<details class="group border-b border-overlay-subtle">
					{@render disclosure(g)}
					<div class="pb-4 space-y-5">
						{#each sectionsInGroup(g) as s (s.key)}
							<div>
								<h4 class="text-caption font-semibold text-apple-text-secondary mb-2 tracking-tight">{s.label}</h4>
								{@render sectionBody(s)}
							</div>
						{/each}
					</div>
				</details>
			{/each}
		{/if}

		<!-- Official KULAS syllabus (source of truth) -->
		<div class="pt-4">
			<a
				href={officialUrl}
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-1 text-body text-apple-blue hover:underline tracking-tight"
			>
				公式シラバスで見る
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
				</svg>
			</a>
		</div>
	</div>
</BottomSheet>

<!-- One disclosure affordance for every collapsible section: label + a chevron
     that rotates when open. No blue-link "read more" anywhere. -->
{#snippet disclosure(label: string)}
	<summary class="flex items-center justify-between gap-2 py-3.5 cursor-pointer list-none select-none">
		<span class="text-caption font-semibold text-apple-text-secondary tracking-tight">{label}</span>
		<svg class="w-4 h-4 shrink-0 text-apple-text-tertiary transition-transform duration-200 group-open:rotate-180" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
		</svg>
	</summary>
{/snippet}

{#snippet sectionBody(s: Section)}
	{#if s.render === 'eval-chart'}
		{@const ev = s.value as import('../types/course').Eval}
		<EvalChart rows={ev.rows} note={ev.note} />
	{:else if s.render === 'longtext'}
		<p class="text-body text-apple-text/90 leading-relaxed whitespace-pre-line tracking-tight">{s.value as string}</p>
	{:else if s.render === 'list'}
		<ol class="list-decimal list-outside pl-5 space-y-1">
			{#each s.value as string[] as item}
				<li class="text-body text-apple-text/90 leading-relaxed tracking-tight">{item}</li>
			{/each}
		</ol>
	{:else if s.render === 'plan-timeline'}
		{@const plan = s.value as import('../types/course').PlanItem[]}
		<ol class="space-y-2">
			{#each plan as p}
				<li class="flex gap-3">
					<span class="shrink-0 w-12 whitespace-nowrap text-micro font-semibold text-apple-blue tabular-nums">第{p.n}回</span>
					<span class="text-body text-apple-text/90 leading-relaxed whitespace-pre-line tracking-tight">{p.text}</span>
				</li>
			{/each}
		</ol>
	{:else if s.render === 'office-table'}
		{@const rows = s.value as import('../types/course').OfficeHour[]}
		<div class="space-y-1">
			{#each rows as o}
				<div class="text-body text-apple-text/90 leading-relaxed tracking-tight">
					{[o.name, o.day, o.time, o.place].filter(Boolean).join(' / ')}
				</div>
			{/each}
		</div>
	{:else if s.render === 'chips'}
		<div class="flex flex-wrap gap-1.5">
			{#each s.value as string[] as chip}
				<span class="px-2 py-0.5 rounded-full bg-overlay-light text-micro text-apple-text/80">{chip}</span>
			{/each}
		</div>
	{:else if s.render === 'base'}
		<dl>
			{#each s.value as [string, string | undefined | null][] as [label, value]}
				{#if value}
					<div class="flex gap-3 py-1.5">
						<dt class="shrink-0 w-24 text-caption text-apple-text-tertiary tracking-tight">{label}</dt>
						<dd class="text-body text-apple-text/90 leading-relaxed tracking-tight">{value}</dd>
					</div>
				{/if}
			{/each}
		</dl>
	{/if}
{/snippet}
