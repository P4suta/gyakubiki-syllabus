// Display mapping for the stable enum strings the Rust pipeline assigns
// (`EvalRow.type`, `Delivery.mode`) plus a per-field icon. Icons are Material
// `ic` (round) from Iconify — one set across the app, no emoji. Colours share
// the course palette's OKLCH hues and carry a light/dark pair so the eval-chart
// arcs stay legible on either modal surface. The *ranking* of fields lives in
// the generated `syllabus-fields.generated.ts`.

import type { Component } from 'svelte'
import IconAttendance from '~icons/ic/round-front-hand'
import IconExam from '~icons/ic/round-history-edu'
import IconOther from '~icons/ic/round-label'
import IconMiniReport from '~icons/ic/round-edit-note'
import IconQuiz from '~icons/ic/round-quiz'
import IconReport from '~icons/ic/round-description'
import IconHybrid from '~icons/ic/round-shuffle'
import IconOndemand from '~icons/ic/round-ondemand-video'
import IconOnline from '~icons/ic/round-videocam'
import IconOnsite from '~icons/ic/round-groups'
// Field / group icons.
import IconAims from '~icons/ic/round-flag'
import IconBase from '~icons/ic/round-info'
import IconEval from '~icons/ic/round-donut-large'
import IconGoals from '~icons/ic/round-check-circle'
import IconGroupOther from '~icons/ic/round-more-horiz'
import IconKeywords from '~icons/ic/round-tag'
import IconNumbering from '~icons/ic/round-tag'
import IconOfficeHour from '~icons/ic/round-meeting-room'
import IconPlan from '~icons/ic/round-event'
import IconPrep from '~icons/ic/round-schedule'
import IconPrereq from '~icons/ic/round-rule'
import IconSummary from '~icons/ic/round-subject'
import IconTeachers from '~icons/ic/round-person'
import IconTextbooks from '~icons/ic/round-menu-book'

export interface KindColor {
	light: string
	dark: string
}

export interface KindStyle {
	icon: Component
	label: string
	color: KindColor
}

export const EVAL_KIND: Record<string, KindStyle> = {
	exam: { icon: IconExam, label: '試験', color: { light: '#fa285c', dark: '#fb7e8c' } },
	report: { icon: IconReport, label: 'レポート', color: { light: '#1a8fef', dark: '#65b0fb' } },
	minireport: { icon: IconMiniReport, label: '小レポート', color: { light: '#1da09a', dark: '#26c3bc' } },
	attendance: { icon: IconAttendance, label: '意欲・参加', color: { light: '#1da751', dark: '#26cb64' } },
	quiz: { icon: IconQuiz, label: '小テスト', color: { light: '#a362f9', dark: '#ba93fb' } },
	other: { icon: IconOther, label: 'その他', color: { light: '#788fa7', dark: '#9badc1' } },
}

export function evalKind(type: string): KindStyle {
	return EVAL_KIND[type] ?? EVAL_KIND.other
}

export interface ModeStyle {
	icon: Component
	label: string
}

export const DELIVERY_MODE: Record<string, ModeStyle> = {
	onsite: { icon: IconOnsite, label: '対面' },
	online: { icon: IconOnline, label: 'オンライン' },
	ondemand: { icon: IconOndemand, label: 'オンデマンド' },
	hybrid: { icon: IconHybrid, label: 'ハイブリッド' },
}

export function deliveryMode(mode: string | undefined): ModeStyle | null {
	if (!mode) return null
	return DELIVERY_MODE[mode] ?? null
}

/// A leading icon per detail field / section group, keyed by FIELD_SPEC key (or
/// group label). Purely decorative wayfinding — the label carries the meaning.
export const FIELD_ICONS: Record<string, Component> = {
	eval: IconEval,
	summary: IconSummary,
	aims: IconAims,
	goals: IconGoals,
	plan: IconPlan,
	textbooks: IconTextbooks,
	prereq: IconPrereq,
	prep: IconPrep,
	officeHour: IconOfficeHour,
	teachers: IconTeachers,
	keywords: IconKeywords,
	numbering: IconNumbering,
	// Group headers + the base (科目情報) block.
	授業内容: IconTextbooks,
	その他: IconGroupOther,
	base: IconBase,
}
