// Display mapping for the stable enum strings the Rust pipeline assigns
// (`EvalRow.type`, `Delivery.mode`). Emoji/color are a pure presentation concern
// — the *ranking* of fields lives in the generated `syllabus-fields.generated.ts`.
// Colours share the course palette's OKLCH hues (one visual family) and carry a
// light/dark pair so the eval-chart arcs stay legible on either modal surface.

export interface KindColor {
	light: string
	dark: string
}

export interface KindStyle {
	emoji: string
	label: string
	color: KindColor
}

export const EVAL_KIND: Record<string, KindStyle> = {
	exam: { emoji: '✍️', label: '試験', color: { light: '#fa285c', dark: '#fb7e8c' } },
	report: { emoji: '📝', label: 'レポート', color: { light: '#1a8fef', dark: '#65b0fb' } },
	attendance: { emoji: '🙋', label: '出席・参加', color: { light: '#1da751', dark: '#26cb64' } },
	presentation: { emoji: '💬', label: '発表', color: { light: '#be7c18', dark: '#e79720' } },
	quiz: { emoji: '🧩', label: '小テスト', color: { light: '#a362f9', dark: '#ba93fb' } },
	other: { emoji: '📌', label: 'その他', color: { light: '#788fa7', dark: '#9badc1' } },
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
