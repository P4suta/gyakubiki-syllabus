<script lang="ts">
import type { ProcessedDataV2 } from '../types/course'
import { validateProcessedData } from '../lib/validate'

interface Props {
	onload: (data: ProcessedDataV2) => void
}

let { onload }: Props = $props()
let error = $state('')
let dragging = $state(false)
let fileName = $state('')

function processFile(file: File) {
	error = ''
	fileName = file.name

	if (!file.name.endsWith('.json')) {
		error = `"${file.name}" はJSONファイルではありません。\n.json拡張子のファイルを選択してください。`
		return
	}

	if (file.size > 100 * 1024 * 1024) {
		error = `ファイルが大きすぎます (${Math.round(file.size / 1024 / 1024)}MB)。\n100MB以下のファイルを使用してください。`
		return
	}

	console.log('[FileLoader] processFile called:', file.name, file.size, 'bytes')
	const reader = new FileReader()
	reader.onerror = () => {
		console.error('[FileLoader] reader.onerror')
		error = `"${file.name}" の読み込みに失敗しました。\nファイルが破損しているか、アクセス権限がない可能性があります。`
	}
	reader.onload = (e) => {
		const text = e.target?.result as string
		console.log('[FileLoader] reader.onload, text length:', text?.length)
		if (!text || text.trim().length === 0) {
			error = 'ファイルの中身が空です。'
			return
		}

		let parsed: unknown
		try {
			parsed = JSON.parse(text)
			console.log('[FileLoader] JSON parsed successfully')
		} catch (parseError) {
			console.error('[FileLoader] JSON parse error:', parseError)
			const msg = parseError instanceof Error ? parseError.message : ''
			error = `JSONの解析に失敗しました。\nファイルの内容がJSON形式ではありません。${msg ? `\n詳細: ${msg}` : ''}`
			return
		}

		const result = validateProcessedData(parsed)
		console.log('[FileLoader] validation result:', result.ok, result.ok ? '' : result.error)
		if (result.ok) {
			onload(result.data)
		} else {
			error = result.error
		}
	}
	reader.readAsText(file)
}

function handleDrop(e: DragEvent) {
	e.preventDefault()
	dragging = false
	const file = e.dataTransfer?.files[0]
	if (file) processFile(file)
}

function handleFileSelect(e: Event) {
	const input = e.target as HTMLInputElement
	const file = input.files?.[0]
	if (file) processFile(file)
}
</script>

<div class="min-h-screen bg-surface-page font-sans flex items-center justify-center p-6">
	<div class="max-w-xl w-full">
		<h1 class="text-xl font-bold text-apple-text mb-2 tracking-tight">
			逆引きシラバス
		</h1>
		<p class="text-body text-apple-text/50 mb-8 leading-relaxed tracking-tight">
			高知大学 時間割ビューアー。変換済みのdata.jsonを読み込むと、曜日x時限の一覧で確認できます。
		</p>

		<div class="bg-surface-primary rounded-xl p-7 shadow-card mb-6">
			<h2 class="text-caption font-semibold text-apple-text/70 mb-4">使い方</h2>
			<div class="text-caption text-apple-text/60 leading-7 space-y-1">
				<p>1. ブラウザのDevToolsでKULASのAPIレスポンスJSONを保存</p>
				<p>2. <code class="bg-overlay-subtle px-1.5 py-0.5 rounded text-caption">syllabus-cli convert raw.json -o data.json</code> で変換</p>
				<p>3. 下のエリアにdata.jsonをドラッグ&ドロップ</p>
			</div>
		</div>

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="border-2 border-dashed rounded-xl p-12 text-center transition-colors cursor-pointer
				{dragging ? 'border-apple-blue bg-apple-blue/10' : 'border-overlay-light hover:border-overlay-strong'}"
			ondragover={(e) => { e.preventDefault(); dragging = true }}
			ondragleave={() => { dragging = false }}
			ondrop={handleDrop}
			onclick={() => document.getElementById('file-input')?.click()}
			onkeydown={(e) => { if (e.key === 'Enter') document.getElementById('file-input')?.click() }}
			role="button"
			tabindex="0"
		>
			<div class="text-apple-text/40 text-4xl mb-3">
				{dragging ? '&#x1F4E5;' : '&#x1F4C1;'}
			</div>
			<p class="text-body text-apple-text/60 font-medium">
				data.jsonをここにドロップ
			</p>
			<p class="text-caption text-apple-text/40 mt-1">
				またはクリックしてファイルを選択
			</p>
			<input
				id="file-input"
				type="file"
				accept=".json"
				class="hidden"
				onchange={handleFileSelect}
			/>
		</div>

		{#if error}
			<div class="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg">
				<p class="text-red-700 text-caption font-medium mb-1">読み込みエラー</p>
				<p class="text-red-600 text-caption whitespace-pre-line">{error}</p>
				{#if fileName}
					<p class="text-red-400 text-micro mt-2">ファイル: {fileName}</p>
				{/if}
			</div>
		{/if}
	</div>
</div>
