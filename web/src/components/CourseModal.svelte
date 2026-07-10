<script lang="ts">
import { quadOut } from 'svelte/easing'
import { slide } from 'svelte/transition'
import IconClose from '~icons/ic/round-close'
import IconExpandMore from '~icons/ic/round-expand-more'
import IconOpenInNew from '~icons/ic/round-open-in-new'
import IconPlace from '~icons/ic/round-place'
import IconSchedule from '~icons/ic/round-schedule'
import { getColor } from '../lib/colors'
import { loadDetail } from '../lib/details'
import { plan } from '../lib/plan.svelte'
import { FIELD_SPEC } from '../lib/syllabus-fields.generated'
import { deliveryMode } from '../lib/syllabus-icons'
import { useTheme } from '../lib/theme.svelte'
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

// Whether this course is in the user's plan (drives the header toggle).
const registered = $derived(plan.has(course.cd))

// The course's palette tint — the same hue the card carries, so opening a card
// continues its colour into the sheet header (and the plan-timeline nodes). Used
// as a surface / non-text sign only; body text stays on the AA-locked ink tokens.
const theme = useTheme()
const tint = $derived.by(() => {
	const c = getColor(course.cd)
	return theme.isDark ? c.dark : c.light
})

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

// Credits as blocks (echoes CourseCard): one square per credit, a half for .5,
// painted in the tile's border colour — the second, non-text use of the tint.
const creditsN = $derived(Number(detail?.unit) || 0)
const creditBlocks = $derived(Array.from({ length: Math.min(Math.floor(creditsN), 8) }))
const creditHalf = $derived(creditsN - Math.floor(creditsN) >= 0.5)

