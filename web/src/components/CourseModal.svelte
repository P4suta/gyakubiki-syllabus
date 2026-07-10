<script lang="ts">
import type { Component } from 'svelte'
import { quadOut } from 'svelte/easing'
import { slide } from 'svelte/transition'
import IconAdd from '~icons/ic/round-add'
import IconCheck from '~icons/ic/round-check'
import IconCheckCircle from '~icons/ic/round-check-circle'
import IconClose from '~icons/ic/round-close'
import IconExam from '~icons/ic/round-history-edu'
import IconExpandMore from '~icons/ic/round-expand-more'
import IconOpenInNew from '~icons/ic/round-open-in-new'
import IconPerson from '~icons/ic/round-person'
import IconPlace from '~icons/ic/round-place'
import IconSchedule from '~icons/ic/round-schedule'
import IconSearch from '~icons/ic/round-search'
import { getColor } from '../lib/colors'
import {
	classifyGoals,
	decodeNumbering,
	formatProse,
	linkifyTitles,
	parseTeachers,
} from '../lib/detail-format'
import { loadDetail } from '../lib/details'
import { plan } from '../lib/plan.svelte'
import { sdgGoal } from '../lib/sdgs'
import { FIELD_SPEC } from '../lib/syllabus-fields.generated'
import { deliveryMode, FIELD_ICONS } from '../lib/syllabus-icons'
import { useTheme } from '../lib/theme.svelte'
import type { Course, CourseDetail, Dictionaries, Eval, OfficeHour, PlanItem } from '../types/course'
import BottomSheet from './BottomSheet.svelte'
import EvalChart from './EvalChart.svelte'

interface Props {
	course: Course
	dicts: Dictionaries
	year: string
	onclose: () => void
	/** Tapping a keyword runs it as a search (reverse-lookup) and closes here. */
	onsearch?: (q: string) => void
}

