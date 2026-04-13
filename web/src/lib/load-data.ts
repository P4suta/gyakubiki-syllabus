import type { ProcessedData } from '../types/course'
import { validateProcessedData } from './validate'

export async function loadData(): Promise<ProcessedData> {
	const url = `${import.meta.env.BASE_URL}data.json`
	const res = await fetch(url)
	if (!res.ok) {
		throw new Error(`データの取得に失敗しました (HTTP ${res.status})`)
	}
	const parsed: unknown = await res.json()
	const result = validateProcessedData(parsed)
	if (!result.ok) {
		throw new Error(result.error)
	}
	return result.data
}