// Spec-driven sections, bundled into groups. Order/group come from FIELD_SPEC;
// `meta`/`delivery-badge` render as header chips, not rows. Hero fields (group
// `''` — 成績評価/概要) sit open at the top; the rest collapse under their group.
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
		// `extra` is a variable-length bag of unknown labels (KULAS layout drift),
		// deliberately kept out of the static FIELD_SPEC — fold it in here so it
		// isn't silently dropped, as longtext under the last group.
		if (detail.extra?.length) {
			for (const e of detail.extra) {
				if (e.text?.trim()) {
					rows.push({
						key: `extra:${e.label}`,
						label: e.label,
						group: OTHER_GROUP,
						render: 'longtext',
						value: e.text,
					})
				}
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

// Per-group open state. A custom button disclosure (not native <details>): the
// `slide` transition animates height in JS, so it needs no `interpolate-size` /
// `::details-content` — that pair stalled axe-core's contrast pass in CI.
let openGroups = $state<Record<string, boolean>>({})

// Taxonomy facets — a quiet middle-dot line, so delivery/credits read first.
const facets = $derived(
	[dicts.kubun[course.kbn], dicts.kaikojiki[course.ki], dicts.campuses[course.campus]]
		.filter(Boolean)
		.join(' · '),
)

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
		<!-- Tinted band: the card's colour carries into the sheet so the two read as
		     one object. Body text uses AA-locked tint ink (text/mutedText). -->
		<div class="px-4 pt-2 pb-3 sm:px-7 sm:pt-6 sm:pb-3 border-b border-overlay-subtle" style="background: {tint.bg};">
			<div class="flex justify-between items-start gap-3">
				<div class="min-w-0">
					<h2 class="text-title font-semibold leading-snug tracking-tight" style="color: {tint.text};">
						{course.nm}
					</h2>
					{#if course.sub}
						<p class="text-sub mt-1 tracking-tight" style="color: {tint.mutedText};">{course.sub}</p>
					{/if}
					<!-- Meta in three registers (mirrors the card): delivery as a filled
					     chip, credits as blocks, taxonomy as a quiet middle-dot line. -->
					<!-- All text on the tinted band uses the tile's own AA-locked ink
					     (mutedText), never the global slate greys — same as the card. -->
					<div class="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 mt-2.5" style="color: {tint.mutedText};">
						{#if delivery}
							<!-- On bg-overlay-medium (tile + slate) mutedText drops below AA;
							     override to the tile's max-contrast ink. -->
							<span class="inline-flex items-center gap-1 rounded-full bg-overlay-medium px-2 py-0.5 text-micro" style="color: {tint.text};">
								{delivery.emoji} {delivery.label}
							</span>
						{/if}
						{#if creditsN > 0}
							<span class="inline-flex items-center gap-1.5 text-micro" aria-label="{detail?.unit}単位">
								<span class="flex items-center gap-0.5" aria-hidden="true">
									{#each creditBlocks as _}
										<span class="w-2 h-2" style="background: {tint.border};"></span>
									{/each}
									{#if creditHalf}
										<span class="w-1 h-2" style="background: {tint.border};"></span>
									{/if}
								</span>
								<span class="tabular-nums">{detail?.unit}単位</span>
							</span>
						{/if}
					</div>
					{#if facets}
						<div class="mt-1.5 text-caption tracking-tight" style="color: {tint.mutedText};">{facets}</div>
					{/if}
				</div>
				<div class="flex items-center gap-1.5 shrink-0">
					<!-- Register / unregister this course into the plan. -->
					<button
						class="h-10 sm:h-8 rounded-full px-3.5 text-caption font-medium transition-colors duration-200 cursor-pointer
							{registered ? 'bg-apple-blue text-on-accent' : 'bg-overlay-light text-apple-text active:bg-overlay-strong sm:hover:bg-overlay-strong'}"
						onclick={() => plan.toggle(course.cd)}
						aria-pressed={registered}
					>
						{registered ? '✓ 登録済み' : '＋ 登録'}
					</button>
					<button
						class="w-10 h-10 sm:w-8 sm:h-8 rounded-full bg-overlay-light flex items-center justify-center active:bg-overlay-strong sm:hover:bg-overlay-strong transition-colors duration-200 cursor-pointer"
						onclick={close}
						aria-label="閉じる"
					>
						<IconClose class="w-3.5 h-3.5 text-apple-text-secondary" />
					</button>
				</div>
			</div>
		</div>
	{/snippet}

	<div class="px-4 pb-6 sm:px-7 sm:pb-7">
		{#if loading}
			{@render skeleton()}
		{:else}
			<!-- Hero: 成績評価 + 概要 — always open (decision-critical), no chevron. -->
			{#each heroSections as s (s.key)}
				<section class="border-b border-overlay-subtle py-4">
					<h3 class="text-caption font-semibold text-apple-text-secondary mb-2.5 tracking-tight">{s.label}</h3>
					{@render sectionBody(s)}
				</section>
			{/each}

			<!-- Each category is one collapsible group (collapsed by default); its
			     fields sit inside as labeled blocks, so the default view is short. A
			     button + `slide` (not <details>) so the open/close animates without
			     the axe-hanging `::details-content`/`interpolate-size`. -->
			{#each groupOrder as g (g)}
				{@const open = openGroups[g] ?? false}
				<div class="border-b border-overlay-subtle">
					<button
						type="button"
						data-section
						class="w-full flex items-center justify-between gap-2 py-4 cursor-pointer select-none text-left"
						aria-expanded={open}
						onclick={() => { openGroups[g] = !open }}
					>
						<h3 class="text-headline font-semibold text-apple-text tracking-tight">{g}</h3>
						<IconExpandMore class="w-4 h-4 shrink-0 text-apple-text-tertiary transition-transform duration-200 {open ? 'rotate-180' : ''}" />
					</button>
					{#if open}
						<div class="pb-4 space-y-6" transition:slide={{ duration: 200, easing: quadOut }}>
							{#each sectionsInGroup(g) as s (s.key)}
								<div>
									<h4 class="text-caption font-semibold text-apple-text-secondary mb-2 tracking-tight">{s.label}</h4>
									{@render sectionBody(s)}
								</div>
							{/each}
						</div>
					{/if}
				</div>
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
				<IconOpenInNew class="w-3.5 h-3.5" />
			</a>
		</div>
	</div>
</BottomSheet>

{#snippet icon(name: 'clock' | 'pin')}
	{#if name === 'clock'}
		<IconSchedule class="w-3.5 h-3.5 shrink-0 text-apple-text-tertiary" />
	{:else}
		<IconPlace class="w-3.5 h-3.5 shrink-0 text-apple-text-tertiary" />
	{/if}
{/snippet}

{#snippet sectionBody(s: Section)}
	{#if s.render === 'eval-chart'}
		{@const ev = s.value as import('../types/course').Eval}
		<div class="rounded-lg bg-overlay-subtle p-4 sm:p-5">
			<EvalChart rows={ev.rows} note={ev.note} />
		</div>
	{:else if s.render === 'longtext'}
		<p class="text-body text-apple-text leading-relaxed whitespace-pre-line tracking-tight">{s.value as string}</p>
	{:else if s.render === 'list'}
		<!-- Hand-numbered so the marker is a calm tabular figure, not a browser
		     bullet — a designed step list rather than a raw <ol>. -->
		<ol class="space-y-2">
			{#each s.value as string[] as item, i}
				<li class="flex gap-2.5">
					<span class="mt-0.5 shrink-0 text-caption font-semibold tabular-nums text-apple-text-tertiary">{i + 1}</span>
					<span class="text-body text-apple-text leading-relaxed tracking-tight">{item}</span>
				</li>
			{/each}
		</ol>
	{:else if s.render === 'plan-timeline'}
		{@const plan = s.value as import('../types/course').PlanItem[]}
		<!-- A real timeline: tinted node per session, a hairline rail connecting
		     them (dropped after the last). Node number sits in the tile hue. -->
		<ol>
			{#each plan as p, i}
				<li class="flex gap-3">
					<div class="flex flex-col items-center shrink-0">
						<span class="flex h-6 w-6 items-center justify-center rounded-full text-micro font-semibold tabular-nums" style="background: {tint.bg}; color: {tint.accentText};">{p.n}</span>
						{#if i < plan.length - 1}
							<span class="w-px grow bg-overlay-medium" aria-hidden="true"></span>
						{/if}
					</div>
					<p class="text-body text-apple-text leading-relaxed whitespace-pre-line tracking-tight pb-4">{p.text}</p>
				</li>
			{/each}
		</ol>
	{:else if s.render === 'office-table'}
		{@const rows = s.value as import('../types/course').OfficeHour[]}
		<!-- Each entry as a small card, keeping name / when / where distinct
		     instead of the old slash-joined line. -->
		<ul class="space-y-2">
			{#each rows as o}
				<li class="rounded-lg bg-overlay-subtle px-3 py-2.5">
					{#if o.name}
						<div class="text-sub font-medium text-apple-text tracking-tight">{o.name}</div>
					{/if}
					{#if o.day || o.time || o.place}
						<div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-caption text-apple-text-secondary">
							{#if o.day || o.time}
								<span class="inline-flex items-center gap-1.5">{@render icon('clock')}{[o.day, o.time].filter(Boolean).join(' ')}</span>
							{/if}
							{#if o.place}
								<span class="inline-flex items-center gap-1.5">{@render icon('pin')}{o.place}</span>
							{/if}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{:else if s.render === 'chips'}
		<div class="flex flex-wrap gap-1.5">
			{#each s.value as string[] as chip}
				<span class="inline-flex items-center rounded-full bg-overlay-light px-2 py-0.5 text-micro text-apple-text-secondary">{chip}</span>
			{/each}
		</div>
	{:else if s.render === 'base'}
		<dl>
			{#each s.value as [string, string | undefined | null][] as [label, value]}
				{#if value}
					<div class="flex gap-3 py-2 border-b border-overlay-subtle last:border-0">
						<dt class="shrink-0 w-24 text-caption text-apple-text-tertiary tracking-tight">{label}</dt>
						<dd class="text-body text-apple-text leading-relaxed tracking-tight">{value}</dd>
					</div>
				{/if}
			{/each}
		</dl>
	{/if}
{/snippet}

{#snippet skeleton()}
	<!-- Shaped like the loaded view: a donut placeholder + legend rows, then a
	     few summary lines and two collapsed group bars, so nothing jumps in. -->
	<div class="py-1 animate-pulse">
		<div class="rounded-lg bg-overlay-subtle p-4 sm:p-5 flex items-center gap-4">
			<div class="w-24 h-24 sm:w-28 sm:h-28 rounded-full bg-overlay-light shrink-0"></div>
			<div class="flex-1 space-y-2">
				<div class="h-3 rounded-lg bg-overlay-light w-3/4"></div>
				<div class="h-3 rounded-lg bg-overlay-light w-1/2"></div>
				<div class="h-3 rounded-lg bg-overlay-light w-2/3"></div>
			</div>
		</div>
		<div class="space-y-2 mt-5">
			<div class="h-3.5 rounded-lg bg-overlay-light w-full"></div>
			<div class="h-3.5 rounded-lg bg-overlay-light w-5/6"></div>
			<div class="h-3.5 rounded-lg bg-overlay-light w-2/3"></div>
		</div>
		<div class="mt-6 space-y-4">
			<div class="h-4 rounded-lg bg-overlay-light w-32"></div>
			<div class="h-4 rounded-lg bg-overlay-light w-24"></div>
		</div>
	</div>
{/snippet}
