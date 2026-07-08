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
	exam: { emoji: '✍️', label: '試験', color: { light: '#c65954', dark: '#eb827b' } },
	report: { emoji: '📝', label: 'レポート', color: { light: '#4780d2', dark: '#70a6f5' } },
	attendance: { emoji: '🙋', label: '出席・参加', color: { light: '#319751', dark: '#62bb78' } },
	presentation: { emoji: '💬', label: '発表', color: { light: '#be6517', dark: '#e28d4f' } },
	quiz: { emoji: '🧩', label: '小テスト', color: { light: '#7d70ce', dark: '#a196f1' } },
	other: { emoji: '📌', label: 'その他', color: { light: '#79818d', dark: '#9da5b1' } },
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