let { course, dicts, year, onclose, onsearch }: Props = $props()

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
// Several (対象学科/年次・科目分類/分野) are empty in the real KULAS grid, so they
// simply omit themselves rather than showing blanks.
const baseFields: [string, string | undefined | null][] = $derived([
	['授業コード', course.cd],
	['時間割', course.raw],
	['担当教員', course.prof],
	['開講責任部署', dicts.departments[course.dept]],
	['学則科目', course.gaku ?? course.nm],
	['対象学科', course.gakka],
	['対象年次', course.nen],
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

// A leading wayfinding icon for a section (base uses its own key).
function sectionIcon(key: string): Component | undefined {
	return FIELD_ICONS[key === '__base__' ? 'base' : key]
}

// A plan session's badge label from its enriched `kind` (text stays verbatim).
function planBadge(kind: string | undefined): string | null {
	if (kind === 'exam') return '試験'
	if (kind === 'milestone') return '節目'
	if (kind === 'start') return '開始'
	return null
}
</script>

<BottomSheet {onclose} ariaLabel={course.nm} accent={tint.bg}>
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
					<div class="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 mt-2.5" style="color: {tint.mutedText};">
						{#if delivery}
							{@const DIcon = delivery.icon}
							<!-- On bg-overlay-medium (tile + slate) mutedText drops below AA;
							     override to the tile's max-contrast ink. -->
							<span class="inline-flex items-center gap-1 rounded-full bg-overlay-medium px-2 py-0.5 text-micro" style="color: {tint.text};">
								<DIcon class="w-3 h-3" aria-hidden="true" />{delivery.label}
							</span>
						{/if}
						{#if detail?.delivery?.isMedia}
							<span class="inline-flex items-center rounded-full bg-overlay-medium px-2 py-0.5 text-micro" style="color: {tint.text};">メディア授業</span>
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
						class="inline-flex items-center gap-1 h-10 sm:h-8 rounded-full px-3.5 text-caption font-medium transition-colors duration-200 cursor-pointer
							{registered ? 'bg-apple-blue text-on-accent' : 'bg-overlay-light text-apple-text active:bg-overlay-strong sm:hover:bg-overlay-strong'}"
						onclick={() => plan.toggle(course.cd)}
						aria-pressed={registered}
					>
						{#if registered}<IconCheck class="w-3.5 h-3.5" aria-hidden="true" />登録済み{:else}<IconAdd class="w-3.5 h-3.5" aria-hidden="true" />登録{/if}
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
				{@const HIcon = sectionIcon(s.key)}
				<section class="border-b border-overlay-subtle py-4">
					<h3 class="flex items-center gap-1.5 text-caption font-semibold text-apple-text-secondary mb-2.5 tracking-tight">
						{#if HIcon}<HIcon class="w-4 h-4 shrink-0 text-apple-text-tertiary" aria-hidden="true" />{/if}{s.label}
					</h3>
					{@render sectionBody(s)}
				</section>
			{/each}

			<!-- Each category is one collapsible group (collapsed by default), led by
			     its own icon so the grouping reads at a glance. A button + `slide`
			     (not <details>) so open/close animates without the axe-hanging
			     `::details-content`/`interpolate-size`. -->
			{#each groupOrder as g (g)}
				{@const open = openGroups[g] ?? false}
				{@const GIcon = FIELD_ICONS[g]}
				<div class="border-b border-overlay-subtle">
					<button
						type="button"
						data-section
						class="w-full flex items-center gap-2 py-4 cursor-pointer select-none text-left"
						aria-expanded={open}
						onclick={() => { openGroups[g] = !open }}
					>
						{#if GIcon}<GIcon class="w-5 h-5 shrink-0 text-apple-text-secondary" aria-hidden="true" />{/if}
						<h3 class="grow text-headline font-semibold text-apple-text tracking-tight">{g}</h3>
						<IconExpandMore class="w-4 h-4 shrink-0 text-apple-text-tertiary transition-transform duration-200 {open ? 'rotate-180' : ''}" />
					</button>
					{#if open}
						<div class="pb-4 space-y-6" transition:slide={{ duration: 200, easing: quadOut }}>
							{#each sectionsInGroup(g) as s (s.key)}
								{@const FIcon = sectionIcon(s.key)}
								<div>
									<h4 class="flex items-center gap-1.5 text-caption font-semibold text-apple-text-secondary mb-2 tracking-tight">
										{#if FIcon}<FIcon class="w-4 h-4 shrink-0 text-apple-text-tertiary" aria-hidden="true" />{/if}{s.label}
									</h4>
									{@render sectionBody(s)}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		{/if}

		<!-- Official KULAS syllabus (source of truth) — kept prominent so this stays
		     a faithful view of the official page, not a replacement for it. -->
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

<!-- Free text with 『』/「」 book titles linked to a Google search. -->
{#snippet linked(text: string)}
	{#each linkifyTitles(text) as part}{#if part.href}<a href={part.href} target="_blank" rel="noopener noreferrer" class="text-apple-blue hover:underline">{part.text}</a>{:else}{part.text}{/if}{/each}
{/snippet}

{#snippet sectionBody(s: Section)}
	{#if s.render === 'eval-chart'}
		{@const ev = s.value as Eval}
		<div class="rounded-lg bg-overlay-subtle p-4 sm:p-5">
			<EvalChart rows={ev.rows} note={ev.note} />
		</div>
	{:else if s.render === 'longtext'}
		<!-- Split on existing structure: bullet runs become a list, the rest stays
		     verbatim prose. No rewording — just paragraphing. -->
		{#each formatProse(s.value as string) as block}
			{#if block.kind === 'list'}
				<ul class="space-y-1.5 my-1">
					{#each block.items as item}
						<li class="flex gap-2">
							<span class="mt-2 h-1 w-1 shrink-0 rounded-full bg-apple-text-tertiary" aria-hidden="true"></span>
							<span class="text-body text-apple-text leading-relaxed tracking-tight">{item}</span>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="text-body text-apple-text leading-relaxed whitespace-pre-line tracking-tight">{block.items[0]}</p>
			{/if}
		{/each}
	{:else if s.render === 'checklist'}
		<!-- 到達目標 as a can-do checklist: a filled check for「〜できる」competencies,
		     a hollow one otherwise. A leading【区分】becomes a small badge. -->
		<ul class="space-y-2.5">
			{#each classifyGoals(s.value as string[]) as g}
				<li class="flex gap-2.5">
					<IconCheckCircle class="w-4 h-4 shrink-0 mt-0.5 {g.canDo ? 'text-apple-blue' : 'text-apple-text-tertiary'}" aria-hidden="true" />
					<span class="text-body text-apple-text leading-relaxed tracking-tight">
						{#if g.tag}<span class="mr-1.5 inline-flex items-center rounded-full bg-overlay-light px-1.5 py-0.5 text-fine font-medium text-apple-text-secondary align-middle">{g.tag}</span>{/if}{g.text}
					</span>
				</li>
			{/each}
		</ul>
	{:else if s.render === 'plan-timeline'}
		{@const sessions = s.value as PlanItem[]}
		{@const examNs = sessions.filter((p) => p.kind === 'exam').map((p) => p.n)}
		<!-- A real timeline: a node per session on a hairline rail. Exam sessions
		     turn red with a badge, so「試験はいつ？」reads at a glance. -->
		{#if examNs.length}
			<div class="mb-3 inline-flex items-center gap-1.5 rounded-full bg-apple-red/10 px-2.5 py-1 text-micro font-medium text-apple-red">
				<IconExam class="w-3.5 h-3.5" aria-hidden="true" />試験: 第{examNs.join('・')}回
			</div>
		{/if}
		<ol>
			{#each sessions as p, i}
				{@const exam = p.kind === 'exam'}
				{@const badge = planBadge(p.kind)}
				<li class="flex gap-3">
					<div class="flex flex-col items-center shrink-0">
						<span
							class="flex h-6 w-6 items-center justify-center rounded-full text-micro font-semibold tabular-nums {exam ? 'bg-apple-red text-on-accent' : ''}"
							style={exam ? undefined : `background: ${tint.bg}; color: ${tint.accentText};`}
						>{p.n}</span>
						{#if i < sessions.length - 1}
							<span class="w-px grow bg-overlay-medium" aria-hidden="true"></span>
						{/if}
					</div>
					<div class="min-w-0 pb-4">
						{#if badge}
							<span class="mb-1 inline-flex items-center rounded-full px-1.5 py-0.5 text-fine font-medium {exam ? 'bg-apple-red/10 text-apple-red' : 'bg-overlay-light text-apple-text-secondary'}">{badge}</span>
						{/if}
						<p class="text-body text-apple-text leading-relaxed whitespace-pre-line tracking-tight">{p.text}</p>
					</div>
				</li>
			{/each}
		</ol>
	{:else if s.render === 'textbooks'}
		{@const info = detail?.textbookInfo}
		{#if info?.isNone}
			<!-- Show the source wording verbatim (「特になし」等), just quieted to a badge. -->
			<span class="inline-flex items-center rounded-full bg-overlay-light px-2.5 py-1 text-caption text-apple-text-secondary">{info.sections[0]?.lines[0] ?? s.value}</span>
		{:else if info}
			<!-- Split into 教科書/参考書… sections; every source line kept verbatim,
			     book titles linkified to a search. -->
			<div class="space-y-3">
				{#each info.sections as sec}
					<div>
						{#if sec.label}
							<div class="text-caption font-semibold text-apple-text-secondary mb-1 tracking-tight">{@render linked(sec.label)}</div>
						{/if}
						{#if sec.lines.length}
							<ul class="space-y-1">
								{#each sec.lines as line}
									<li class="text-body text-apple-text leading-relaxed tracking-tight">{@render linked(line)}</li>
								{/each}
							</ul>
						{/if}
					</div>
				{/each}
			</div>
		{:else}
			<p class="text-body text-apple-text leading-relaxed whitespace-pre-line tracking-tight">{@render linked(s.value as string)}</p>
		{/if}
	{:else if s.render === 'prep'}
		{@const info = detail?.prepInfo}
		<!-- Surface the study-time when the text states one; the full text is
		     always shown so nothing is hidden. -->
		<div class="space-y-2.5">
			{#if info?.hours}
				<div>
					<span class="inline-flex items-center gap-1.5 rounded-full bg-overlay-light px-2.5 py-1 text-caption text-apple-text">
						<IconSchedule class="w-3.5 h-3.5 text-apple-blue" aria-hidden="true" />目安 約{info.hours}時間 / 回
					</span>
				</div>
			{/if}
			<p class="text-body text-apple-text leading-relaxed whitespace-pre-line tracking-tight">{s.value as string}</p>
		</div>
	{:else if s.render === 'office-table'}
		{@const rows = s.value as OfficeHour[]}
		<!-- Each entry as a small card, keeping name / when / where distinct. -->
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
	{:else if s.render === 'people'}
		{@const t = parseTeachers(s.value as string[])}
		<!-- 代表教員 (KULAS's ◎) first with a badge, co-instructors after — the row
		     of numbers is gone, each name gets a person icon instead. -->
		<div class="flex flex-wrap gap-1.5">
			{#if t.rep}
				<span class="inline-flex items-center gap-1.5 rounded-full bg-overlay-light py-1 pl-1.5 pr-2 text-caption text-apple-text">
					<IconPerson class="w-3.5 h-3.5 text-apple-blue" aria-hidden="true" />{t.rep}
					<span class="rounded-full bg-apple-blue/10 px-1.5 text-fine font-medium text-apple-blue">代表</span>
				</span>
			{/if}
			{#each t.others as name}
				<span class="inline-flex items-center gap-1.5 rounded-full bg-overlay-light py-1 pl-1.5 pr-2.5 text-caption text-apple-text-secondary">
					<IconPerson class="w-3.5 h-3.5 text-apple-text-tertiary" aria-hidden="true" />{name}
				</span>
			{/each}
			{#if t.omnibus}
				<span class="inline-flex items-center rounded-full bg-overlay-light px-2 py-1 text-caption text-apple-text-secondary">オムニバス</span>
			{/if}
		</div>
	{:else if s.render === 'keywords'}
		<!-- Keywords are the reverse-lookup surface: tap one to search it. -->
		<div class="flex flex-wrap gap-1.5">
			{#each s.value as string[] as kw}
				<button
					type="button"
					onclick={() => onsearch?.(kw)}
					class="inline-flex items-center gap-1 rounded-full bg-overlay-light px-2 py-0.5 text-micro text-apple-text-secondary active:bg-overlay-medium sm:hover:bg-overlay-medium transition-colors cursor-pointer"
				>
					<IconSearch class="w-3 h-3 text-apple-text-tertiary" aria-hidden="true" />{kw}
				</button>
			{/each}
		</div>
	{:else if s.render === 'numbering'}
		<!-- Raw code first (the authority); a verified faculty label appended only
		     when confident, so nothing is inferred away from the official value. -->
		<div class="flex flex-wrap gap-1.5">
			{#each s.value as string[] as code}
				{@const dec = decodeNumbering(code)}
				<span class="inline-flex items-center gap-1.5 rounded-full bg-overlay-light px-2.5 py-0.5 text-micro text-apple-text-secondary">
					<span class="tabular-nums tracking-tight">{code}</span>
					{#if dec}<span class="text-apple-text-tertiary">· {dec.field}</span>{/if}
				</span>
			{/each}
		</div>
	{:else if s.render === 'sdgs'}
		<!-- Each goal links out to its UNICEF page; the official-colour badge is a
		     decorative sign (aria-hidden), the title carries the meaning as ink. -->
		<div class="flex flex-wrap gap-1.5">
			{#each s.value as string[] as raw}
				{@const g = sdgGoal(raw)}
				{#if g}
					<a
						href={g.url}
						target="_blank"
						rel="noopener noreferrer"
						class="inline-flex items-center gap-1.5 rounded-full bg-overlay-light py-0.5 pl-0.5 pr-2.5 text-micro text-apple-text-secondary active:bg-overlay-medium sm:hover:bg-overlay-medium transition-colors"
						aria-label="SDGs目標{g.n} {g.title}（新しいタブで開く）"
					>
						<span class="flex h-5 w-5 items-center justify-center rounded-full text-micro font-semibold tabular-nums text-white" style="background: {g.color};" aria-hidden="true">{g.n}</span>
						<span>{g.title}</span>
						<IconOpenInNew class="w-3 h-3 shrink-0 text-apple-text-tertiary" aria-hidden="true" />
					</a>
				{:else}
					<span class="inline-flex items-center rounded-full bg-overlay-light px-2 py-0.5 text-micro text-apple-text-secondary">{raw}</span>
				{/if}
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
