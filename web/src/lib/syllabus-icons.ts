// Display mapping for the stable enum strings the Rust pipeline assigns
// (`EvalRow.type`, `Delivery.mode`). Emoji/color are a pure presentation concern
// — the *ranking* of fields lives in the generated `syllabus-fields.generated.ts`.
// Colors are mid-tones chosen to stay legible on both light and dark backgrounds.

export interface KindStyle {
	emoji: string
	label: string
	color: string
}

export const EVAL_KIND: Record<string, KindStyle> = {
	exam: { emoji: '✍️', label: '試験', color: '#FF6B6B' },
	report: { emoji: '📝', label: 'レポート', color: '#4D96FF' },
	attendance: { emoji: '🙋', label: '出席・参加', color: '#6BCB77' },
	presentation: { emoji: '💬', label: '発表', color: '#FFB84C' },
	quiz: { emoji: '🧩', label: '小テスト', color: '#A66CFF' },
	other: { emoji: '📌', label: 'その他', color: '#9AA0A6' },
}

export function evalKind(type: string): KindStyle {
	return EVAL_KIND[type] ?? EVAL_KIND.other
}

export interface ModeStyle {
	emoji: string
	label: string
}

export const DELIVERY_MODE: Record<string, ModeStyle> = {
	onsite: { emoji: '🏫', label: '対面' },
	online: { emoji: '💻', label: 'オンライン' },
	ondemand: { emoji: '📼', label: 'オンデマンド' },
	hybrid: { emoji: '🔀', label: 'ハイブリッド' },
}

export function deliveryMode(mode: string | undefined): ModeStyle | null {
	if (!mode) return null
	return DELIVERY_MODE[mode] ?? null
}
